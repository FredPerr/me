import { join } from "@tauri-apps/api/path";
import { exists, readDir } from "@tauri-apps/plugin-fs";
import {
	matchesQuery,
	type SlashItem,
	type SlashProvider,
} from "@/models/agent-chat/slash/SlashProvider";

const PROVIDER_ID = "steering";
const STEERING_RELATIVE_PATH = ".kiro/steering";
const MAX_RESULTS = 20;

/**
 * Slash provider for Kiro steering files under `.kiro/steering`. Selecting one
 * inserts a `#steering:<name>` reference the agent can resolve. Returns no items
 * when the directory is absent, so projects without steering simply show
 * nothing from this provider.
 */
export function createSteeringProvider(basePath: string): SlashProvider {
	return {
		id: PROVIDER_ID,
		title: "Steering",
		async query(query: string): Promise<SlashItem[]> {
			const steeringPath = await join(basePath, STEERING_RELATIVE_PATH);
			if (!(await exists(steeringPath))) return [];

			const entries = await readDir(steeringPath);
			return entries
				.filter((entry) => entry.isFile && entry.name.endsWith(".md"))
				.map((entry) => entry.name.replace(/\.md$/, ""))
				.filter((name) => matchesQuery(name, query))
				.slice(0, MAX_RESULTS)
				.map((name) => ({
					id: `${PROVIDER_ID}:${name}`,
					label: name,
					description: `${STEERING_RELATIVE_PATH}/${name}.md`,
					insertText: `#steering:${name}`,
					providerId: PROVIDER_ID,
				}));
		},
	};
}
