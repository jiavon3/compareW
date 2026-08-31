use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FolderKind {
    Dir,
    Archive,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FolderStatus {
    Equal,
    Different,
    LeftOnly,
    RightOnly,
    TypeConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRow {
    pub name: String,
    pub kind: FolderKind,
    pub status: FolderStatus,
    pub left_size: Option<u64>,
    pub right_size: Option<u64>,
    pub left_mtime: Option<i64>,
    pub right_mtime: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct FolderNode {
    pub kind: FolderKind,
    pub hash: Option<[u8; 32]>,
    pub size: Option<u64>,
    pub mtime: Option<i64>,
    pub children: BTreeMap<String, FolderNode>,
}

impl FolderNode {
    pub fn dir() -> Self {
        Self {
            kind: FolderKind::Dir,
            hash: None,
            size: None,
            mtime: None,
            children: BTreeMap::new(),
        }
    }

    pub fn file(kind: FolderKind, hash: [u8; 32], size: u64) -> Self {
        Self {
            kind,
            hash: Some(hash),
            size: Some(size),
            mtime: None,
            children: BTreeMap::new(),
        }
    }

    pub fn with_mtime(mut self, mtime: Option<i64>) -> Self {
        self.mtime = mtime;
        self
    }

    pub fn insert_file(&mut self, rel: &str, kind: FolderKind, hash: [u8; 32], size: u64) {
        self.insert_entry(rel, kind, hash, size, None);
    }

    pub fn insert_entry(
        &mut self,
        rel: &str,
        kind: FolderKind,
        hash: [u8; 32],
        size: u64,
        mtime: Option<i64>,
    ) {
        let parts: Vec<&str> = rel.split('/').filter(|part| !part.is_empty()).collect();
        if parts.is_empty() {
            return;
        }
        let mut node = self;
        for (index, part) in parts.iter().enumerate() {
            if index + 1 == parts.len() {
                node.children.insert(
                    (*part).to_string(),
                    FolderNode::file(kind, hash, size).with_mtime(mtime),
                );
            } else {
                node = node
                    .children
                    .entry((*part).to_string())
                    .or_insert_with(FolderNode::dir);
                node.kind = FolderKind::Dir;
            }
        }
    }

    pub fn ensure_dir(&mut self, rel: &str) {
        let parts: Vec<&str> = rel.split('/').filter(|part| !part.is_empty()).collect();
        let mut node = self;
        for part in parts {
            node = node
                .children
                .entry(part.to_string())
                .or_insert_with(FolderNode::dir);
            node.kind = FolderKind::Dir;
        }
    }

    pub fn child(&self, name: &str) -> Option<&FolderNode> {
        self.children.get(name)
    }
}

pub fn is_archive_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".jar")
        || lower.ends_with(".zip")
        || lower.ends_with(".war")
        || lower.ends_with(".ear")
}

pub fn is_class_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".class")
}

pub fn is_text_name(name: &str) -> bool {
    const EXTS: &[&str] = &[
        ".xml",
        ".yml",
        ".yaml",
        ".properties",
        ".html",
        ".htm",
        ".md",
        ".txt",
        ".json",
        ".js",
        ".ts",
        ".css",
        ".java",
        ".kt",
        ".sql",
        ".conf",
        ".ini",
        ".csv",
        ".gradle",
    ];
    let lower = name.to_ascii_lowercase();
    EXTS.iter().any(|ext| lower.ends_with(ext))
}

fn looks_like_dir(node: &FolderNode) -> bool {
    node.kind == FolderKind::Dir
}

fn row_kind(left: Option<&FolderNode>, right: Option<&FolderNode>) -> FolderKind {
    match (left, right) {
        (Some(node), None) | (None, Some(node)) | (Some(node), Some(_)) => {
            if let (Some(l), Some(r)) = (left, right) {
                if looks_like_dir(l) || looks_like_dir(r) {
                    if looks_like_dir(l) && looks_like_dir(r) {
                        return FolderKind::Dir;
                    }
                }
                if l.kind == FolderKind::Archive || r.kind == FolderKind::Archive {
                    return FolderKind::Archive;
                }
                if looks_like_dir(l) || looks_like_dir(r) {
                    return if looks_like_dir(l) { l.kind } else { r.kind };
                }
            }
            node.kind
        }
        (None, None) => FolderKind::File,
    }
}

fn has_conflict(left: &FolderNode, right: &FolderNode) -> bool {
    looks_like_dir(left) != looks_like_dir(right)
}

