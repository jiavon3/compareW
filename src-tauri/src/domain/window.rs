use super::align::{DiffKind, DiffRow};

pub fn row_matches(kind: DiffKind, filter: &str) -> bool {
    match filter {
        "same" => kind == DiffKind::Equal,
        "diff" => kind != DiffKind::Equal,
        _ => true,
    }
}

pub fn window_rows(
    rows: &[DiffRow],
    filter: &str,
    offset: usize,
    limit: usize,
) -> (Vec<DiffRow>, usize) {
    let total = rows.iter().filter(|row| row_matches(row.kind, filter)).count();
    let window = rows
        .iter()
        .filter(|row| row_matches(row.kind, filter))
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();
    (window, total)
}

pub fn dirty_lines(rows: &[DiffRow], left_side: bool) -> Vec<u32> {
    let mut lines = Vec::new();
    for row in rows {
        if row.kind == DiffKind::Equal {
            continue;
        }
        let line = if left_side {
            row.left_line
        } else {
            row.right_line
        };
        if let Some(number) = line {
            lines.push(number);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::align::align_diff;

    #[test]
    fn diff_filter_returns_only_inconsistent_rows() {
        let result = align_diff("keep\nold\n", "keep\nnew\n");
        let (window, total) = window_rows(&result.rows, "diff", 0, 20);
        assert_eq!(total, 1);
        assert_eq!(window.len(), 1);
        assert_eq!(window[0].kind, DiffKind::Replace);
    }

    #[test]
    fn same_filter_returns_only_equal_rows() {
        let result = align_diff("keep\nold\n", "keep\nnew\n");
        let (window, total) = window_rows(&result.rows, "same", 0, 20);
        assert_eq!(total, 1);
        assert_eq!(window[0].kind, DiffKind::Equal);
        assert_eq!(window[0].left_text, "keep");
    }

    #[test]
    fn window_offset_skips_filtered_rows() {
        let result = align_diff("a\nb\nc\n", "a\nb\nx\n");
        let (window, total) = window_rows(&result.rows, "all", 2, 1);
        assert_eq!(total, 3);
        assert_eq!(window.len(), 1);
        assert_eq!(window[0].left_text, "c");
    }

    #[test]
    fn dirty_lines_skip_equal_and_gaps() {
        let result = align_diff("keep\ngone\n", "keep\n");
        let left = dirty_lines(&result.rows, true);
        let right = dirty_lines(&result.rows, false);
        assert_eq!(left, vec![2]);
        assert!(right.is_empty());
    }
}
