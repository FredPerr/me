import { ProjectTabs } from "@/components/project-tabs/ProjectTabs";
import type { Project } from "@/models/Project";

const PROJECTS: Project[] = [
	{
		name: "Targipsum",
		tag: "targi",
		path: "",
		subprojects: [],
	},
	{
		name: "dotfiles",
		tag: "dotfiles",
		path: "",
		subprojects: [],
	},
];

export function HomePage() {
	return <ProjectTabs projects={PROJECTS} />;
}
