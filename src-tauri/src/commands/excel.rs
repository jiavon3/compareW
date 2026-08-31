use std::fs;
use std::sync::Mutex;

use calamine::{open_workbook_auto, Data, Reader};
use serde::Serialize;
use tauri::State;

use crate::commands::file::reject_if_too_large;
use crate::domain::excel::{
    align_workbooks, format_number, ExcelRow, ExcelSheet, ExcelSheetStatus, Table,
};
use crate::domain::window::cap_marks;

const DIFF_MARK_CAP: usize = 4_000;

#[derive(Default)]
pub struct ExcelStore {
    sheets: Vec<ExcelSheet>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelSheetInfo {
    pub name: String,
    pub status: ExcelSheetStatus,
    pub width: u32,
    pub height: u32,
    pub dirty_rows: Vec<u32>,
    pub changed_cells: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelSummary {
    pub sheets: Vec<ExcelSheetInfo>,
    pub changed_cells: u32,
    pub dirty_sheets: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelWindow {
    pub rows: Vec<ExcelRow>,
    pub total: u32,
    pub offset: u32,
    pub width: u32,
}

fn data_text(value: &Data) -> String {
    match value {
        Data::Empty => String::new(),
        Data::String(text) => text.clone(),
        Data::Int(n) => n.to_string(),
        Data::Float(n) => format_number(*n),
        Data::Bool(flag) => {
            if *flag {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Data::Error(err) => format!("#{err}"),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(text) | Data::DurationIso(text) => text.clone(),
    }
}

fn range_table(range: &calamine::Range<Data>) -> Table {
    range
        .rows()
        .map(|row| row.iter().map(data_text).collect())
        .collect()
}

fn load_path(path: &str) -> Result<Vec<(String, Table)>, String> {
    if path.is_empty() {
        return Ok(Vec::new());
    }
    let metadata = fs::metadata(path).map_err(|_| "无法打开 Excel".to_string())?;
    reject_if_too_large(metadata.len())?;
    let mut workbook = open_workbook_auto(path).map_err(|_| "无法打开 Excel".to_string())?;
    let names = workbook.sheet_names().to_vec();
    let mut sheets = Vec::with_capacity(names.len());
    for name in names {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|_| format!("无法读取工作表：{name}"))?;
        sheets.push((name, range_table(&range)));
    }
    Ok(sheets)
}

fn sheet_info(sheet: &ExcelSheet) -> ExcelSheetInfo {
    ExcelSheetInfo {
        name: sheet.name.clone(),
        status: sheet.status,
        width: sheet.width,
        height: sheet.rows.len() as u32,
        dirty_rows: cap_marks(sheet.dirty_rows.clone(), DIFF_MARK_CAP),
        changed_cells: sheet.changed_cells,
    }
}

fn summary_from(sheets: &[ExcelSheet]) -> ExcelSummary {
    let dirty_sheets = sheets
        .iter()
        .filter(|sheet| sheet.status != ExcelSheetStatus::Equal)
        .count() as u32;
    let changed_cells = sheets.iter().map(|sheet| sheet.changed_cells).sum();
    ExcelSummary {
        sheets: sheets.iter().map(sheet_info).collect(),
        changed_cells,
        dirty_sheets,
    }
}

fn row_matches(dirty: bool, filter: &str) -> bool {
    match filter {
        "same" => !dirty,
        "diff" => dirty,
        _ => true,
    }
}

#[tauri::command]
pub fn compare_excel(
    left: String,
    right: String,
    store: State<Mutex<ExcelStore>>,
) -> Result<ExcelSummary, String> {
    if left.is_empty() && right.is_empty() {
        *store.lock().expect("excel store") = ExcelStore::default();
        return Ok(ExcelSummary {
            sheets: Vec::new(),
            changed_cells: 0,
            dirty_sheets: 0,
        });
    }
    let left_sheets = load_path(&left)?;
    let right_sheets = load_path(&right)?;
    let sheets = align_workbooks(left_sheets, right_sheets)?;
    let summary = summary_from(&sheets);
    *store.lock().expect("excel store") = ExcelStore { sheets };
    Ok(summary)
}

#[tauri::command]
pub fn get_excel_rows(
    sheet: u32,
    filter: String,
    offset: u32,
    limit: u32,
    store: State<Mutex<ExcelStore>>,
) -> ExcelWindow {
    let store = store.lock().expect("excel store");
    let empty = ExcelWindow {
        rows: Vec::new(),
        total: 0,
        offset,
        width: 0,
    };
    let Some(current) = store.sheets.get(sheet as usize) else {
        return empty;
    };
    let limit = limit.clamp(1, 200) as usize;
    let offset = offset as usize;
    let total = current
        .rows
        .iter()
        .filter(|row| row_matches(row.dirty, &filter))
        .count();
    let rows = current
        .rows
        .iter()
        .filter(|row| row_matches(row.dirty, &filter))
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();
    ExcelWindow {
        rows,
        total: total as u32,
        offset: offset as u32,
        width: current.width,
    }
}
