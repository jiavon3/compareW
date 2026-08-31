use serde::Serialize;
use similar::{capture_diff_slices, Algorithm, DiffOp};

const MAX_SHEET_CELLS: usize = 500_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExcelSheetStatus {
    Equal,
    Different,
    LeftOnly,
    RightOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelCell {
    pub left: String,
    pub right: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelRow {
    pub left_index: Option<u32>,
    pub right_index: Option<u32>,
    pub dirty: bool,
    pub cells: Vec<ExcelCell>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelSheet {
    pub name: String,
    pub status: ExcelSheetStatus,
    pub width: u32,
    pub rows: Vec<ExcelRow>,
    pub dirty_rows: Vec<u32>,
    pub changed_cells: u32,
}

pub type Table = Vec<Vec<String>>;

pub fn format_number(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        let text = format!("{value}");
        if text.contains('.') {
            text.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            text
        }
    }
}

pub fn align_names(left: &[String], right: &[String]) -> Vec<(Option<usize>, Option<usize>)> {
    let mut used_right = vec![false; right.len()];
    let mut pairs = Vec::with_capacity(left.len() + right.len());
    for (left_index, left_name) in left.iter().enumerate() {
        let found = right.iter().enumerate().find(|(right_index, right_name)| {
            !used_right[*right_index] && right_name.eq_ignore_ascii_case(left_name)
        });
        if let Some((right_index, _)) = found {
            used_right[right_index] = true;
            pairs.push((Some(left_index), Some(right_index)));
        } else {
            pairs.push((Some(left_index), None));
        }
    }
    for (right_index, used) in used_right.iter().enumerate() {
        if !*used {
            pairs.push((None, Some(right_index)));
        }
    }
    pairs
}

fn cell_at(table: &Table, row: usize, col: usize) -> &str {
    table
        .get(row)
        .and_then(|line| line.get(col))
        .map(String::as_str)
        .unwrap_or("")
}

fn padded_row(table: &Table, row: usize, width: usize) -> Vec<String> {
    (0..width)
        .map(|col| cell_at(table, row, col).to_string())
        .collect()
}

fn excel_line(start: usize, offset: usize) -> u32 {
    (start + offset + 1) as u32
}

fn pair_cells(
    left: Option<&[String]>,
    right: Option<&[String]>,
    width: usize,
) -> (Vec<ExcelCell>, bool, u32) {
    if width == 0 {
        let dirty = left.is_none() || right.is_none();
        return (Vec::new(), dirty, 0);
    }
    let mut cells = Vec::with_capacity(width);
    let mut dirty = false;
    let mut changed_cells = 0u32;
    for col in 0..width {
        let left_value = left
            .and_then(|row| row.get(col))
            .cloned()
            .unwrap_or_default();
        let right_value = right
            .and_then(|row| row.get(col))
            .cloned()
            .unwrap_or_default();
        let changed = left.is_none() || right.is_none() || left_value != right_value;
        if changed {
            dirty = true;
            changed_cells += 1;
        }
        cells.push(ExcelCell {
            left: left_value,
            right: right_value,
            changed,
        });
    }
    (cells, dirty, changed_cells)
}

fn push_aligned_row(
    rows: &mut Vec<ExcelRow>,
    dirty_rows: &mut Vec<u32>,
    changed_cells: &mut u32,
    left_index: Option<u32>,
    right_index: Option<u32>,
    left: Option<&[String]>,
    right: Option<&[String]>,
    width: usize,
) {
    let (cells, dirty, changed) = pair_cells(left, right, width);
    *changed_cells += changed;
    if dirty {
        dirty_rows.push(rows.len() as u32);
    }
    rows.push(ExcelRow {
        left_index,
        right_index,
        dirty,
        cells,
    });
}

pub fn align_grid(
    name: String,
    left: Option<&Table>,
    right: Option<&Table>,
) -> Result<ExcelSheet, String> {
    let empty: Table = Vec::new();
    let left_table = left.unwrap_or(&empty);
    let right_table = right.unwrap_or(&empty);
    let width = left_table
        .iter()
        .chain(right_table.iter())
        .map(|row| row.len())
        .max()
        .unwrap_or(0);
    let bound = left_table
        .len()
        .saturating_add(right_table.len())
        .saturating_mul(width.max(1));
    if bound > MAX_SHEET_CELLS {
        return Err(format!("工作表过大：{name}"));
    }

    let left_rows: Vec<Vec<String>> = (0..left_table.len())
        .map(|row| padded_row(left_table, row, width))
        .collect();
    let right_rows: Vec<Vec<String>> = (0..right_table.len())
        .map(|row| padded_row(right_table, row, width))
        .collect();
    let ops = capture_diff_slices(Algorithm::Myers, &left_rows, &right_rows);

    let mut rows = Vec::new();
    let mut dirty_rows = Vec::new();
    let mut changed_cells = 0u32;

    for op in ops {
        match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for i in 0..len {
                    push_aligned_row(
                        &mut rows,
                        &mut dirty_rows,
                        &mut changed_cells,
                        Some(excel_line(old_index, i)),
                        Some(excel_line(new_index, i)),
                        Some(&left_rows[old_index + i]),
                        Some(&right_rows[new_index + i]),
                        width,
                    );
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for i in 0..old_len {
                    push_aligned_row(
                        &mut rows,
                        &mut dirty_rows,
                        &mut changed_cells,
                        Some(excel_line(old_index, i)),
                        None,
                        Some(&left_rows[old_index + i]),
                        None,
                        width,
                    );
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for i in 0..new_len {
                    push_aligned_row(
                        &mut rows,
                        &mut dirty_rows,
                        &mut changed_cells,
                        None,
                        Some(excel_line(new_index, i)),
                        None,
                        Some(&right_rows[new_index + i]),
                        width,
                    );
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
                    let left_row = (i < old_len).then(|| left_rows[old_index + i].as_slice());
                    let right_row = (i < new_len).then(|| right_rows[new_index + i].as_slice());
                    push_aligned_row(
                        &mut rows,
                        &mut dirty_rows,
                        &mut changed_cells,
                        (i < old_len).then(|| excel_line(old_index, i)),
                        (i < new_len).then(|| excel_line(new_index, i)),
                        left_row,
                        right_row,
                        width,
                    );
                }
            }
        }
    }

    if rows.len().saturating_mul(width.max(1)) > MAX_SHEET_CELLS {
        return Err(format!("工作表过大：{name}"));
    }

    let status = match (left, right) {
        (None, Some(_)) => ExcelSheetStatus::RightOnly,
        (Some(_), None) => ExcelSheetStatus::LeftOnly,
        _ if dirty_rows.is_empty() => ExcelSheetStatus::Equal,
        _ => ExcelSheetStatus::Different,
    };

    Ok(ExcelSheet {
        name,
        status,
        width: width as u32,
        rows,
        dirty_rows,
        changed_cells,
    })
}

pub fn align_workbooks(
    left: Vec<(String, Table)>,
    right: Vec<(String, Table)>,
) -> Result<Vec<ExcelSheet>, String> {
    let left_names: Vec<String> = left.iter().map(|(name, _)| name.clone()).collect();
    let right_names: Vec<String> = right.iter().map(|(name, _)| name.clone()).collect();
    let mut sheets = Vec::new();
    for (left_index, right_index) in align_names(&left_names, &right_names) {
        let name = match (left_index, right_index) {
            (Some(index), _) => left[index].0.clone(),
            (_, Some(index)) => right[index].0.clone(),
            _ => "Sheet".to_string(),
        };
        let left_table = left_index.map(|index| &left[index].1);
        let right_table = right_index.map(|index| &right[index].1);
        sheets.push(align_grid(name, left_table, right_table)?);
    }
    Ok(sheets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_whole_float_as_int() {
        assert_eq!(format_number(12.0), "12");
        assert_eq!(format_number(12.5), "12.5");
    }

    #[test]
    fn names_match_case_insensitive() {
        let pairs = align_names(
            &["Sheet1".into(), "Data".into()],
            &["data".into(), "Extra".into()],
        );
        assert_eq!(
            pairs,
            vec![(Some(0), None), (Some(1), Some(0)), (None, Some(1))]
        );
    }

    #[test]
    fn grid_marks_changed_cells() {
        let left = vec![vec!["a".into(), "b".into()]];
        let right = vec![vec!["a".into(), "c".into()]];
        let sheet = align_grid("S".into(), Some(&left), Some(&right)).unwrap();
        assert_eq!(sheet.status, ExcelSheetStatus::Different);
        assert!(!sheet.rows[0].cells[0].changed);
        assert!(sheet.rows[0].cells[1].changed);
        assert_eq!(sheet.dirty_rows, vec![0]);
        assert_eq!(sheet.changed_cells, 1);
    }

    #[test]
    fn left_only_sheet_is_dirty() {
        let left = vec![vec!["x".into()]];
        let sheet = align_grid("Only".into(), Some(&left), None).unwrap();
        assert_eq!(sheet.status, ExcelSheetStatus::LeftOnly);
        assert!(sheet.rows[0].cells[0].changed);
        assert_eq!(sheet.rows[0].cells[0].right, "");
    }

    #[test]
    fn identical_grids_are_equal() {
        let table = vec![vec!["1".into(), "2".into()]];
        let sheet = align_grid("S".into(), Some(&table), Some(&table)).unwrap();
        assert_eq!(sheet.status, ExcelSheetStatus::Equal);
        assert!(sheet.dirty_rows.is_empty());
    }

    #[test]
    fn inserted_row_keeps_later_rows_aligned() {
        let left = vec![vec!["a".into()], vec!["b".into()], vec!["c".into()]];
        let right = vec![
            vec!["a".into()],
            vec!["x".into()],
            vec!["b".into()],
            vec!["c".into()],
        ];
        let sheet = align_grid("S".into(), Some(&left), Some(&right)).unwrap();
        assert_eq!(sheet.rows.len(), 4);
        assert!(!sheet.rows[0].dirty);
        assert!(sheet.rows[1].dirty);
        assert!(!sheet.rows[2].dirty);
        assert!(!sheet.rows[3].dirty);
        assert_eq!(sheet.rows[1].cells[0].left, "");
        assert_eq!(sheet.rows[1].cells[0].right, "x");
        assert_eq!(sheet.rows[2].cells[0].left, "b");
        assert_eq!(sheet.rows[2].cells[0].right, "b");
        assert_eq!(sheet.dirty_rows, vec![1]);
        assert_eq!(sheet.changed_cells, 1);
        assert_eq!(sheet.rows[0].left_index, Some(1));
        assert_eq!(sheet.rows[0].right_index, Some(1));
        assert_eq!(sheet.rows[1].left_index, None);
        assert_eq!(sheet.rows[1].right_index, Some(2));
        assert_eq!(sheet.rows[2].left_index, Some(2));
        assert_eq!(sheet.rows[2].right_index, Some(3));
        assert_eq!(sheet.rows[3].left_index, Some(3));
        assert_eq!(sheet.rows[3].right_index, Some(4));
    }

    #[test]
    fn deleted_row_keeps_later_rows_aligned() {
        let left = vec![vec!["a".into()], vec!["x".into()], vec!["b".into()]];
        let right = vec![vec!["a".into()], vec!["b".into()]];
        let sheet = align_grid("S".into(), Some(&left), Some(&right)).unwrap();
        assert_eq!(sheet.rows.len(), 3);
        assert!(!sheet.rows[0].dirty);
        assert!(sheet.rows[1].dirty);
        assert!(!sheet.rows[2].dirty);
        assert_eq!(sheet.rows[1].cells[0].left, "x");
        assert_eq!(sheet.rows[1].cells[0].right, "");
        assert_eq!(sheet.rows[2].cells[0].left, "b");
        assert_eq!(sheet.rows[2].cells[0].right, "b");
        assert_eq!(sheet.dirty_rows, vec![1]);
        assert_eq!(sheet.rows[1].left_index, Some(2));
        assert_eq!(sheet.rows[1].right_index, None);
        assert_eq!(sheet.rows[2].left_index, Some(3));
        assert_eq!(sheet.rows[2].right_index, Some(2));
    }

    #[test]
    fn changed_cell_stays_on_the_same_row() {
        let left = vec![vec!["a".into()], vec!["b".into()], vec!["c".into()]];
        let right = vec![vec!["a".into()], vec!["B".into()], vec!["c".into()]];
        let sheet = align_grid("S".into(), Some(&left), Some(&right)).unwrap();
        assert_eq!(sheet.rows.len(), 3);
        assert!(!sheet.rows[0].dirty);
        assert!(sheet.rows[1].dirty);
        assert!(!sheet.rows[2].dirty);
        assert_eq!(sheet.rows[1].left_index, Some(2));
        assert_eq!(sheet.rows[1].right_index, Some(2));
        assert!(sheet.rows[1].cells[0].changed);
        assert_eq!(sheet.changed_cells, 1);
    }
}
