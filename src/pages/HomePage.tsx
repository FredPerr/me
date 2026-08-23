import { Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { ProjectTabs } from "@/components/project-tabs/ProjectTabs";
import { useProjects } from "@/hooks/useProjects";

export function HomePage() {
	const { t } = useTranslation();
	const { projects, loading } = useProjects();

	if (loading) {
		return null;
	}

	if (projects.length === 0) {
		return <Text c="dimmed">{t("project.noProjects")}</Text>;
	}

	return <ProjectTabs projects={projects} />;
}
