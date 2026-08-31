use serde::Serialize;
use similar::{ChangeTag, DiffOp, TextDiff};

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
pub struct TextSpan {
    pub text: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffRow {
    pub left_line: Option<u32>,
    pub right_line: Option<u32>,
    pub left_text: String,
    pub right_text: String,
    pub kind: DiffKind,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub left_spans: Vec<TextSpan>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub right_spans: Vec<TextSpan>,
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

const INLINE_MAX_BYTES: usize = 8_000;
const CHAR_REFINE_MIN_RATIO: f32 = 0.4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Word,
    Space,
    Punct,
    Other,
}

fn token_kind(c: char) -> TokenKind {
    if c.is_ascii_alphanumeric() || c == '_' {
        TokenKind::Word
    } else if c.is_whitespace() {
        TokenKind::Space
    } else if c.is_ascii() {
        TokenKind::Punct
    } else {
        TokenKind::Other
    }
}

fn tokenize(s: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0;
    let mut prev: Option<TokenKind> = None;
    for (i, c) in s.char_indices() {
        let kind = token_kind(c);
        let can_merge = match prev {
            Some(TokenKind::Other) => false,
            Some(prev_kind) if prev_kind == kind => true,
            Some(_) => false,
            None => true,
        };
        if prev.is_some() && !can_merge {
            tokens.push(&s[start..i]);
            start = i;
        }
        prev = Some(kind);
    }
    if prev.is_some() {
        tokens.push(&s[start..]);
    }
    tokens
}

fn push_span(spans: &mut Vec<TextSpan>, text: &str, changed: bool) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut() {
        if last.changed == changed {
            last.text.push_str(text);
            return;
        }
    }
    spans.push(TextSpan {
        text: text.to_string(),
        changed,
    });
}

fn whole_changed(text: &str) -> Vec<TextSpan> {
    if text.is_empty() {
        Vec::new()
    } else {
        vec![TextSpan {
            text: text.to_string(),
            changed: true,
        }]
    }
}

fn spans_from_char_diff(left: &str, right: &str) -> Option<(Vec<TextSpan>, Vec<TextSpan>)> {
    let diff = TextDiff::from_chars(left, right);
    if diff.ratio() < CHAR_REFINE_MIN_RATIO {
        return None;
    }
    let mut left_spans = Vec::new();
    let mut right_spans = Vec::new();
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                push_span(&mut left_spans, change.value(), false);
                push_span(&mut right_spans, change.value(), false);
            }
            ChangeTag::Delete => push_span(&mut left_spans, change.value(), true),
            ChangeTag::Insert => push_span(&mut right_spans, change.value(), true),
        }
    }
    Some((left_spans, right_spans))
}

