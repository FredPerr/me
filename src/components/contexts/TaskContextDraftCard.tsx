import { Anchor, Card, Group, Stack, Text, Textarea, TextInput } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import type { ProjectView } from "@/bindings";
import { useBranchNameSuggestion } from "@/hooks/workspace/useBranchNameSuggestion";
import type { TaskContextDraft, ValidatedTaskContextDraft } from "@/models/bulk/taskContextDrafts";

type TaskContextDraftCardProps = {
	project: ProjectView;
	validatedDraft: ValidatedTaskContextDraft;
	disabled: boolean;
	onChange: (draft: TaskContextDraft) => void;
};

export function TaskContextDraftCard({
	project,
	validatedDraft,
	disabled,
	onChange,
}: TaskContextDraftCardProps) {
	const { t } = useTranslation();
	const { draft, issue } = validatedDraft;
	const branchName = useBranchNameSuggestion(
		project.tag,
		draft.contextName,
		project.branchNaming.defaultPrefix,
	);

	return (
		<Card withBorder padding="sm" radius="sm">
			<Stack gap="sm">
				<Group justify="space-between" wrap="nowrap" gap="sm">
					<Text fw={600} size="sm" truncate>
						{draft.task.task.title}
					</Text>
					<Anchor
						component="button"
						type="button"
						size="xs"
						onClick={() => openUrl(draft.task.task.url)}
						style={{ flexShrink: 0 }}
					>
						<Group gap={4} wrap="nowrap">
							<ArrowSquareOutIcon size={12} />
							{t("workTracking.createFromTask.openTask")}
						</Group>
					</Anchor>
				</Group>
				<TextInput
					label={t("workTracking.createFromTask.contextNameLabel")}
					description={
						branchName
							? t("workTracking.createFromTask.branchPreview", { branch: branchName })
							: undefined
					}
					error={issue ? t(`contexts.fromTasks.issues.${issue}`) : undefined}
					value={draft.contextName}
					onChange={(event) => onChange({ ...draft, contextName: event.currentTarget.value })}
					disabled={disabled}
					required
				/>
				<Textarea
					label={t("workTracking.createFromTask.promptLabel")}
					description={t("workTracking.createFromTask.promptHint")}
					value={draft.prompt}
					onChange={(event) => onChange({ ...draft, prompt: event.currentTarget.value })}
					autosize
					minRows={3}
					maxRows={8}
					disabled={disabled}
				/>
			</Stack>
		</Card>
	);
}
