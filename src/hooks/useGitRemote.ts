import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export function useGitRemote(path: string | undefined) {
	const [remoteUrl, setRemoteUrl] = useState<string | null>(null);

	useEffect(() => {
		if (!path) {
			setRemoteUrl(null);
			return;
		}

		invoke<string | null>("get_git_remote_url", { path }).then(setRemoteUrl);
	}, [path]);

	return { remoteUrl };
}
