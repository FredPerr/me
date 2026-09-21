import { appDataDir, join } from "@tauri-apps/api/path";
import { exists, readDir, remove } from "@tauri-apps/plugin-fs";
import { load } from "@tauri-apps/plugin-store";
import { Project, type ProjectData } from "@/models/Project";

const PROJECTS_DIR = "projects";
const PROJECT_KEY = "project";
const JSON_EXTENSION = ".json";

function storeFile(projectTag: string): string {
	return `${PROJECTS_DIR}/${projectTag}${JSON_EXTENSION}`;
}

async function openProjectStore(projectTag: string) {
	return load(storeFile(projectTag), { autoSave: true });
}

export const ProjectDirectory = {
	async loadAllProjects(): Promise<Project[]> {
		const appData = await appDataDir();
		const projectsPath = await join(appData, PROJECTS_DIR);

		if (!(await exists(projectsPath))) {
			return [];
		}

		const entries = await readDir(projectsPath);
		const projects: Project[] = [];

		for (const entry of entries) {
			if (!entry.name?.endsWith(JSON_EXTENSION)) continue;

			const projectTag = entry.name.slice(0, -JSON_EXTENSION.length);
			const store = await openProjectStore(projectTag);
			const data = await store.get<ProjectData>(PROJECT_KEY);
			if (data) {
				projects.push(Project.fromJSON(data));
			}
		}

		return projects;
	},

	async saveProject(project: Project): Promise<void> {
		const store = await openProjectStore(project.tag);
		await store.set(PROJECT_KEY, project.toJSON());
		await store.save();
	},

	async getConfigFilePath(projectTag: string): Promise<string> {
		const appData = await appDataDir();
		return await join(appData, PROJECTS_DIR, `${projectTag}${JSON_EXTENSION}`);
	},

	async deleteProject(projectTag: string): Promise<void> {
		const filePath = await this.getConfigFilePath(projectTag);
		if (await exists(filePath)) {
			await remove(filePath);
		}
	},
};
