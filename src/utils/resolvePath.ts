import { homeDir } from "@tauri-apps/api/path";

export async function resolvePath(path: string): Promise<string> {
	if (path.startsWith("~/")) {
		const home = await homeDir();
		return `${home}${path.slice(2)}`;
	}

	if (path === "~") {
		return await homeDir();
	}

	return path;
}