fn rollup(rows: &[FolderRow]) -> FolderStatus {
    if rows.is_empty() {
        return FolderStatus::Equal;
    }
    if rows.iter().all(|row| row.status == FolderStatus::Equal) {
        FolderStatus::Equal
    } else {
        FolderStatus::Different
    }
}

pub fn compare_nodes(left: Option<&FolderNode>, right: Option<&FolderNode>) -> FolderStatus {
    match (left, right) {
        (None, None) => FolderStatus::Equal,
        (Some(_), None) => FolderStatus::LeftOnly,
        (None, Some(_)) => FolderStatus::RightOnly,
        (Some(l), Some(r)) => {
            if has_conflict(l, r) {
                FolderStatus::TypeConflict
            } else if looks_like_dir(l) && looks_like_dir(r) {
                rollup(&align_children(&l.children, &r.children))
            } else if l.hash == r.hash {
                FolderStatus::Equal
            } else {
                FolderStatus::Different
            }
        }
    }
}

pub fn align_children(
    left: &BTreeMap<String, FolderNode>,
    right: &BTreeMap<String, FolderNode>,
) -> Vec<FolderRow> {
    let mut names: Vec<&str> = left
        .keys()
        .chain(right.keys())
        .map(String::as_str)
        .collect();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .map(|name| {
            let l = left.get(name);
            let r = right.get(name);
            FolderRow {
                name: name.to_string(),
                kind: row_kind(l, r),
                status: compare_nodes(l, r),
                left_size: l.and_then(|node| node.size),
                right_size: r.and_then(|node| node.size),
                left_mtime: l.and_then(|node| node.mtime),
                right_mtime: r.and_then(|node| node.mtime),
            }
        })
        .collect()
}

pub fn folder_row_matches(status: FolderStatus, filter: &str) -> bool {
    match filter {
        "same" => status == FolderStatus::Equal,
        "diff" => status != FolderStatus::Equal,
        _ => true,
    }
}

pub fn window_folder_rows(
    rows: &[FolderRow],
    filter: &str,
    offset: usize,
    limit: usize,
) -> (Vec<FolderRow>, usize) {
    let total = rows
        .iter()
        .filter(|row| folder_row_matches(row.status, filter))
        .count();
    let window = rows
        .iter()
        .filter(|row| folder_row_matches(row.status, filter))
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();
    (window, total)
}

pub fn folder_summary_counts(rows: &[FolderRow]) -> (u32, u32) {
    let equal = rows
        .iter()
        .filter(|row| row.status == FolderStatus::Equal)
        .count() as u32;
    let different = rows.len() as u32 - equal;
    (equal, different)
}

