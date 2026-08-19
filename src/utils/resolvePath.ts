import { homeDir } from "@tauri-apps/api/path";

type ResolvePathOptions = {
	basePath?: string
	isFolder?: boolean
}

export async function resolvePath(path: string, {isFolder, basePath}: ResolvePathOptions): Promise<string> {
	if (path.startsWith("~/")) {
		const home = await homeDir();
		const normalizedHome = home.endsWith("/") ? home : `${home}/`;
		return `${normalizedHome}${path.slice(2)}`;
	}

	if (path === "~") {
		return await homeDir();
	}

	if (path.startsWith("./") && basePath) {
		return `${basePath}/${path.slice(2)}`;
	}

	if (path === "." && basePath) {
		return basePath;
	}

	if (!path.startsWith("/") && basePath) {
		return `${basePath}/${path}`;
	}

	const finalPath = isFolder && !path.endsWith('/') ? `${path}/`: path
	return finalPath;
}
