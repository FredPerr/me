import { useEffect, useState } from "react";
import { commands, type ProjectView, type RepositoryView } from "@/bindings";

export type RepositoryRemote = {
	repository: RepositoryView;
	remoteUrl: string;
};

async function resolveRemote(repository: RepositoryView): Promise<RepositoryRemote | null> {
	try {
		const remoteUrl = await commands.getGitRemoteUrl(repository.path);
		return remoteUrl ? { repository, remoteUrl } : null;
	} catch {
		return null;
	}
}

export function useProjectRemotes(project: ProjectView) {
	const [remotes, setRemotes] = useState<RepositoryRemote[]>([]);
	const repositories = project.effectiveRepositories;

	useEffect(() => {
		let cancelled = false;

		async function loadRemotes() {
			const results = await Promise.all(repositories.map(resolveRemote));
			if (!cancelled) {
				setRemotes(results.filter((entry): entry is RepositoryRemote => !!entry));
			}
		}

		loadRemotes();
		return () => {
			cancelled = true;
		};
	}, [repositories]);

	return { remotes };
}
