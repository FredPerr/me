import { invoke } from "@tauri-apps/api/core";

export async function openInIde(ideCommand: string, projectPath: string): Promise<void> {
	await invoke("open_in_ide", { command: ideCommand, path: projectPath });
}
