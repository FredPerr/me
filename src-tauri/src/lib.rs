#[cfg(desktop)]
use tauri::Manager;

mod git;
mod git_provider;
mod ide_launcher;
mod shortcuts;
mod workspace_detector;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn open_in_ide(command: String, path: String, shell: Option<String>) -> Result<(), String> {
    ide_launcher::launch(&command, &path, shell.as_deref())
}

#[tauri::command]
async fn check_path_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_store::Builder::new().build());

    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_focus();
                }
            }))
            .plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(shortcuts::plugin())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            open_in_ide,
            check_path_exists,
            git::get_git_remote_url,
            git::list_worktrees,
            git::get_worktree_status,
            git::list_branches,
            git::list_local_branches,
            git::delete_local_branch,
            git::create_worktree,
            git::delete_worktree,
            git::prune_worktrees,
            git::create_context,
            git::delete_context,
            git::check_is_git_repository,
            git::pull_worktree,
            git::push_worktree,
            git_provider::github::github_start_device_authorization,
            git_provider::github::github_poll_for_access_token,
            git_provider::github::github_get_authenticated_user,
            git_provider::github::github_list_repositories,
            git_provider::github::github_list_pull_requests,
            git_provider::github::github_is_connected,
            git_provider::github::github_disconnect,
            workspace_detector::get_active_workspaces
        ])
        .setup(|app| {
            shortcuts::register(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
