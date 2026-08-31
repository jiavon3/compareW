use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::domain::align::{align_diff, DiffRow, DiffStats};
use crate::domain::window::{cap_marks, dirty_lines, dirty_row_indexes, window_rows};

const DIRTY_LINE_CAP: usize = 40_000;
const DIFF_MARK_CAP: usize = 4_000;

#[derive(Default)]
pub struct DiffStore {
    rows: Vec<DiffRow>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareSummary {
    pub stats: DiffStats,
    pub row_count: u32,
    pub dirty_left: Vec<u32>,
    pub dirty_right: Vec<u32>,
    pub diff_marks: Vec<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffWindow {
    pub rows: Vec<DiffRow>,
    pub total: u32,
    pub offset: u32,
}

fn summary_from(rows: &[DiffRow], stats: &DiffStats) -> CompareSummary {
    let include_dirty = rows.len() <= DIRTY_LINE_CAP;
    CompareSummary {
        stats: stats.clone(),
        row_count: rows.len() as u32,
        dirty_left: if include_dirty {
            dirty_lines(rows, true)
        } else {
            Vec::new()
        },
        dirty_right: if include_dirty {
            dirty_lines(rows, false)
        } else {
            Vec::new()
        },
        diff_marks: cap_marks(dirty_row_indexes(rows), DIFF_MARK_CAP),
    }
}

#[tauri::command]
pub fn compare_texts(
    left: String,
    right: String,
    store: State<Mutex<DiffStore>>,
) -> CompareSummary {
    let result = align_diff(&left, &right);
    let summary = summary_from(&result.rows, &result.stats);
    *store.lock().expect("diff store") = DiffStore { rows: result.rows };
    summary
}

#[tauri::command]
pub fn get_diff_rows(
    filter: String,
    offset: u32,
    limit: u32,
    store: State<Mutex<DiffStore>>,
) -> DiffWindow {
    let store = store.lock().expect("diff store");
    let limit = limit.clamp(1, 300) as usize;
    let offset = offset as usize;
    let (rows, total) = window_rows(&store.rows, &filter, offset, limit);
    DiffWindow {
        rows,
        total: total as u32,
        offset: offset as u32,
    }
}
