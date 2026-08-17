import { Text } from "@mantine/core";
import { ProjectTabs } from "@/components/project-tabs/ProjectTabs";
import { useProjects } from "@/hooks/useProjects";

export function HomePage() {
	const { projects, loading } = useProjects();

	if (loading) {
		return null;
	}

	if (projects.length === 0) {
		return <Text c="dimmed">No projects configured. Add some in Settings.</Text>;
	}

	return <ProjectTabs projects={projects} />;
}
