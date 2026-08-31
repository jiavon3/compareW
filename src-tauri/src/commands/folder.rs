use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, State};

use crate::commands::file::{reject_if_too_large, MAX_FILE_BYTES};
use crate::domain::folder::{
    align_children, folder_summary_counts, format_path_bar, is_archive_name, is_class_name,
    is_text_name, window_folder_rows, FolderKind, FolderNode, FolderRow, FolderStatus,
};
use crate::domain::scan::{
    bytes_as_utf8, open_root, read_fs_file, read_zip_bytes_entry, read_zip_entry_bytes,
    scan_zip_bytes,
};

#[derive(Clone)]
enum Source {
    Dir(PathBuf),
    Zip(PathBuf),
    Memory(Vec<u8>),
}

struct Frame {
    left_src: Source,
    right_src: Source,
    left_tree: FolderNode,
    right_tree: FolderNode,
    prefix: Vec<String>,
    segments: Vec<(String, bool)>,
}

pub struct FolderSession {
    left_src: Source,
    right_src: Source,
    left_tree: FolderNode,
    right_tree: FolderNode,
    prefix: Vec<String>,
    segments: Vec<(String, bool)>,
    rows: Vec<FolderRow>,
    stack: Vec<Frame>,
    nested: HashMap<String, Source>,
}

impl FolderSession {
    pub fn start(left: &Path, right: &Path) -> Result<Self, String> {
        if path_empty(left) && path_empty(right) {
            return Err("无法打开文件夹".to_string());
        }
        let (left_src, left_tree) = open_side(left)?;
        let (right_src, right_tree) = open_side(right)?;
        let mut session = Self {
            left_src,
            right_src,
            left_tree,
            right_tree,
            prefix: Vec::new(),
            segments: Vec::new(),
            rows: Vec::new(),
            stack: Vec::new(),
            nested: HashMap::new(),
        };
        session.refresh_rows();
        Ok(session)
    }

    fn refresh_rows(&mut self) {
        let left = walk(&self.left_tree, &self.prefix);
        let right = walk(&self.right_tree, &self.prefix);
        self.rows = align_children(&left.children, &right.children);
    }

    pub fn summary(&self) -> FolderSummary {
        let (equal, different) = folder_summary_counts(&self.rows);
        FolderSummary {
            path_bar: format_path_bar(&self.segments),
            can_go_up: !self.prefix.is_empty() || !self.stack.is_empty(),
            row_count: self.rows.len() as u32,
            equal,
            different,
        }
    }

    pub fn window(&self, filter: &str, offset: u32, limit: u32) -> FolderWindow {
        let limit = limit.clamp(1, 300) as usize;
        let offset = offset as usize;
        let (rows, total) = window_folder_rows(&self.rows, filter, offset, limit);
        FolderWindow {
            rows,
            total: total as u32,
            offset: offset as u32,
        }
    }

    pub fn enter(&mut self, name: &str) -> Result<(), String> {
        let row = self
            .rows
            .iter()
            .find(|row| row.name == name)
            .ok_or_else(|| "无法打开文件夹".to_string())?;
        if row.status == FolderStatus::TypeConflict {
            return Err("无法打开文件夹".to_string());
        }
        match row.kind {
            FolderKind::Dir => {
                self.prefix.push(name.to_string());
                self.segments.push((name.to_string(), false));
                self.refresh_rows();
                Ok(())
            }
            FolderKind::Archive => {
                let left_bytes = self.read_side("left", name).ok();
                let right_bytes = self.read_side("right", name).ok();
                self.stack.push(Frame {
                    left_src: self.left_src.clone(),
                    right_src: self.right_src.clone(),
                    left_tree: self.left_tree.clone(),
                    right_tree: self.right_tree.clone(),
                    prefix: self.prefix.clone(),
                    segments: self.segments.clone(),
                });
                self.left_tree = match &left_bytes {
                    Some(bytes) => scan_zip_bytes(bytes)?,
                    None => FolderNode::dir(),
                };
                self.right_tree = match &right_bytes {
                    Some(bytes) => scan_zip_bytes(bytes)?,
                    None => FolderNode::dir(),
                };
                self.left_src = left_bytes
                    .map(Source::Memory)
                    .unwrap_or(Source::Memory(Vec::new()));
                self.right_src = right_bytes
                    .map(Source::Memory)
                    .unwrap_or(Source::Memory(Vec::new()));
                self.prefix.clear();
                self.segments.push((name.to_string(), true));
                self.refresh_rows();
                Ok(())
            }
            FolderKind::File => Err("无法打开文件夹".to_string()),
        }
    }

