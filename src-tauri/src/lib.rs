mod git;
mod shortcuts;
mod workspace_detector;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn open_in_ide(command: String, path: String) -> Result<(), String> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("{} \"{}\"", command, path))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(shortcuts::plugin())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            open_in_ide,
            git::get_git_remote_url,
            git::list_worktrees,
            git::get_worktree_status,
            git::list_branches,
            git::create_worktree,
            git::delete_worktree,
            git::create_context,
            git::delete_context,
            git::check_is_git_repository,
            workspace_detector::get_active_workspaces
        ])
        .setup(|app| {
            shortcuts::register(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
