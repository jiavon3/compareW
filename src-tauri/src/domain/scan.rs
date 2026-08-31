use std::fs::{self, File, Metadata};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use sha2::{Digest, Sha256};
use zip::DateTime;
use zip::ZipArchive;

use super::folder::{is_archive_name, FolderKind, FolderNode};

pub fn hash_reader(mut reader: impl Read) -> Result<([u8; 32], u64), String> {
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65_536];
    let mut total = 0u64;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|_| "无法打开文件".to_string())?;
        if n == 0 {
            break;
        }
        total += n as u64;
        hasher.update(&buf[..n]);
    }
    let digest: [u8; 32] = hasher.finalize().into();
    Ok((digest, total))
}

pub fn is_zip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_archive_name)
}

pub fn open_root(path: &Path) -> Result<FolderNode, String> {
    let meta = fs::metadata(path).map_err(|_| {
        if path.is_dir() {
            "无法打开文件夹".to_string()
        } else {
            "无法打开文件".to_string()
        }
    })?;
    if meta.is_dir() {
        if meta.file_type().is_symlink() {
            return Err("无法打开文件夹".to_string());
        }
        return scan_dir(path);
    }
    if is_zip_path(path) {
        return scan_zip_path(path);
    }
    Err("无法作为压缩包打开".to_string())
}

pub fn scan_dir(root: &Path) -> Result<FolderNode, String> {
    let mut tree = FolderNode::dir();
    scan_dir_into(root, &mut tree)?;
    Ok(tree)
}

fn scan_dir_into(dir: &Path, tree: &mut FolderNode) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|_| "无法打开文件夹".to_string())?;
    for entry in entries {
        let entry = entry.map_err(|_| "无法打开文件夹".to_string())?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            if meta.is_dir() {
                continue;
            }
        }
        if meta.is_dir() {
            if meta.file_type().is_symlink() {
                continue;
            }
            tree.ensure_dir(&name);
            let child = tree.children.get_mut(&name).expect("dir just inserted");
            child.mtime = mtime_from_meta(&meta);
            let mut nested = FolderNode::dir();
            scan_dir_into(&path, &mut nested)?;
            child.children = nested.children;
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        let file = File::open(&path).map_err(|_| "无法打开文件".to_string())?;
        let (hash, size) = hash_reader(file)?;
        let kind = if is_archive_name(&name) {
            FolderKind::Archive
        } else {
            FolderKind::File
        };
        tree.children.insert(
            name,
            FolderNode::file(kind, hash, size).with_mtime(mtime_from_meta(&meta)),
        );
    }
    Ok(())
}

pub fn scan_zip_path(path: &Path) -> Result<FolderNode, String> {
    let file = File::open(path).map_err(|_| "无法打开文件".to_string())?;
    let archive = ZipArchive::new(file).map_err(|_| "无法作为压缩包打开".to_string())?;
    scan_zip_archive(archive)
}

pub fn scan_zip_bytes(bytes: &[u8]) -> Result<FolderNode, String> {
    let archive = ZipArchive::new(Cursor::new(bytes.to_vec()))
        .map_err(|_| "无法作为压缩包打开".to_string())?;
    scan_zip_archive(archive)
}

fn scan_zip_archive<R: Read + std::io::Seek>(
    mut archive: ZipArchive<R>,
) -> Result<FolderNode, String> {
    let mut tree = FolderNode::dir();
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| "无法作为压缩包打开".to_string())?;
        let raw_name = file.name().replace('\\', "/");
        if raw_name.is_empty() {
            continue;
        }
        if file.is_dir() || raw_name.ends_with('/') {
            tree.ensure_dir(raw_name.trim_end_matches('/'));
            continue;
        }
        let mtime = zip_mtime(file.last_modified());
        let (hash, size) = hash_reader(&mut file)?;
        let kind = if is_archive_name(&raw_name) {
            FolderKind::Archive
        } else {
            FolderKind::File
        };
        tree.insert_entry(&raw_name, kind, hash, size, mtime);
    }
    Ok(tree)
}

pub fn read_zip_entry_bytes(path: &Path, entry: &str) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|_| "无法打开文件".to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|_| "无法作为压缩包打开".to_string())?;
    read_entry_from_archive(&mut archive, entry)
}

pub fn read_zip_bytes_entry(bytes: &[u8], entry: &str) -> Result<Vec<u8>, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes.to_vec()))
        .map_err(|_| "无法作为压缩包打开".to_string())?;
    read_entry_from_archive(&mut archive, entry)
}

fn read_entry_from_archive<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    entry: &str,
) -> Result<Vec<u8>, String> {
    let mut file = archive
        .by_name(entry)
        .map_err(|_| "无法打开文件".to_string())?;
    let mut out = Vec::new();
    file.read_to_end(&mut out)
        .map_err(|_| "无法打开文件".to_string())?;
    Ok(out)
}

pub fn read_fs_file(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|_| "无法打开文件".to_string())
}

pub fn bytes_as_utf8(bytes: &[u8]) -> Result<String, String> {
    if bytes.contains(&0) {
        return Err("无法读取文件：不是有效的 UTF-8".to_string());
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| "无法读取文件：不是有效的 UTF-8".to_string())?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    Ok(text.to_string())
}

fn mtime_from_meta(meta: &Metadata) -> Option<i64> {
    meta.modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_secs() as i64)
}

fn zip_mtime(value: Option<DateTime>) -> Option<i64> {
    let stamp = value?;
    civil_to_unix(
        stamp.year(),
        stamp.month(),
        stamp.day(),
        stamp.hour(),
        stamp.minute(),
        stamp.second(),
    )
}

fn civil_to_unix(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Option<i64> {
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let days = days_from_civil(i32::from(year), u32::from(month), u32::from(day));
    Some(days * 86_400 + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second))
}

