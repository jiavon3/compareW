use serde::Serialize;

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
    pub index: u32,
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

pub fn align_grid(
    name: String,
    left: Option<&Table>,
    right: Option<&Table>,
) -> Result<ExcelSheet, String> {
    let left_table = left.unwrap_or(&Vec::new());
    let right_table = right.unwrap_or(&Vec::new());
    let height = left_table.len().max(right_table.len());
    let width = left_table
        .iter()
        .chain(right_table.iter())
        .map(|row| row.len())
        .max()
        .unwrap_or(0);
    if height.saturating_mul(width.max(1)) > MAX_SHEET_CELLS {
        return Err(format!("工作表过大：{name}"));
    }

    let mut rows = Vec::with_capacity(height);
    let mut dirty_rows = Vec::new();
    let mut changed_cells = 0u32;

    for row_index in 0..height {
        let mut cells = Vec::with_capacity(width);
        let mut dirty = false;
        for col in 0..width {
            let left_value = cell_at(left_table, row_index, col).to_string();
            let right_value = cell_at(right_table, row_index, col).to_string();
            let left_missing = left.is_none() || row_index >= left_table.len();
            let right_missing = right.is_none() || row_index >= right_table.len();
            let changed = left_value != right_value || left_missing != right_missing;
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
        if dirty {
            dirty_rows.push(row_index as u32);
        }
        rows.push(ExcelRow {
            index: (row_index as u32) + 1,
            dirty,
            cells,
        });
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
}
