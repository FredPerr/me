import { invoke } from "@tauri-apps/api/core";

/**
 * Read the pull request template for the repository checked out at
 * `worktreePath`, or `undefined` when the repository defines none.
 *
 * The lookup runs in the Rust backend (`read_pull_request_template`) rather
 * than the frontend fs plugin: worktrees live in arbitrary project directories
 * that are outside the plugin's allowed path scope, so a scoped `exists` call
 * would be rejected as a forbidden path instead of reporting a missing file.
 * GitHub resolves the template in the repo root, `.github/`, or `docs/` (both
 * the single-file and multi-template-directory forms); the backend mirrors that.
 */
export async function readPullRequestTemplate(worktreePath: string): Promise<string | undefined> {
	const template = await invoke<string | null>("read_pull_request_template", {
		path: worktreePath,
	});
	return template ?? undefined;
}
