import { Alert, List, Text } from "@mantine/core";
import { WarningIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { UnreadableProjectView } from "@/bindings";

type UnreadableProjectsAlertProps = {
	unreadableProjects: readonly UnreadableProjectView[];
};

export function UnreadableProjectsAlert({ unreadableProjects }: UnreadableProjectsAlertProps) {
	const { t } = useTranslation();
	const entries = unreadableProjects.map((project) => ({
		name: project.tag,
		reason: project.reason,
	}));
	if (entries.length === 0) return null;

	return (
		<Alert
			color="orange"
			variant="light"
			icon={<WarningIcon size={16} />}
			title={t("workspace.unreadable.title", { count: entries.length })}
		>
			<Text size="sm" mb="xs">
				{t("workspace.unreadable.description")}
			</Text>
			<List size="sm" spacing={4}>
				{entries.map((entry) => (
					<List.Item key={entry.name}>
						<Text span size="sm" fw={600}>
							{entry.name}
						</Text>
						<Text span size="sm" c="dimmed">
							{" — "}
							{entry.reason}
						</Text>
					</List.Item>
				))}
			</List>
		</Alert>
	);
}