    pub fn up(&mut self) -> Result<(), String> {
        if !self.prefix.is_empty() {
            self.prefix.pop();
            self.segments.pop();
            self.refresh_rows();
            return Ok(());
        }
        let frame = self
            .stack
            .pop()
            .ok_or_else(|| "无法打开文件夹".to_string())?;
        self.left_src = frame.left_src;
        self.right_src = frame.right_src;
        self.left_tree = frame.left_tree;
        self.right_tree = frame.right_tree;
        self.prefix = frame.prefix;
        self.segments = frame.segments;
        self.refresh_rows();
        Ok(())
    }

    pub fn children_at(&mut self, path: &[String]) -> Result<Vec<FolderRow>, String> {
        for i in 1..=path.len() {
            self.ensure_archive(&path[..i])?;
        }
        let empty = std::collections::BTreeMap::new();
        let left = node_at(&self.left_tree, path)
            .map(|node| node.children.clone())
            .unwrap_or(empty.clone());
        let right = node_at(&self.right_tree, path)
            .map(|node| node.children.clone())
            .unwrap_or(empty);
        Ok(align_children(&left, &right))
    }

    fn ensure_archive(&mut self, path: &[String]) -> Result<(), String> {
        if path.is_empty() {
            return Ok(());
        }
        let archive = node_at(&self.left_tree, path)
            .or_else(|| node_at(&self.right_tree, path))
            .is_some_and(|node| node.kind == FolderKind::Archive);
        if !archive {
            return Ok(());
        }
        let key_path = path.join("/");
        let need_left = node_at(&self.left_tree, path)
            .is_some_and(|node| node.kind == FolderKind::Archive && node.children.is_empty());
        if need_left {
            if let Ok(bytes) = self.read_path("left", path) {
                let inner = scan_zip_bytes(&bytes)?;
                if let Some(node) = node_at_mut(&mut self.left_tree, path) {
                    node.children = inner.children;
                }
                self.nested
                    .insert(format!("left:{key_path}"), Source::Memory(bytes));
            }
        }
        let need_right = node_at(&self.right_tree, path)
            .is_some_and(|node| node.kind == FolderKind::Archive && node.children.is_empty());
        if need_right {
            if let Ok(bytes) = self.read_path("right", path) {
                let inner = scan_zip_bytes(&bytes)?;
                if let Some(node) = node_at_mut(&mut self.right_tree, path) {
                    node.children = inner.children;
                }
                self.nested
                    .insert(format!("right:{key_path}"), Source::Memory(bytes));
            }
        }
        Ok(())
    }

    fn read_from_root(&self, side: &str, path: &[String]) -> Result<Vec<u8>, String> {
        let src = match side {
            "right" => &self.right_src,
            _ => &self.left_src,
        };
        if path.is_empty() {
            return Err("无法打开文件".to_string());
        }
        read_from_source(src, &path[..path.len() - 1], path.last().unwrap())
    }

    pub fn read_path(&self, side: &str, path: &[String]) -> Result<Vec<u8>, String> {
        if path.is_empty() {
            return Err("无法打开文件".to_string());
        }
        for end in (1..path.len()).rev() {
            let prefix = &path[..end];
            let key = format!("{side}:{}", prefix.join("/"));
            if let Some(src) = self.nested.get(&key) {
                let rest = &path[end..];
                return read_from_source(src, &rest[..rest.len() - 1], rest.last().unwrap());
            }
        }
        self.read_from_root(side, path)
    }

    pub fn read_side(&self, side: &str, name: &str) -> Result<Vec<u8>, String> {
        self.read_path(side, &[name.to_string()])
    }

