use tauri::Manager;

mod ai_session;
mod binary_resolver;
mod context_checkout;
mod git;
mod git_provider;
mod ide_launcher;
mod pull_request_template;
mod secret_store;
mod shortcuts;
mod work_tracking;
mod workspace;
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

/// TEMPORARY: commands already moved to tauri-specta. The dispatcher in [`run`]
/// routes these to the typed handler and every other command to the old
/// `generate_handler!`. Delete this list, the dispatcher and the old handler
/// once the last command has moved. Names are the Rust function names.
const TYPED_COMMANDS: &[&str] = &[
    "check_is_git_repository",
    "get_git_remote_url",
    "list_worktrees",
    "get_worktree_status",
    "get_worktree_diff",
    "read_pull_request_template",
    "list_branches",
    "list_local_branches",
    "delete_local_branch",
    "delete_worktree",
    "prune_worktrees",
    "create_context",
    "delete_context",
    "pull_worktree",
    "push_worktree",
    "fetch_pr_branch",
    "workspace_list_projects",
    "workspace_get_project",
    "workspace_create_context",
    "workspace_delete_context",
];

/// The typed commands, exported with their types to `src/bindings.ts`.
fn typed_commands() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .commands(tauri_specta::collect_commands![
            git::commands::check_is_git_repository,
            git::commands::get_git_remote_url,
            git::commands::list_worktrees,
            git::commands::get_worktree_status,
            git::commands::get_worktree_diff,
            pull_request_template::read_pull_request_template,
            git::commands::list_branches,
            git::commands::list_local_branches,
            git::commands::delete_local_branch,
            git::commands::delete_worktree,
            git::commands::prune_worktrees,
            context_checkout::create_context,
            context_checkout::delete_context,
            git::commands::pull_worktree,
            git::commands::push_worktree,
            git::commands::fetch_pr_branch,
            workspace::commands::workspace_list_projects,
            workspace::commands::workspace_get_project,
            workspace::commands::workspace_create_context,
            workspace::commands::workspace_delete_context,
        ])
        .events(tauri_specta::collect_events![
            workspace::infra::tauri_project_events::ProjectChanged
        ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let typed_builder = typed_commands();
    let typed_handler = typed_builder.invoke_handler();
    // TEMPORARY: see TYPED_COMMANDS.
    let legacy_handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
        greet,
        open_in_ide,
        check_path_exists,
        git_provider::github::github_start_device_authorization,
        git_provider::github::github_poll_for_access_token,
        git_provider::github::github_get_authenticated_user,
        git_provider::github::github_list_repositories,
        git_provider::github::github_list_pull_requests,
        git_provider::github::github_create_pull_request,
        git_provider::github::github_is_connected,
        git_provider::github::github_disconnect,
        git_provider::bitbucket::bitbucket_connect_with_token,
        git_provider::bitbucket::bitbucket_get_authenticated_user,
        git_provider::bitbucket::bitbucket_list_repositories,
        git_provider::bitbucket::bitbucket_list_pull_requests,
        git_provider::bitbucket::bitbucket_create_pull_request,
        git_provider::bitbucket::bitbucket_is_connected,
        git_provider::bitbucket::bitbucket_disconnect,
        work_tracking::commands::work_tracking_list_connections,
        work_tracking::commands::work_tracking_save_connection,
        work_tracking::commands::work_tracking_remove_connection,
        work_tracking::commands::work_tracking_list_projects,
        work_tracking::commands::work_tracking_list_groups,
        work_tracking::commands::work_tracking_list_items,
        work_tracking::commands::work_tracking_list_project_items,
        work_tracking::commands::work_tracking_assign_me_to_task,
        workspace_detector::get_active_workspaces,
        ai_session::spawn_ai_session,
        ai_session::kill_ai_session,
        ai_session::list_ai_sessions,
        ai_session::list_ai_adapters
    ];
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
        .invoke_handler(move |invoke| {
            if TYPED_COMMANDS.contains(&invoke.message.command()) {
                typed_handler(invoke)
            } else {
                legacy_handler(invoke)
            }
        })
        .setup(move |app| {
            typed_builder.mount_events(app);
            tauri::Manager::manage(app, work_tracking::build_service(app.handle().clone()));
            app.manage(ai_session::registry::SessionRegistry::new());
            let git = git::build();
            let workspace = workspace::build(
                app.handle().clone(),
                &app.path().app_data_dir()?,
                git.repositories(),
            )?;
            app.manage(git);
            app.manage(workspace);
            shortcuts::register(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod bindings_tests {
    use std::path::PathBuf;

    const BINDINGS_FILE: &str = "../src/bindings.ts";

    /// Regenerates `src/bindings.ts` and fails when the committed file was out
    /// of date, so CI catches Rust changes whose bindings were not committed.
    #[test]
    fn committed_bindings_are_current() {
        let committed_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(BINDINGS_FILE);
        let generated_path =
            std::env::temp_dir().join(format!("me-bindings-{}.ts", std::process::id()));
        super::typed_commands()
            .export(specta_typescript::Typescript::default(), &generated_path)
            .expect("export TypeScript bindings");
        let generated = std::fs::read_to_string(&generated_path).expect("read generated bindings");
        let _ = std::fs::remove_file(&generated_path);
        let committed = std::fs::read_to_string(&committed_path).unwrap_or_default();
        if committed != generated {
            std::fs::write(&committed_path, &generated).expect("write bindings");
            panic!("src/bindings.ts was out of date and has been regenerated: commit it");
        }
    }
}