fn days_from_civil(mut year: i32, month: u32, day: u32) -> i64 {
    year -= i32::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = (year - era * 400) as u32;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i64::from(era) * 146_097 + i64::from(doe) - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::folder::{align_children, FolderStatus};
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::DateTime;
    use zip::ZipWriter;

    fn temp_dir(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("comparew-scan-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
        write_zip_with_time(path, files, None);
    }

    fn write_zip_with_time(path: &Path, files: &[(&str, &[u8])], time: Option<DateTime>) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        for (name, data) in files {
            let mut options = SimpleFileOptions::default();
            if let Some(time) = time {
                options = options.last_modified_time(time);
            }
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn identical_directories_are_equal() {
        let left = temp_dir("id-l");
        let right = temp_dir("id-r");
        fs::write(left.join("a.txt"), "hello").unwrap();
        fs::write(right.join("a.txt"), "hello").unwrap();
        let l = scan_dir(&left).unwrap();
        let r = scan_dir(&right).unwrap();
        let rows = align_children(&l.children, &r.children);
        assert_eq!(rows[0].status, FolderStatus::Equal);
        assert!(rows[0].left_mtime.is_some());
        assert!(rows[0].right_mtime.is_some());
        let _ = fs::remove_dir_all(left);
        let _ = fs::remove_dir_all(right);
    }

    #[test]
    fn zip_timestamp_change_keeps_equal_content() {
        let dir = temp_dir("zip-ts");
        let a = dir.join("a.zip");
        let b = dir.join("b.zip");
        let t1 = DateTime::from_date_and_time(2020, 1, 1, 0, 0, 0).unwrap();
        let t2 = DateTime::from_date_and_time(2021, 6, 6, 12, 0, 0).unwrap();
        write_zip_with_time(
            &a,
            &[("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\n")],
            Some(t1),
        );
        write_zip_with_time(
            &b,
            &[("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\n")],
            Some(t2),
        );
        let l = scan_zip_path(&a).unwrap();
        let r = scan_zip_path(&b).unwrap();
        let rows = align_children(&l.children, &r.children);
        assert_eq!(rows[0].name, "META-INF");
        assert_eq!(rows[0].status, FolderStatus::Equal);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn nested_jar_same_bytes_equal_without_enter() {
        let dir = temp_dir("nested-eq");
        let inner = dir.join("inner.jar");
        write_zip(&inner, &[("com/A.class", b"class-bytes")]);
        let inner_bytes = fs::read(&inner).unwrap();
        let left = dir.join("left.jar");
        let right = dir.join("right.jar");
        write_zip(&left, &[("BOOT-INF/lib/foo.jar", &inner_bytes)]);
        write_zip(&right, &[("BOOT-INF/lib/foo.jar", &inner_bytes)]);
        let l = scan_zip_path(&left).unwrap();
        let r = scan_zip_path(&right).unwrap();
        let top = align_children(&l.children, &r.children);
        assert_eq!(top[0].name, "BOOT-INF");
        assert_eq!(top[0].status, FolderStatus::Equal);
        let lib = align_children(
            &l.child("BOOT-INF").unwrap().child("lib").unwrap().children,
            &r.child("BOOT-INF").unwrap().child("lib").unwrap().children,
        );
        assert_eq!(lib[0].kind, crate::domain::folder::FolderKind::Archive);
        assert_eq!(lib[0].status, FolderStatus::Equal);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn nested_jar_different_bytes_lists_inner_after_scan() {
        let dir = temp_dir("nested-diff");
        let inner_l = dir.join("inner-l.jar");
        let inner_r = dir.join("inner-r.jar");
        write_zip(&inner_l, &[("com/A.class", b"old")]);
        write_zip(&inner_r, &[("com/A.class", b"new")]);
        let left = dir.join("left.jar");
        let right = dir.join("right.jar");
        write_zip(
            &left,
            &[("BOOT-INF/lib/foo.jar", &fs::read(&inner_l).unwrap())],
        );
        write_zip(
            &right,
            &[("BOOT-INF/lib/foo.jar", &fs::read(&inner_r).unwrap())],
        );
        let l = scan_zip_path(&left).unwrap();
        let r = scan_zip_path(&right).unwrap();
        let lib = align_children(
            &l.child("BOOT-INF").unwrap().child("lib").unwrap().children,
            &r.child("BOOT-INF").unwrap().child("lib").unwrap().children,
        );
        assert_eq!(lib[0].status, FolderStatus::Different);
        let left_bytes = read_zip_entry_bytes(&left, "BOOT-INF/lib/foo.jar").unwrap();
        let right_bytes = read_zip_entry_bytes(&right, "BOOT-INF/lib/foo.jar").unwrap();
        let inner_l_tree = scan_zip_bytes(&left_bytes).unwrap();
        let inner_r_tree = scan_zip_bytes(&right_bytes).unwrap();
        let inner_rows = align_children(&inner_l_tree.children, &inner_r_tree.children);
        assert_eq!(inner_rows[0].name, "com");
        assert_eq!(inner_rows[0].status, FolderStatus::Different);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_zip_fails_to_open() {
        let dir = temp_dir("bad-zip");
        let path = dir.join("bad.jar");
        fs::write(&path, b"not a zip").unwrap();
        let err = scan_zip_path(&path).unwrap_err();
        assert_eq!(err, "无法作为压缩包打开");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn empty_zip_scans() {
        let dir = temp_dir("empty-zip");
        let path = dir.join("empty.zip");
        write_zip(&path, &[]);
        let tree = scan_zip_path(&path).unwrap();
        assert!(tree.children.is_empty());
        let _ = fs::remove_dir_all(dir);
    }
}
