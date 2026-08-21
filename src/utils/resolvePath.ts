import { homeDir } from "@tauri-apps/api/path";

type ResolvePathOptions = {
	basePath?: string
	isFolder?: boolean
}

export async function resolvePath(path: string, {isFolder, basePath}: ResolvePathOptions): Promise<string> {
	const normalizedBase = basePath?.endsWith("/") ? basePath.slice(0, -1) : basePath;

	if (path.startsWith("~/")) {
		const home = await homeDir();
		const normalizedHome = home.endsWith("/") ? home.slice(0, -1) : home;
		return `${normalizedHome}/${path.slice(2)}`;
	}

	if (path === "~") {
		return await homeDir();
	}

	if (path.startsWith("./") && normalizedBase) {
		return `${normalizedBase}/${path.slice(2)}`;
	}

	if (path === "." && normalizedBase) {
		return normalizedBase;
	}

	if (!path.startsWith("/") && normalizedBase) {
		return `${normalizedBase}/${path}`;
	}

	const finalPath = isFolder && !path.endsWith('/') ? `${path}/`: path
	return finalPath;
}
