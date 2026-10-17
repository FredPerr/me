use std::sync::Arc;

use tauri::{Manager, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

use workspace::application::agent_runs::AgentRunService;
use workspace::application::quit_guard::{decide_close, CloseDecision};

mod ai_assist;
mod ai_session;
mod binary_resolver;
mod composition;
mod git;
mod git_hosting;
mod ide_launcher;
mod pull_request_template;
mod secret_store;
mod settings;
mod shortcuts;
mod storage;
mod work_tracking;
mod workspace;
mod workspace_detector;

/// The typed commands, exported with their types to `src/bindings.ts`.
fn typed_commands() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .commands(tauri_specta::collect_commands![
            ide_launcher::open_in_ide,
            workspace_detector::get_active_workspaces,
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
            git::commands::pull_worktree,
            git::commands::push_worktree,
            git::commands::fetch_pr_branch,
            workspace::commands::workspace_list_projects,
            workspace::commands::workspace_get_project,
            workspace::commands::workspace_create_context,
            workspace::commands::workspace_delete_context,
            workspace::commands::workspace_set_context_status,
            workspace::commands::workspace_set_context_static,
            workspace::commands::workspace_save_pull_request_draft,
            workspace::commands::workspace_set_agent_prompt,
            workspace::commands::workspace_record_agent_conversation,
            workspace::commands::workspace_run_agent,
            workspace::commands::workspace_stop_agent_run,
            workspace::commands::workspace_list_agent_runs,
            workspace::commands::workspace_update_project_settings,
            workspace::commands::workspace_add_project,
            workspace::commands::workspace_delete_project,
            workspace::commands::workspace_suggest_branch_name,
            workspace::commands::workbench_get,
            workspace::commands::workbench_set_placement,
            workspace::commands::workbench_reorder,
            workspace::commands::workbench_set_last_project,
            git_hosting::commands::git_hosting_list_accounts,
            git_hosting::commands::git_hosting_start_sign_in,
            git_hosting::commands::git_hosting_poll_sign_in,
            git_hosting::commands::git_hosting_sign_in_with_token,
            git_hosting::commands::git_hosting_sign_out,
            git_hosting::commands::git_hosting_project_pull_requests,
            git_hosting::commands::git_hosting_context_pull_requests,
            git_hosting::commands::git_hosting_create_pull_request,
            settings::commands::settings_get,
            settings::commands::settings_set_assign_me_to_tasks,
            settings::commands::settings_set_default_ai_tool,
            ai_assist::commands::ai_assist_suggest_contexts,
            ai_assist::commands::ai_assist_cancel_contexts,
            ai_assist::commands::ai_assist_suggest_pull_request,
            ai_assist::commands::ai_assist_cancel_pull_request,
            work_tracking::commands::work_tracking_list_connections,
            work_tracking::commands::work_tracking_save_connection,
            work_tracking::commands::work_tracking_remove_connection,
            work_tracking::commands::work_tracking_list_projects,
            work_tracking::commands::work_tracking_list_groups,
            work_tracking::commands::work_tracking_list_items,
            work_tracking::commands::work_tracking_list_project_items,
            work_tracking::commands::work_tracking_assign_me_to_task,
            work_tracking::commands::work_tracking_assign_me_to_tasks,
            work_tracking::commands::work_tracking_pending_tasks,
        ])
        .events(tauri_specta::collect_events![
            workspace::infra::tauri_project_events::ProjectChanged,
            workspace::infra::tauri_agent_run_events::AgentRunChanged,
            workspace::infra::tauri_workbench_events::WorkbenchChanged,
            git_hosting::infra::tauri_git_hosting_events::GitHostingChanged
        ])
}