    pub fn row(&self, name: &str) -> Option<&FolderRow> {
        self.rows.iter().find(|row| row.name == name)
    }
}

fn path_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}

fn open_side(path: &Path) -> Result<(Source, FolderNode), String> {
    if path_empty(path) {
        return Ok((Source::Memory(Vec::new()), FolderNode::dir()));
    }
    Ok((source_for(path)?, open_root(path)?))
}

fn source_for(path: &Path) -> Result<Source, String> {
    let meta = std::fs::metadata(path).map_err(|_| {
        if path.is_dir() {
            "无法打开文件夹".to_string()
        } else {
            "无法打开文件".to_string()
        }
    })?;
    if meta.is_dir() {
        Ok(Source::Dir(path.to_path_buf()))
    } else if crate::domain::scan::is_zip_path(path) {
        Ok(Source::Zip(path.to_path_buf()))
    } else {
        Err("无法作为压缩包打开".to_string())
    }
}

fn node_at<'a>(root: &'a FolderNode, path: &[String]) -> Option<&'a FolderNode> {
    let mut node = root;
    for name in path {
        node = node.child(name)?;
    }
    Some(node)
}

fn node_at_mut<'a>(root: &'a mut FolderNode, path: &[String]) -> Option<&'a mut FolderNode> {
    let mut node = root;
    for name in path {
        node = node.children.get_mut(name)?;
    }
    Some(node)
}

fn walk(root: &FolderNode, prefix: &[String]) -> FolderNode {
    node_at(root, prefix)
        .cloned()
        .unwrap_or_else(FolderNode::dir)
}

fn read_from_source(src: &Source, prefix: &[String], name: &str) -> Result<Vec<u8>, String> {
    match src {
        Source::Dir(root) => {
            let mut path = root.clone();
            for part in prefix {
                path.push(part);
            }
            path.push(name);
            read_fs_file(&path)
        }
        Source::Zip(path) => {
            let rel = zip_rel(prefix, name);
            read_zip_entry_bytes(path, &rel)
        }
        Source::Memory(bytes) => {
            if bytes.is_empty() {
                return Err("无法打开文件".to_string());
            }
            let rel = zip_rel(prefix, name);
            read_zip_bytes_entry(bytes, &rel)
        }
    }
}

