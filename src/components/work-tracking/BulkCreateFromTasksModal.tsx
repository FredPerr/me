import { Alert, Badge, Button, Group, Modal, Select, Stack, Text, Textarea } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { InfoIcon } from "@phosphor-icons/react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProjectView } from "@/bindings";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import { useBranchNameSuggestion } from "@/hooks/workspace/useBranchNameSuggestion";
import { useCreateContexts } from "@/hooks/workspace/useCreateContexts";
import type { BulkContextSpec } from "@/models/bulk/BulkContextSpec";
import { DEFAULT_TASKS_PREAMBLE, tasksToContextSpecs } from "@/models/bulk/tasksToContextSpecs";
import { findContext } from "@/models/projectView";

type BulkCreateFromTasksModalProps = {
	opened: boolean;
	onClose: () => void;
	project: ProjectView;
	connectionId: string;
	tasks: readonly WorkItem[];
	onCreated: () => void;
};

/**
 * Creates one git-worktree context per selected Teamwork task. The user edits a
 * shared preamble describing what to do with every task; each task's title and
 * description are appended to that preamble and stored as the context's Kiro
 * preprompt. Branch names come from the task titles via the project's naming
 * policy. Nothing is auto-run — prompts are prefilled when a context is opened.
 */
export function BulkCreateFromTasksModal({
	opened,
	onClose,
	project,
	connectionId,
	tasks,
	onCreated,
}: BulkCreateFromTasksModalProps) {
	const { t } = useTranslation();
	const { createContexts, creating } = useCreateContexts(project);
	const [preamble, setPreamble] = useState(DEFAULT_TASKS_PREAMBLE);
	const [baseContextId, setBaseContextId] = useState<string | null>(null);

	// Existing non-default contexts a new batch can branch off of.
	const baseContextOptions = useMemo(
		() => project.contexts.filter((context) => !context.isDefault),
		[project.contexts],
	);

	// Reset to defaults whenever the modal is reopened.
	useEffect(() => {
		if (opened) {
			setPreamble(DEFAULT_TASKS_PREAMBLE);
			setBaseContextId(null);
		}
	}, [opened]);

	const specs = useMemo(
		() => tasksToContextSpecs(tasks, preamble, connectionId),
		[tasks, preamble, connectionId],
	);

	function handleClose() {
		if (creating) return;
		onClose();
	}

	async function handleCreate() {
		if (specs.length === 0) return;
		const baseContext = baseContextId ? findContext(project, baseContextId) : undefined;
		try {
			await createContexts(specs, baseContext);
		} catch {
			return;
		}
		notifications.show({
			title: t("workTracking.bulkCreate.createdTitle"),
			message: t("workTracking.bulkCreate.createdMessage", { count: specs.length }),
			color: "green",
		});
		onCreated();
		onClose();
	}

	return (
		<Modal
			opened={opened}
			onClose={handleClose}
			title={t("workTracking.bulkCreate.title")}
			size="lg"
			closeOnClickOutside={!creating}
		>
			<Stack gap="md">
				<Text size="sm" c="dimmed">
					{t("workTracking.bulkCreate.description", { count: tasks.length })}
				</Text>
				<Textarea
					label={t("workTracking.bulkCreate.preambleLabel")}
					description={t("workTracking.bulkCreate.preambleHint")}
					value={preamble}
					onChange={(event) => setPreamble(event.currentTarget.value)}
					autosize
					minRows={4}
					maxRows={10}
					disabled={creating}
				/>
				{baseContextOptions.length > 0 && (
					<Select
						label={t("workTracking.bulkCreate.baseContextLabel")}
						description={t("workTracking.bulkCreate.baseContextHint")}
						placeholder={t("workTracking.bulkCreate.baseContextDefault")}
						data={baseContextOptions.map((context) => ({
							value: context.id,
							label: context.name,
						}))}
						value={baseContextId}
						onChange={setBaseContextId}
						disabled={creating}
						clearable
						searchable
					/>
				)}
				<Stack gap="xs">
					<Text size="sm" fw={500}>
						{t("workTracking.bulkCreate.previewTitle")}
					</Text>
					{specs.length === 0 ? (
						<Alert icon={<InfoIcon size={16} />} color="yellow" variant="light">
							{t("workTracking.bulkCreate.noValidTasks")}
						</Alert>
					) : (
						<Stack
							component="ul"
							gap={6}
							p={0}
							m={0}
							style={{ listStyle: "none", maxHeight: 240, overflowY: "auto" }}
						>
							{specs.map((spec) => (
								<SpecPreviewRow key={spec.task?.taskId} project={project} spec={spec} />
							))}
						</Stack>
					)}
				</Stack>
				<Group justify="flex-end">
					<Button variant="subtle" onClick={handleClose} disabled={creating}>
						{t("common.cancel")}
					</Button>
					<Button onClick={handleCreate} loading={creating} disabled={specs.length === 0}>
						{t("workTracking.bulkCreate.createButton", { count: specs.length })}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}

type SpecPreviewRowProps = {
	project: ProjectView;
	spec: BulkContextSpec;
};

function SpecPreviewRow({ project, spec }: SpecPreviewRowProps) {
	const branchName = useBranchNameSuggestion(
		project.tag,
		spec.contextName,
		project.branchNaming.defaultPrefix,
	);
	return (
		<Group component="li" justify="space-between" wrap="nowrap" gap="sm">
			<Text size="sm" truncate>
				{spec.contextName}
			</Text>
			{branchName && (
				<Badge variant="light" size="sm" style={{ flexShrink: 0 }}>
					{branchName}
				</Badge>
			)}
		</Group>
	);
}
