import { join } from "@tauri-apps/api/path";
import { type DirEntry, exists, readDir } from "@tauri-apps/plugin-fs";
import {
	matchesQuery,
	type SlashItem,
	type SlashProvider,
} from "@/models/agent-chat/slash/SlashProvider";

const PROVIDER_ID = "file";
const MAX_RESULTS = 20;
const MAX_DEPTH = 3;

/** Directories never worth offering as file references. */
const IGNORED_DIRECTORIES = new Set([
	".git",
	"node_modules",
	"target",
	"dist",
	"build",
	".worktrees",
]);

/**
 * Walks the tree under `basePath` breadth-first up to a small depth, collecting
 * relative file paths. Heavy or generated directories are skipped. The walk is
 * bounded by `limit` candidate files so a large repo never blocks the menu.
 */
async function collectFiles(basePath: string, limit: number): Promise<string[]> {
	const results: string[] = [];
	let frontier: Array<{ absolute: string; relative: string; depth: number }> = [
		{ absolute: basePath, relative: "", depth: 0 },
	];

	while (frontier.length > 0 && results.length < limit) {
		const next: typeof frontier = [];
		for (const directory of frontier) {
			if (results.length >= limit) break;
			let entries: DirEntry[];
			try {
				entries = await readDir(directory.absolute);
			} catch {
				continue;
			}
			for (const entry of entries) {
				const relativePath = directory.relative
					? `${directory.relative}/${entry.name}`
					: entry.name;
				if (entry.isDirectory) {
					if (IGNORED_DIRECTORIES.has(entry.name) || entry.name.startsWith(".")) continue;
					if (directory.depth + 1 < MAX_DEPTH) {
						next.push({
							absolute: await join(directory.absolute, entry.name),
							relative: relativePath,
							depth: directory.depth + 1,
						});
					}
				} else if (entry.isFile) {
					results.push(relativePath);
					if (results.length >= limit) break;
				}
			}
		}
		frontier = next;
	}

	return results;
}

/**
 * Slash provider for workspace files under the session's working directory.
 * Selecting a file inserts a `#file:<relativePath>` reference. The candidate
 * walk is bounded in depth and count to stay responsive on large repos.
 */
export function createFileProvider(basePath: string): SlashProvider {
	return {
		id: PROVIDER_ID,
		title: "Files",
		async query(query: string): Promise<SlashItem[]> {
			if (!(await exists(basePath))) return [];

			// Over-collect, then filter by query and cap, so a query can reach
			// files beyond the first MAX_RESULTS discovered.
			const candidates = await collectFiles(basePath, MAX_RESULTS * 10);
			return candidates
				.filter((relativePath) => matchesQuery(relativePath, query))
				.slice(0, MAX_RESULTS)
				.map((relativePath) => ({
					id: `${PROVIDER_ID}:${relativePath}`,
					label: relativePath.split("/").pop() ?? relativePath,
					description: relativePath,
					insertText: `#file:${relativePath}`,
					providerId: PROVIDER_ID,
				}));
		},
	};
}
