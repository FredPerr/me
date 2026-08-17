import { appDataDir, join } from "@tauri-apps/api/path";
import {
	BaseDirectory,
	exists,
	mkdir,
	readDir,
	readTextFile,
	remove,
	writeTextFile,
} from "@tauri-apps/plugin-fs";
import type { Project } from "@/models/Project";

const PROJECTS_DIR = "projects";

export const ProjectDirectory = {
	async loadAllProjects(): Promise<Project[]> {
		const directoryExists = await exists(PROJECTS_DIR, { baseDir: BaseDirectory.AppData });

		if (!directoryExists) {
			return [];
		}

		const entries = await readDir(PROJECTS_DIR, { baseDir: BaseDirectory.AppData });
		const projects: Project[] = [];

		for (const entry of entries) {
			if (entry.name?.endsWith(".json")) {
				const filePath = `${PROJECTS_DIR}/${entry.name}`;
				const content = await readTextFile(filePath, { baseDir: BaseDirectory.AppData });
				projects.push(JSON.parse(content) as Project);
			}
		}

		return projects;
	},

	async saveProject(project: Project): Promise<void> {
		await ensureProjectsDirectory();
		const filePath = `${PROJECTS_DIR}/${project.tag}.json`;
		const content = JSON.stringify(project, null, 2);

		await writeTextFile(filePath, content, { baseDir: BaseDirectory.AppData });
	},

	async getConfigFilePath(projectTag: string): Promise<string> {
		const appData = await appDataDir();
		return await join(appData, PROJECTS_DIR, `${projectTag}.json`);
	},

	async deleteProject(projectTag: string): Promise<void> {
		const filePath = `${PROJECTS_DIR}/${projectTag}.json`;
		const fileExists = await exists(filePath, { baseDir: BaseDirectory.AppData });

		if (fileExists) {
			await remove(filePath, { baseDir: BaseDirectory.AppData });
		}
	},
};

async function ensureProjectsDirectory(): Promise<void> {
	const directoryExists = await exists(PROJECTS_DIR, { baseDir: BaseDirectory.AppData });

	if (!directoryExists) {
		await mkdir(PROJECTS_DIR, { baseDir: BaseDirectory.AppData, recursive: true });
	}
}
