use std::fs;

pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

pub fn reject_if_too_large(len: u64) -> Result<(), String> {
    if len > MAX_FILE_BYTES {
        Err("文件超过 64MB".to_string())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    let metadata = fs::metadata(&path).map_err(|_| "无法打开文件".to_string())?;
    reject_if_too_large(metadata.len())?;
    let bytes = fs::read(&path).map_err(|_| "无法打开文件".to_string())?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| "无法读取文件：不是有效的 UTF-8".to_string())?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("comparew-{name}-{}", std::process::id()));
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn rejects_sizes_over_64_mib() {
        assert!(reject_if_too_large(MAX_FILE_BYTES).is_ok());
        assert_eq!(
            reject_if_too_large(MAX_FILE_BYTES + 1).unwrap_err(),
            "文件超过 64MB"
        );
    }

    #[test]
    fn rejects_invalid_utf8() {
        let path = temp_path("bad-utf8");
        fs::write(&path, [0xff, 0xfe, 0xfd]).unwrap();
        let err = read_text_file(path.to_string_lossy().into()).unwrap_err();
        assert_eq!(err, "无法读取文件：不是有效的 UTF-8");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn strips_utf8_bom() {
        let path = temp_path("bom");
        let mut bytes = vec![0xef, 0xbb, 0xbf];
        bytes.extend_from_slice("hello".as_bytes());
        fs::write(&path, bytes).unwrap();
        let text = read_text_file(path.to_string_lossy().into()).unwrap();
        assert_eq!(text, "hello");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_file_is_cannot_open() {
        let err = read_text_file("/tmp/comparew-definitely-missing-file.txt".into()).unwrap_err();
        assert_eq!(err, "无法打开文件");
    }
}
