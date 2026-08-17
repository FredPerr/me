import { appDataDir } from "@tauri-apps/api/path";
import { exists, mkdir, readDir, readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import type { Project } from "@/models/Project";

const PROJECTS_DIR = "projects";

export const ProjectDirectory = {
	async loadAllProjects(): Promise<Project[]> {
		const directory = await getProjectsDirectory();
		const directoryExists = await exists(directory);

		if (!directoryExists) {
			return [];
		}

		const entries = await readDir(directory);
		const projects: Project[] = [];

		for (const entry of entries) {
			if (entry.name?.endsWith(".json")) {
				const filePath = `${directory}/${entry.name}`;
				const content = await readTextFile(filePath);
				projects.push(JSON.parse(content) as Project);
			}
		}

		return projects;
	},

	async saveProject(project: Project): Promise<void> {
		const directory = await ensureProjectsDirectory();
		const filePath = `${directory}/${project.tag}.json`;
		const content = JSON.stringify(project, null, 2);

		await writeTextFile(filePath, content);
	},
};

async function getProjectsDirectory(): Promise<string> {
	const appData = await appDataDir();
	return `${appData}${PROJECTS_DIR}`;
}

async function ensureProjectsDirectory(): Promise<string> {
	const directory = await getProjectsDirectory();
	const directoryExists = await exists(directory);

	if (!directoryExists) {
		await mkdir(directory, { recursive: true });
	}

	return directory;
}
