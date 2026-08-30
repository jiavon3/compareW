#[cfg(target_os = "macos")]
mod macos {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSOpenPanel;
    use objc2_foundation::NSString;

    pub fn pick_file_or_folder() -> Option<String> {
        let mtm = MainThreadMarker::new().expect("NSOpenPanel must run on the main thread");
        let panel = unsafe { NSOpenPanel::openPanel(mtm) };
        panel.setCanChooseFiles(true);
        panel.setCanChooseDirectories(true);
        panel.setAllowsMultipleSelection(false);
        panel.setCanCreateDirectories(false);
        panel.setResolvesAliases(true);
        panel.setTitle(Some(&NSString::from_str("选择文件夹或 jar/zip")));
        panel.setMessage(Some(&NSString::from_str(
            "可以选择文件夹，也可以选择 .jar / .zip / .war",
        )));
        panel.setAllowedFileTypes(None);
        let response = panel.runModal();
        if response != 1 {
            return None;
        }
        panel
            .URL()
            .and_then(|url| url.path())
            .map(|path| path.to_string())
    }
}

#[tauri::command]
pub fn pick_compare_root(app: tauri::AppHandle) -> Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let _ = tx.send(macos::pick_file_or_folder());
        })
        .map_err(|err| err.to_string())?;
        rx.recv().map_err(|err| err.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("unsupported".into())
    }
}
