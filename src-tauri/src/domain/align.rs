use serde::Serialize;
use similar::{DiffOp, TextDiff};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffKind {
    Equal,
    Delete,
    Insert,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffRow {
    pub left_line: Option<u32>,
    pub right_line: Option<u32>,
    pub left_text: String,
    pub right_text: String,
    pub kind: DiffKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffStats {
    pub equal: u32,
    pub insert: u32,
    pub delete: u32,
    pub replace: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub rows: Vec<DiffRow>,
    pub stats: DiffStats,
}

fn line_number(start: usize, offset: usize) -> u32 {
    (start + offset + 1) as u32
}

fn strip_nl(value: &str) -> String {
    value.trim_end_matches(['\n', '\r']).to_string()
}

fn stats_from(rows: &[DiffRow]) -> DiffStats {
    let mut stats = DiffStats {
        equal: 0,
        insert: 0,
        delete: 0,
        replace: 0,
    };
    for row in rows {
        match row.kind {
            DiffKind::Equal => stats.equal += 1,
            DiffKind::Insert => stats.insert += 1,
            DiffKind::Delete => stats.delete += 1,
            DiffKind::Replace => stats.replace += 1,
        }
    }
    stats
}

pub fn align_diff(left: &str, right: &str) -> DiffResult {
    if left.is_empty() && right.is_empty() {
        return DiffResult {
            rows: vec![],
            stats: DiffStats {
                equal: 0,
                insert: 0,
                delete: 0,
                replace: 0,
            },
        };
    }

    let diff = TextDiff::from_lines(left, right);
    let old = diff.old_slices();
    let new = diff.new_slices();
    let mut rows = Vec::new();

    for op in diff.ops() {
        match *op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for i in 0..len {
                    let text = strip_nl(old[old_index + i]);
                    rows.push(DiffRow {
                        left_line: Some(line_number(old_index, i)),
                        right_line: Some(line_number(new_index, i)),
                        left_text: text.clone(),
                        right_text: text,
                        kind: DiffKind::Equal,
                    });
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for i in 0..old_len {
                    rows.push(DiffRow {
                        left_line: Some(line_number(old_index, i)),
                        right_line: None,
                        left_text: strip_nl(old[old_index + i]),
                        right_text: String::new(),
                        kind: DiffKind::Delete,
                    });
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for i in 0..new_len {
                    rows.push(DiffRow {
                        left_line: None,
                        right_line: Some(line_number(new_index, i)),
                        left_text: String::new(),
                        right_text: strip_nl(new[new_index + i]),
                        kind: DiffKind::Insert,
                    });
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let max = old_len.max(new_len);
                for i in 0..max {
                    let left_text = if i < old_len {
                        strip_nl(old[old_index + i])
                    } else {
                        String::new()
                    };
                    let right_text = if i < new_len {
                        strip_nl(new[new_index + i])
                    } else {
                        String::new()
                    };
                    rows.push(DiffRow {
                        left_line: if i < old_len {
                            Some(line_number(old_index, i))
                        } else {
                            None
                        },
                        right_line: if i < new_len {
                            Some(line_number(new_index, i))
                        } else {
                            None
                        },
                        left_text,
                        right_text,
                        kind: DiffKind::Replace,
                    });
                }
            }
        }
    }

    let stats = stats_from(&rows);
    DiffResult { rows, stats }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_inputs_yield_empty_result() {
        let result = align_diff("", "");
        assert_eq!(
            result,
            DiffResult {
                rows: vec![],
                stats: DiffStats {
                    equal: 0,
                    insert: 0,
                    delete: 0,
                    replace: 0
                }
            }
        );
    }

    #[test]
    fn identical_texts_are_equal_rows() {
        let result = align_diff("hello\nworld\n", "hello\nworld\n");
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[0].kind, DiffKind::Equal);
        assert_eq!(result.rows[0].left_line, Some(1));
        assert_eq!(result.rows[0].right_line, Some(1));
        assert_eq!(result.rows[0].left_text, "hello");
        assert_eq!(result.rows[1].left_line, Some(2));
        assert_eq!(result.stats.equal, 2);
    }

    #[test]
    fn insertion_only_pads_the_left() {
        let result = align_diff("keep\n", "keep\nnew\n");
        assert_eq!(result.rows[1].kind, DiffKind::Insert);
        assert_eq!(result.rows[1].left_line, None);
        assert_eq!(result.rows[1].left_text, "");
        assert_eq!(result.rows[1].right_text, "new");
        assert_eq!(result.stats.insert, 1);
    }

    #[test]
    fn deletion_only_pads_the_right() {
        let result = align_diff("keep\ngone\n", "keep\n");
        assert_eq!(result.rows[1].kind, DiffKind::Delete);
        assert_eq!(result.rows[1].right_line, None);
        assert_eq!(result.rows[1].right_text, "");
        assert_eq!(result.rows[1].left_text, "gone");
        assert_eq!(result.stats.delete, 1);
    }

    #[test]
    fn replace_pads_the_shorter_side() {
        let result = align_diff("a\nb\n", "x\n");
        assert_eq!(result.rows.len(), 2);
        assert!(result.rows.iter().all(|row| row.kind == DiffKind::Replace));
        assert_eq!(result.rows[0].left_text, "a");
        assert_eq!(result.rows[0].right_text, "x");
        assert_eq!(result.rows[1].left_text, "b");
        assert_eq!(result.rows[1].right_line, None);
        assert_eq!(result.stats.replace, 2);
    }
}