fn zip_rel(prefix: &[String], name: &str) -> String {
    let mut parts = prefix.to_vec();
    parts.push(name.to_string());
    parts.join("/")
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderSummary {
    pub path_bar: String,
    pub can_go_up: bool,
    pub row_count: u32,
    pub equal: u32,
    pub different: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderWindow {
    pub rows: Vec<FolderRow>,
    pub total: u32,
    pub offset: u32,
}

#[derive(Default)]
pub struct FolderStore {
    session: Option<FolderSession>,
    generation: u64,
}

#[tauri::command]
pub async fn start_folder_compare(
    left: String,
    right: String,
    store: State<'_, Mutex<FolderStore>>,
) -> Result<FolderSummary, String> {
    let session = tauri::async_runtime::spawn_blocking(move || {
        FolderSession::start(Path::new(&left), Path::new(&right))
    })
    .await
    .map_err(|_| "无法打开文件夹".to_string())??;
    let summary = session.summary();
    let mut guard = store.lock().expect("folder store");
    guard.generation += 1;
    guard.session = Some(session);
    Ok(summary)
}

#[tauri::command]
pub fn list_folder_children(
    path: Vec<String>,
    store: State<Mutex<FolderStore>>,
) -> Result<Vec<FolderRow>, String> {
    let mut guard = store.lock().expect("folder store");
    let session = guard
        .session
        .as_mut()
        .ok_or_else(|| "无法打开文件夹".to_string())?;
    session.children_at(&path)
}

#[tauri::command]
pub fn list_folder_rows(
    filter: String,
    offset: u32,
    limit: u32,
    store: State<Mutex<FolderStore>>,
) -> Result<FolderWindow, String> {
    let guard = store.lock().expect("folder store");
    let session = guard
        .session
        .as_ref()
        .ok_or_else(|| "无法打开文件夹".to_string())?;
    Ok(session.window(&filter, offset, limit))
}

#[tauri::command]
pub fn folder_enter(
    name: String,
    store: State<Mutex<FolderStore>>,
) -> Result<FolderSummary, String> {
    let mut guard = store.lock().expect("folder store");
    let session = guard
        .session
        .as_mut()
        .ok_or_else(|| "无法打开文件夹".to_string())?;
    session.enter(&name)?;
    Ok(session.summary())
}

#[tauri::command]
pub fn folder_up(store: State<Mutex<FolderStore>>) -> Result<FolderSummary, String> {
    let mut guard = store.lock().expect("folder store");
    let session = guard
        .session
        .as_mut()
        .ok_or_else(|| "无法打开文件夹".to_string())?;
    session.up()?;
    Ok(session.summary())
}

#[tauri::command]
pub fn cancel_folder_compare(store: State<Mutex<FolderStore>>) {
    let mut guard = store.lock().expect("folder store");
    guard.generation += 1;
    guard.session = None;
}

#[tauri::command]
pub fn read_folder_entry(
    side: String,
    path: Vec<String>,
    store: State<Mutex<FolderStore>>,
) -> Result<String, String> {
    let guard = store.lock().expect("folder store");
    let session = guard
        .session
        .as_ref()
        .ok_or_else(|| "无法打开文件".to_string())?;
    let bytes = session.read_path(&side, &path)?;
    reject_if_too_large(bytes.len() as u64)?;
    bytes_as_utf8(&bytes)
}

#[tauri::command]
pub fn java_available() -> bool {
    Command::new("java")
        .arg("-version")
        .output()
        .map(|out| out.status.success() || !out.stderr.is_empty() || !out.stdout.is_empty())
        .unwrap_or(false)
}

#[tauri::command]
pub fn decompile_class(
    side: String,
    path: Vec<String>,
    app: tauri::AppHandle,
    store: State<Mutex<FolderStore>>,
) -> Result<String, String> {
    if !java_available() {
        return Err("反编译失败".to_string());
    }
    if !is_class_name(path.last().map(String::as_str).unwrap_or("")) {
        return Err("反编译失败".to_string());
    }
    let bytes = {
        let guard = store.lock().expect("folder store");
        let session = guard
            .session
            .as_ref()
            .ok_or_else(|| "反编译失败".to_string())?;
        session.read_path(&side, &path)?
    };
    reject_if_too_large(bytes.len() as u64)?;
    decompile_bytes(&bytes, &app)
}

fn cfr_jar(app: &tauri::AppHandle) -> Option<PathBuf> {
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/cfr.jar");
    if dev.is_file() {
        return Some(dev);
    }
    app.path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("resources/cfr.jar"))
        .filter(|path| path.is_file())
}

fn decompile_bytes(bytes: &[u8], app: &tauri::AppHandle) -> Result<String, String> {
    let dir = std::env::temp_dir().join(format!("comparew-cfr-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let class_path = dir.join("T.class");
    std::fs::write(&class_path, bytes).map_err(|_| "反编译失败".to_string())?;
    let class_s = class_path.to_string_lossy().into_owned();
    let output = if let Some(cfr) = cfr_jar(app) {
        let cfr_s = cfr.to_string_lossy().into_owned();
        Command::new("java")
            .args(["-jar", &cfr_s, &class_s])
            .output()
    } else {
        Command::new("javap")
            .args(["-c", "-p", "-v", &class_s])
            .output()
    };
    let _ = std::fs::remove_file(&class_path);
    let output = output.map_err(|_| "反编译失败".to_string())?;
    if !output.status.success() && output.stdout.is_empty() {
        return Err("反编译失败".to_string());
    }
    String::from_utf8(output.stdout).map_err(|_| "反编译失败".to_string())
}

pub fn entry_is_text(name: &str, bytes: &[u8]) -> bool {
    if is_class_name(name) {
        return false;
    }
    if is_archive_name(name) {
        return false;
    }
    if is_text_name(name) {
        return bytes_as_utf8(bytes).is_ok();
    }
    !bytes.contains(&0) && bytes_as_utf8(bytes).is_ok() && bytes.len() as u64 <= MAX_FILE_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::folder::FolderStatus;
    use std::fs;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn temp_dir(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("comparew-sess-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        for (name, data) in files {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn session_lists_and_enters_nested_jar() {
        let dir = temp_dir("nav");
        let inner_l = dir.join("il.jar");
        let inner_r = dir.join("ir.jar");
        write_zip(&inner_l, &[("com/Hello.class", b"old")]);
        write_zip(&inner_r, &[("com/Hello.class", b"new")]);
        let left = dir.join("prod.jar");
        let right = dir.join("next.jar");
        write_zip(
            &left,
            &[
                ("BOOT-INF/lib/foo.jar", &fs::read(&inner_l).unwrap()),
                ("README.txt", b"prod"),
            ],
        );
        write_zip(
            &right,
            &[
                ("BOOT-INF/lib/foo.jar", &fs::read(&inner_r).unwrap()),
                ("README.txt", b"prod"),
            ],
        );
        let mut session = FolderSession::start(&left, &right).unwrap();
        let roots = session.children_at(&[]).unwrap();
        assert!(roots.iter().any(|row| row.name == "BOOT-INF"));
        assert!(roots
            .iter()
            .any(|row| { row.name == "README.txt" && row.status == FolderStatus::Equal }));
        let lib = session
            .children_at(&["BOOT-INF".into(), "lib".into()])
            .unwrap();
        let foo = lib.iter().find(|row| row.name == "foo.jar").unwrap();
        assert_eq!(foo.status, FolderStatus::Different);
        let inner = session
            .children_at(&["BOOT-INF".into(), "lib".into(), "foo.jar".into()])
            .unwrap();
        assert!(inner
            .iter()
            .any(|row| row.name == "com" && row.status == FolderStatus::Different));
        let class_rows = session
            .children_at(&[
                "BOOT-INF".into(),
                "lib".into(),
                "foo.jar".into(),
                "com".into(),
            ])
            .unwrap();
        assert_eq!(class_rows[0].name, "Hello.class");
        assert_eq!(class_rows[0].status, FolderStatus::Different);
        let left_bytes = session
            .read_path(
                "left",
                &[
                    "BOOT-INF".into(),
                    "lib".into(),
                    "foo.jar".into(),
                    "com".into(),
                    "Hello.class".into(),
                ],
            )
            .unwrap();
        assert_eq!(left_bytes, b"old");
        let still = session
            .children_at(&["BOOT-INF".into(), "lib".into()])
            .unwrap();
        assert_eq!(
            still.iter().find(|row| row.name == "foo.jar").unwrap().kind,
            FolderKind::Archive
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn session_lists_one_side_without_the_other() {
        let left = temp_dir("one-l");
        fs::write(left.join("only.txt"), "hello").unwrap();
        fs::create_dir(left.join("sub")).unwrap();
        fs::write(left.join("sub").join("a.md"), "a").unwrap();
        let mut session = FolderSession::start(&left, Path::new("")).unwrap();
        let roots = session.children_at(&[]).unwrap();
        assert!(roots
            .iter()
            .any(|row| { row.name == "only.txt" && row.status == FolderStatus::LeftOnly }));
        let sub = session.children_at(&["sub".into()]).unwrap();
        assert_eq!(sub[0].name, "a.md");
        assert_eq!(sub[0].status, FolderStatus::LeftOnly);
        let _ = fs::remove_dir_all(left);
    }

    #[test]
    fn text_entry_reads_utf8() {
        let dir = temp_dir("txt");
        fs::write(dir.join("a.xml"), "<ok/>").unwrap();
        let right = temp_dir("txt-r");
        fs::write(right.join("a.xml"), "<ok/>").unwrap();
        let session = FolderSession::start(&dir, &right).unwrap();
        let bytes = session.read_side("left", "a.xml").unwrap();
        assert!(entry_is_text("a.xml", &bytes));
        assert_eq!(bytes_as_utf8(&bytes).unwrap(), "<ok/>");
        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_dir_all(right);
    }
}
