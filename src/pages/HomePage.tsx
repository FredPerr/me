import { Alert, Stack, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { ProjectTabs } from "@/components/project-tabs/ProjectTabs";
import { UnreadableProjectsAlert } from "@/components/shared/UnreadableProjectsAlert";
import { useProjects } from "@/hooks/workspace/useProjects";
import { describeError } from "@/infra/commandError";

export function HomePage() {
	const { t } = useTranslation();
	const { projects, unreadableProjects, loading, error } = useProjects();

	if (loading) {
		return null;
	}

	return (
		<Stack gap="sm" style={{ flex: 1, minHeight: 0 }}>
			{error && (
				<Alert color="red" title={t("workspace.loadFailed")}>
					{describeError(error)}
				</Alert>
			)}
			<UnreadableProjectsAlert unreadableProjects={unreadableProjects} />
			{projects.length === 0 ? (
				<Text c="dimmed">{t("project.noProjects")}</Text>
			) : (
				<ProjectTabs projects={projects} />
			)}
		</Stack>
	);
}