fn inline_spans(left: &str, right: &str) -> (Vec<TextSpan>, Vec<TextSpan>) {
    if left.is_empty() {
        return (Vec::new(), whole_changed(right));
    }
    if right.is_empty() {
        return (whole_changed(left), Vec::new());
    }
    if left.len() + right.len() > INLINE_MAX_BYTES {
        return (whole_changed(left), whole_changed(right));
    }

    let left_tokens = tokenize(left);
    let right_tokens = tokenize(right);
    let diff = TextDiff::from_slices(&left_tokens, &right_tokens);
    let mut left_spans = Vec::new();
    let mut right_spans = Vec::new();

    for op in diff.ops() {
        match *op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for i in 0..len {
                    push_span(&mut left_spans, left_tokens[old_index + i], false);
                    push_span(&mut right_spans, right_tokens[new_index + i], false);
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for i in 0..old_len {
                    push_span(&mut left_spans, left_tokens[old_index + i], true);
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for i in 0..new_len {
                    push_span(&mut right_spans, right_tokens[new_index + i], true);
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let hunk_left: String = (0..old_len).map(|i| left_tokens[old_index + i]).collect();
                let hunk_right: String = (0..new_len).map(|i| right_tokens[new_index + i]).collect();
                if let Some((ls, rs)) = spans_from_char_diff(&hunk_left, &hunk_right) {
                    for span in ls {
                        push_span(&mut left_spans, &span.text, span.changed);
                    }
                    for span in rs {
                        push_span(&mut right_spans, &span.text, span.changed);
                    }
                } else {
                    push_span(&mut left_spans, &hunk_left, true);
                    push_span(&mut right_spans, &hunk_right, true);
                }
            }
        }
    }

    (left_spans, right_spans)
}

fn diff_row(
    left_line: Option<u32>,
    right_line: Option<u32>,
    left_text: String,
    right_text: String,
    kind: DiffKind,
) -> DiffRow {
    let (left_spans, right_spans) = match kind {
        DiffKind::Equal => (Vec::new(), Vec::new()),
        _ => inline_spans(&left_text, &right_text),
    };
    DiffRow {
        left_line,
        right_line,
        left_text,
        right_text,
        kind,
        left_spans,
        right_spans,
    }
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
                    rows.push(diff_row(
                        Some(line_number(old_index, i)),
                        Some(line_number(new_index, i)),
                        text.clone(),
                        text,
                        DiffKind::Equal,
                    ));
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for i in 0..old_len {
                    rows.push(diff_row(
                        Some(line_number(old_index, i)),
                        None,
                        strip_nl(old[old_index + i]),
                        String::new(),
                        DiffKind::Delete,
                    ));
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for i in 0..new_len {
                    rows.push(diff_row(
                        None,
                        Some(line_number(new_index, i)),
                        String::new(),
                        strip_nl(new[new_index + i]),
                        DiffKind::Insert,
                    ));
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
                    rows.push(diff_row(
                        if i < old_len {
                            Some(line_number(old_index, i))
                        } else {
                            None
                        },
                        if i < new_len {
                            Some(line_number(new_index, i))
                        } else {
                            None
                        },
                        left_text,
                        right_text,
                        DiffKind::Replace,
                    ));
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

    fn changed_text(spans: &[TextSpan]) -> String {
        spans
            .iter()
            .filter(|span| span.changed)
            .map(|span| span.text.as_str())
            .collect()
    }

    fn equal_text(spans: &[TextSpan]) -> String {
        spans
            .iter()
            .filter(|span| !span.changed)
            .map(|span| span.text.as_str())
            .collect()
    }

    #[test]
    fn replace_highlights_changed_words() {
        let result = align_diff("hello world\n", "hello there\n");
        let row = &result.rows[0];
        assert_eq!(row.kind, DiffKind::Replace);
        assert_eq!(equal_text(&row.left_spans), "hello ");
        assert_eq!(changed_text(&row.left_spans), "world");
        assert_eq!(equal_text(&row.right_spans), "hello ");
        assert_eq!(changed_text(&row.right_spans), "there");
    }

    #[test]
    fn replace_highlights_changed_cjk() {
        let result = align_diff("你好世界\n", "你好朋友\n");
        let row = &result.rows[0];
        assert_eq!(changed_text(&row.left_spans), "世界");
        assert_eq!(changed_text(&row.right_spans), "朋友");
        assert_eq!(equal_text(&row.left_spans), "你好");
    }

    #[test]
    fn replace_refines_similar_words() {
        let result = align_diff("hello1\n", "hello2\n");
        let row = &result.rows[0];
        assert_eq!(equal_text(&row.left_spans), "hello");
        assert_eq!(changed_text(&row.left_spans), "1");
        assert_eq!(changed_text(&row.right_spans), "2");
    }

    #[test]
    fn delete_marks_whole_left_line() {
        let result = align_diff("keep\ngone\n", "keep\n");
        let row = &result.rows[1];
        assert!(row.left_spans.iter().all(|span| span.changed));
        assert_eq!(changed_text(&row.left_spans), "gone");
        assert!(row.right_spans.is_empty());
    }

    #[test]
    fn equal_rows_have_no_spans() {
        let result = align_diff("hello\n", "hello\n");
        assert!(result.rows[0].left_spans.is_empty());
        assert!(result.rows[0].right_spans.is_empty());
    }
}
