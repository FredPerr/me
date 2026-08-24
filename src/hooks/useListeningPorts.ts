import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { Project } from "@/models/Project";

type RawListeningPort = {
	port: number;
	pid: number;
	process_name: string;
	cwd: string;
};

export type MatchedPort = {
	port: number;
	pid: number;
	processName: string;
	cwd: string;
	contextName: string | null;
	repositoryName: string | null;
};

export function useListeningPorts(
	project: Project,
	ignoredPorts: number[] = [],
	pollingInterval = 5000,
) {
	const [ports, setPorts] = useState<MatchedPort[]>([]);

	const refresh = useCallback(async () => {
		try {
			const raw = await invoke<RawListeningPort[]>("list_listening_ports");
			const filtered = raw.filter((r) => !ignoredPorts.includes(r.port));
			const matched = matchPortsToProject(filtered, project);
			setPorts(matched);
		} catch {
			setPorts([]);
		}
	}, [project, ignoredPorts]);

	useEffect(() => {
		refresh();
		const interval = setInterval(refresh, pollingInterval);
		return () => clearInterval(interval);
	}, [refresh, pollingInterval]);

	return { ports, refresh };
}

function matchPortsToProject(raw: RawListeningPort[], project: Project): MatchedPort[] {
	const projectPath = normalizePath(project.path);
	const results: MatchedPort[] = [];

	for (const entry of raw) {
		const cwd = normalizePath(entry.cwd);

		if (!cwd.startsWith(projectPath)) continue;

		const contextName = resolveContextName(cwd, projectPath);
		const repositoryName = resolveRepositoryName(cwd, projectPath, project);

		results.push({
			port: entry.port,
			pid: entry.pid,
			processName: entry.process_name,
			cwd: entry.cwd,
			contextName,
			repositoryName,
		});
	}

	return deduplicateByPort(results);
}

function resolveContextName(cwd: string, projectPath: string): string | null {
	const worktreesPrefix = `${projectPath}/.worktrees/`;
	if (!cwd.startsWith(worktreesPrefix)) return null;

	const afterWorktrees = cwd.slice(worktreesPrefix.length);
	const slashIndex = afterWorktrees.indexOf("/");
	return slashIndex > 0 ? afterWorktrees.slice(0, slashIndex) : afterWorktrees;
}

function resolveRepositoryName(cwd: string, projectPath: string, project: Project): string | null {
	const worktreesPrefix = `${projectPath}/.worktrees/`;
	if (cwd.startsWith(worktreesPrefix)) {
		const afterWorktrees = cwd.slice(worktreesPrefix.length);
		const parts = afterWorktrees.split("/");
		if (parts.length >= 2) {
			return parts[1];
		}
	}

	for (const repo of project.repositories) {
		const repoRel = repo.relPath.startsWith("./") ? repo.relPath.slice(2) : repo.relPath;
		const repoPath = `${projectPath}/${repoRel}`;
		if (cwd.startsWith(repoPath)) {
			return repo.name;
		}
	}

	return null;
}

function normalizePath(path: string): string {
	return path.endsWith("/") ? path.slice(0, -1) : path;
}

function deduplicateByPort(ports: MatchedPort[]): MatchedPort[] {
	const seen = new Map<number, MatchedPort>();
	for (const port of ports) {
		if (!seen.has(port.port)) {
			seen.set(port.port, port);
		}
	}
	return Array.from(seen.values());
}
