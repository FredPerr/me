import {
	Anchor,
	Button,
	Group,
	Input,
	Modal,
	ScrollArea,
	Select,
	Stack,
	Text,
	Textarea,
	TextInput,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { LinkedTask } from "@/application/work-tracking/listLinkedProjectTasks";
import { workItemRefOf } from "@/application/work-tracking/selectPendingTasks";
import type { BulkContextSpec } from "@/hooks/useContexts";
import { buildTaskPreprompt, DEFAULT_TASKS_PREAMBLE } from "@/models/bulk/tasksToContextSpecs";
import type { Context, Project } from "@/models/Project";

type CreateFromTaskModalProps = {
	task: LinkedTask;
	project: Project;
	contexts: readonly Context[];
	onClose: () => void;
	onCreate: (spec: BulkContextSpec, baseContext: Context) => Promise<void>;
};

export function CreateFromTaskModal({
	task,
	project,
	contexts,
	onClose,
	onCreate,
}: CreateFromTaskModalProps) {
	const { t } = useTranslation();
	const defaultBaseContext = contexts.find((context) => context.isDefault) ?? contexts[0];
	const [selectedBaseContextId, setSelectedBaseContextId] = useState<string | null>(null);
	const [contextName, setContextName] = useState(task.item.title.trim());
	const [prompt, setPrompt] = useState(() => buildTaskPreprompt(DEFAULT_TASKS_PREAMBLE, task.item));
	const [creating, setCreating] = useState(false);

	const baseContextId = selectedBaseContextId ?? defaultBaseContext?.id ?? null;
	const baseContext = contexts.find((context) => context.id === baseContextId);
	const trimmedName = contextName.trim();
	const branchName = trimmedName
		? project.branchNaming.composeBranchName(trimmedName, project.branchNaming.defaultPrefix)
		: "";
	const description = task.item.description?.trim();
	const canCreate = Boolean(baseContext && trimmedName && branchName) && !creating;

	function handleClose() {
		if (!creating) onClose();
	}

	async function handleCreate() {
		if (!baseContext || !canCreate) return;
		setCreating(true);
		try {
			await onCreate(
				{
					contextName: trimmedName,
					branchName,
					preprompt: prompt.trim() || undefined,
					workItemRef: workItemRefOf(task),
				},
				baseContext,
			);
			notifications.show({
				title: t("contexts.created"),
				message: t("workTracking.createFromTask.createdMessage", { name: trimmedName }),
				color: "green",
			});
			onClose();
		} catch (error) {
			notifications.show({
				title: t("contexts.createFailed"),
				message: String(error),
				color: "red",
			});
		} finally {
			setCreating(false);
		}
	}

	return (
		<Modal
			opened
			onClose={handleClose}
			title={t("workTracking.createFromTask.title")}
			size="lg"
			closeOnClickOutside={!creating}
		>
			<Stack gap="md">
				<Stack gap={4}>
					<Text fw={600}>{task.item.title}</Text>
					<Anchor component="button" type="button" size="xs" onClick={() => openUrl(task.item.url)}>
						<Group gap={4} wrap="nowrap">
							<ArrowSquareOutIcon size={12} />
							{t("workTracking.createFromTask.openTask")}
						</Group>
					</Anchor>
				</Stack>
				<Stack gap={4}>
					<Input.Label>{t("workTracking.createFromTask.descriptionLabel")}</Input.Label>
					<ScrollArea.Autosize mah={200} type="auto">
						<Text
							size="sm"
							c={description ? undefined : "dimmed"}
							style={{ whiteSpace: "pre-wrap" }}
						>
							{description || t("workTracking.createFromTask.noDescription")}
						</Text>
					</ScrollArea.Autosize>
				</Stack>
				<Select
					label={t("workTracking.createFromTask.baseContextLabel")}
					description={t("workTracking.createFromTask.baseContextHint")}
					data={contexts.map((context) => ({
						value: context.id,
						label: context.isDefault ? t("common.default") : context.name,
					}))}
					value={baseContextId}
					onChange={setSelectedBaseContextId}
					allowDeselect={false}
					disabled={creating}
					searchable
				/>
				<TextInput
					label={t("workTracking.createFromTask.contextNameLabel")}
					description={
						branchName
							? t("workTracking.createFromTask.branchPreview", { branch: branchName })
							: undefined
					}
					error={
						trimmedName && !branchName ? t("workTracking.createFromTask.invalidBranch") : undefined
					}
					value={contextName}
					onChange={(event) => setContextName(event.currentTarget.value)}
					disabled={creating}
					required
				/>
				<Textarea
					label={t("workTracking.createFromTask.promptLabel")}
					description={t("workTracking.createFromTask.promptHint")}
					value={prompt}
					onChange={(event) => setPrompt(event.currentTarget.value)}
					autosize
					minRows={4}
					maxRows={10}
					disabled={creating}
				/>
				<Group justify="flex-end">
					<Button variant="subtle" onClick={handleClose} disabled={creating}>
						{t("common.cancel")}
					</Button>
					<Button onClick={handleCreate} loading={creating} disabled={!canCreate}>
						{t("common.create")}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}