/// Wires the quit guard to the window's close request. The decision (whether
/// to warn and what the dialog says) and the stopping of runs are tested
/// functions on `AgentRunService`; this shell only drives the window and the
/// native dialog, which can't be unit-tested.
///
/// On a close request: with no run in progress the close goes ahead. With runs
/// in progress the close is prevented and a native dialog lists them with
/// "Stop runs and quit" and "Cancel". Cancelling leaves the window open;
/// confirming stops every run (recording each Stopped and moving its context to
/// needs human) then exits. The exit is deferred to the dialog's async callback
/// because the close was already prevented; re-emitting the close after
/// stopping would re-run this guard, so we exit the app directly instead.
fn guard_window_close(window: &tauri::WebviewWindow, agent_run_service: Arc<AgentRunService>) {
    let window_handle = window.clone();
    window.on_window_event(move |event| {
        let WindowEvent::CloseRequested { api, .. } = event else {
            return;
        };
        let CloseDecision::Confirm { message } =
            decide_close(&agent_run_service.in_progress_runs())
        else {
            return;
        };
        api.prevent_close();
        let app = window_handle.app_handle().clone();
        let service = Arc::clone(&agent_run_service);
        window_handle
            .dialog()
            .message(message)
            .title("Agent runs in progress")
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Stop runs and quit".to_string(),
                "Cancel".to_string(),
            ))
            .show(move |stop_and_quit| {
                if !stop_and_quit {
                    return;
                }
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = service.stop_all_in_progress().await {
                        eprintln!("[workspace] runs not stopped on quit: {error}");
                    }
                    app.exit(0);
                });
            });
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let typed_builder = typed_commands();
    let typed_handler = typed_builder.invoke_handler();
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
        .invoke_handler(typed_handler)
        .setup(move |app| {
            typed_builder.mount_events(app);
            app.manage(ai_session::registry::SessionRegistry::new());
            let app_data_dir = app.path().app_data_dir()?;
            let database = storage::open(&app_data_dir)?;
            let settings = settings::build(Arc::clone(&database))?;
            let git = git::build(git_hosting::remote_credentials());
            // Workspace is built before work_tracking: the pending-tasks use
            // case reads a Project's remote project links through a composition
            // adapter over WorkspaceService. The agent runner is a composition
            // adapter over ai_session; SessionRegistry is already managed above.
            let agent_runner = Arc::new(
                composition::ai_session_agent_runner::AiSessionAgentRunner::new(
                    app.handle().clone(),
                ),
            );
            // A fresh context's first run uses the Default AI tool (#90), read
            // from settings through a composition adapter so workspace never
            // names settings; the context then keeps the tool it recorded.
            let default_agent_tool =
                composition::settings_default_agent_tool::SettingsDefaultAgentTool::new(
                    Arc::clone(&settings.service),
                );
            let workspace = workspace::build(
                app.handle().clone(),
                Arc::clone(&database),
                git.repositories(),
                agent_runner,
                Arc::new(default_agent_tool),
            )?;
            let project_task_links =
                composition::workspace_project_task_links::WorkspaceProjectTaskLinks::new(
                    Arc::clone(&workspace.service),
                );
            let work_tracking = work_tracking::build_service(
                Arc::clone(&database),
                &app_data_dir,
                Arc::new(project_task_links),
            )?;
            app.manage(work_tracking);
            let project_remotes =
                composition::workspace_project_remotes::WorkspaceProjectRemotes::new(
                    Arc::clone(&workspace.service),
                    git.repositories(),
                );
            let worktree_pushes =
                composition::git_worktree_pushes::GitWorktreePushes::new(git.repositories());
            let git_hosting = git_hosting::build(
                app.handle().clone(),
                database,
                Arc::new(project_remotes),
                Arc::new(worktree_pushes),
            )?;
            let push_listener = git::PushListener::new(Arc::new(
                composition::git_hosting_push_listener::GitHostingPushListener::new(Arc::clone(
                    &git_hosting.service,
                )),
            ));
            // Reconcile runs the previous process left mid-run (#82): at
            // startup, before any run is started, every run persisted as
            // Running is a crash leftover, recorded Interrupted with its
            // context moved to needs human.
            let agent_run_service = Arc::clone(&workspace.agent_run_service);
            if let Err(error) = agent_run_service.reconcile_interrupted_runs() {
                eprintln!("[workspace] interrupted runs not reconciled: {error}");
            }
            // AI assists (#88): the suggestion use case over the read-only
            // runner (a composition adapter over ai_session), with the default
            // tool read from settings, branch naming read from the workspace
            // Project policy, and a throwaway temp folder per run. All three
            // ports are composition adapters so ai_assist names no other
            // context.
            let ai_assist_runner = Arc::new(
                composition::ai_session_ai_assist_runner::AiSessionAiAssistRunner::new(
                    app.handle().clone(),
                ),
            );
            let default_ai_tool = composition::settings_default_ai_tool::SettingsDefaultAiTool::new(
                Arc::clone(&settings.service),
            );
            let project_branch_naming =
                composition::workspace_project_branch_naming::WorkspaceProjectBranchNaming::new(
                    Arc::clone(&workspace.service),
                );
            // #89: the pull-request suggestion reads its target (names, worktree,
            // base branch) from the workspace Project and its diff + template
            // from git — both through composition ports so ai_assist names no
            // other context.
            let pull_request_context =
                composition::workspace_pull_request_context::WorkspacePullRequestContext::new(
                    Arc::clone(&workspace.service),
                );
            let repository_diff =
                composition::git_repository_diff::GitRepositoryDiff::new(git.repositories());
            let ai_assist = ai_assist::AiAssist {
                service: Arc::new(
                    ai_assist::application::suggest_contexts::AiAssistService::new(
                        ai_assist_runner,
                        Arc::new(default_ai_tool),
                        Arc::new(project_branch_naming),
                        Arc::new(composition::tempfile_temp_workspace::TempfileTempWorkspace),
                        Arc::new(pull_request_context),
                        Arc::new(repository_diff),
                    ),
                ),
            };
            app.manage(git);
            app.manage(push_listener);
            app.manage(git_hosting);
            app.manage(workspace);
            app.manage(settings);
            app.manage(ai_assist);
            shortcuts::register(app.handle())?;
            // Guard the quit (#82): closing with runs in progress asks first
            // and stops them cleanly. The decision and the stop live in tested
            // functions; this wiring is a thin shell over them.
            if let Some(window) = app.get_webview_window("main") {
                guard_window_close(&window, agent_run_service);
            }
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
