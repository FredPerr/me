import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { type Project, Repository } from "@/models/Project";

export type RepositoryRemote = {
	repository: Repository;
	remoteUrl: string;
};

export function useProjectRemotes(project: Project) {
	const [remotes, setRemotes] = useState<RepositoryRemote[]>([]);

	useEffect(() => {
		let cancelled = false;

		async function resolveRemote(repository: Repository): Promise<RepositoryRemote | null> {
			try {
				const resolvedPath = await repository.resolveAbsolutePath(project.path);
				const remoteUrl = await invoke<string | null>("get_git_remote_url", {
					path: resolvedPath,
				});
				return remoteUrl ? { repository, remoteUrl } : null;
			} catch {
				return null;
			}
		}

		async function resolveProjectRootRemote(): Promise<RepositoryRemote | null> {
			try {
				const remoteUrl = await invoke<string | null>("get_git_remote_url", {
					path: project.path,
				});
				if (!remoteUrl) return null;
				const repository = new Repository(project.tag, project.name, ".");
				return { repository, remoteUrl };
			} catch {
				return null;
			}
		}

		async function loadRemotes() {
			// Fall back to the project root when a mono-repo project has no
			// explicit repository entries configured.
			const results =
				project.repositories.length > 0
					? await Promise.all(project.repositories.map(resolveRemote))
					: [await resolveProjectRootRemote()];

			if (!cancelled) {
				setRemotes(results.filter((entry): entry is RepositoryRemote => !!entry));
			}
		}

		loadRemotes();

		return () => {
			cancelled = true;
		};
	}, [project]);

	return { remotes };
}