pub fn format_path_bar(segments: &[(String, bool)]) -> String {
    if segments.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (index, (name, archive)) in segments.iter().enumerate() {
        out.push_str(name);
        if *archive {
            out.push_str("!/");
        } else if index + 1 == segments.len() {
            out.push('/');
        } else {
            out.push('/');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; 32] {
        let mut value = [0u8; 32];
        value[0] = byte;
        value
    }

    #[test]
    fn identical_files_are_equal() {
        let mut left = FolderNode::dir();
        let mut right = FolderNode::dir();
        left.insert_file("a.txt", FolderKind::File, hash(1), 3);
        right.insert_file("a.txt", FolderKind::File, hash(1), 3);
        let rows = align_children(&left.children, &right.children);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, FolderStatus::Equal);
        assert_eq!(rows[0].kind, FolderKind::File);
    }

    #[test]
    fn left_only_and_right_only() {
        let mut left = FolderNode::dir();
        let mut right = FolderNode::dir();
        left.insert_file("old.txt", FolderKind::File, hash(1), 1);
        right.insert_file("new.txt", FolderKind::File, hash(2), 1);
        let rows = align_children(&left.children, &right.children);
        assert_eq!(rows[0].name, "new.txt");
        assert_eq!(rows[0].status, FolderStatus::RightOnly);
        assert_eq!(rows[1].name, "old.txt");
        assert_eq!(rows[1].status, FolderStatus::LeftOnly);
    }

    #[test]
    fn different_hashes_are_different() {
        let mut left = FolderNode::dir();
        let mut right = FolderNode::dir();
        left.insert_file("a.txt", FolderKind::File, hash(1), 1);
        right.insert_file("a.txt", FolderKind::File, hash(2), 2);
        let rows = align_children(&left.children, &right.children);
        assert_eq!(rows[0].status, FolderStatus::Different);
        assert_eq!(rows[0].left_size, Some(1));
        assert_eq!(rows[0].right_size, Some(2));
    }

    #[test]
    fn file_versus_directory_is_type_conflict() {
        let mut left = FolderNode::dir();
        let mut right = FolderNode::dir();
        left.insert_file("item", FolderKind::File, hash(1), 1);
        right.ensure_dir("item");
        let rows = align_children(&left.children, &right.children);
        assert_eq!(rows[0].status, FolderStatus::TypeConflict);
    }

    #[test]
    fn directory_rollup_follows_children() {
        let mut left = FolderNode::dir();
        let mut right = FolderNode::dir();
        left.insert_file("dir/keep.txt", FolderKind::File, hash(1), 1);
        left.insert_file("dir/old.txt", FolderKind::File, hash(2), 1);
        right.insert_file("dir/keep.txt", FolderKind::File, hash(1), 1);
        let rows = align_children(&left.children, &right.children);
        assert_eq!(rows[0].name, "dir");
        assert_eq!(rows[0].kind, FolderKind::Dir);
        assert_eq!(rows[0].status, FolderStatus::Different);

        let mut same_l = FolderNode::dir();
        let mut same_r = FolderNode::dir();
        same_l.insert_file("dir/a.txt", FolderKind::File, hash(9), 1);
        same_r.insert_file("dir/a.txt", FolderKind::File, hash(9), 1);
        let same = align_children(&same_l.children, &same_r.children);
        assert_eq!(same[0].status, FolderStatus::Equal);
    }

    #[test]
    fn empty_directories_are_equal() {
        let mut left = FolderNode::dir();
        let mut right = FolderNode::dir();
        left.ensure_dir("empty");
        right.ensure_dir("empty");
        let rows = align_children(&left.children, &right.children);
        assert_eq!(rows[0].status, FolderStatus::Equal);
        assert_eq!(rows[0].kind, FolderKind::Dir);
    }

    #[test]
    fn nested_archive_compares_as_file_until_entered() {
        let mut left = FolderNode::dir();
        let mut right = FolderNode::dir();
        left.insert_file("lib/foo.jar", FolderKind::Archive, hash(4), 10);
        right.insert_file("lib/foo.jar", FolderKind::Archive, hash(4), 10);
        let rows = align_children(&left.children, &right.children);
        assert_eq!(rows[0].kind, FolderKind::Dir);
        assert_eq!(rows[0].status, FolderStatus::Equal);
        let inner = align_children(
            &left.child("lib").unwrap().children,
            &right.child("lib").unwrap().children,
        );
        assert_eq!(inner[0].kind, FolderKind::Archive);
        assert_eq!(inner[0].status, FolderStatus::Equal);

        right.children.clear();
        right.insert_file("lib/foo.jar", FolderKind::Archive, hash(5), 11);
        let dirty = align_children(
            &left.child("lib").unwrap().children,
            &right.child("lib").unwrap().children,
        );
        assert_eq!(dirty[0].status, FolderStatus::Different);
    }

    #[test]
    fn diff_filter_window_skips_equal_rows() {
        let rows = vec![
            FolderRow {
                name: "a".into(),
                kind: FolderKind::File,
                status: FolderStatus::Equal,
                left_size: Some(1),
                right_size: Some(1),
                left_mtime: None,
                right_mtime: None,
            },
            FolderRow {
                name: "b".into(),
                kind: FolderKind::File,
                status: FolderStatus::Different,
                left_size: Some(1),
                right_size: Some(2),
                left_mtime: None,
                right_mtime: None,
            },
            FolderRow {
                name: "c".into(),
                kind: FolderKind::File,
                status: FolderStatus::LeftOnly,
                left_size: Some(1),
                right_size: None,
                left_mtime: None,
                right_mtime: None,
            },
        ];
        let (window, total) = window_folder_rows(&rows, "diff", 1, 1);
        assert_eq!(total, 2);
        assert_eq!(window.len(), 1);
        assert_eq!(window[0].name, "c");
    }

    #[test]
    fn path_bar_joins_archives_with_bang() {
        let bar = format_path_bar(&[
            ("BOOT-INF".into(), false),
            ("lib".into(), false),
            ("foo.jar".into(), true),
        ]);
        assert_eq!(bar, "BOOT-INF/lib/foo.jar!/");
    }
}
