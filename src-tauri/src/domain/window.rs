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
    let total = rows
        .iter()
        .filter(|row| row_matches(row.kind, filter))
        .count();
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

pub fn dirty_row_indexes(rows: &[DiffRow]) -> Vec<u32> {
    rows.iter()
        .enumerate()
        .filter(|(_, row)| row.kind != DiffKind::Equal)
        .map(|(index, _)| index as u32)
        .collect()
}

pub fn cap_marks(marks: Vec<u32>, cap: usize) -> Vec<u32> {
    if marks.len() <= cap || cap == 0 {
        return marks;
    }
    if cap == 1 {
        return vec![marks[0]];
    }
    let mut out = Vec::with_capacity(cap);
    let last = marks.len() - 1;
    for i in 0..cap {
        let index = i * last / (cap - 1);
        let value = marks[index];
        if out.last() != Some(&value) {
            out.push(value);
        }
    }
    out
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

    #[test]
    fn dirty_row_indexes_skip_equal() {
        let result = align_diff("keep\nold\n", "keep\nnew\n");
        assert_eq!(dirty_row_indexes(&result.rows), vec![1]);
    }

    #[test]
    fn cap_marks_keeps_ends() {
        let marks: Vec<u32> = (0..100).collect();
        let capped = cap_marks(marks, 5);
        assert_eq!(capped.first().copied(), Some(0));
        assert_eq!(capped.last().copied(), Some(99));
        assert!(capped.len() <= 5);
    }
}
