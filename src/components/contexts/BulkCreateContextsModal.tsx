import {
	ActionIcon,
	Badge,
	Box,
	Button,
	Checkbox,
	Group,
	Modal,
	Stack,
	Text,
	Textarea,
	TextInput,
	Tooltip,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { ArrowClockwiseIcon, PlusIcon, SparkleIcon, TrashIcon } from "@phosphor-icons/react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { type BulkDraftError, useBulkContextDraft } from "@/hooks/useBulkContextDraft";
import type { BulkContextSpec } from "@/hooks/useContexts";
import type { BulkContextDraft } from "@/models/bulk/BulkContextDraft";
import type { Project } from "@/models/Project";

type BulkCreateContextsModalProps = {
	opened: boolean;
	onClose: () => void;
	project: Project;
	onCreate: (specs: BulkContextSpec[]) => Promise<void>;
};

type RowOrigin = "generated" | "manual";

type ReviewRow = BulkContextDraft & { rowId: string; origin: RowOrigin; include: boolean };

type EditableRowField = "contextName" | "branchName" | "preprompt";

function toGeneratedRow(draft: BulkContextDraft): ReviewRow {
	return { ...draft, rowId: crypto.randomUUID(), origin: "generated", include: true };
}

function emptyManualRow(): ReviewRow {
	return {
		rowId: crypto.randomUUID(),
		origin: "manual",
		include: true,
		contextName: "",
		branchName: "",
	};
}

function errorMessageKey(error: BulkDraftError): string {
	if (error.kind === "session") return "contexts.bulk.errorSession";
	switch (error.reason) {
		case "no-json-found":
			return "contexts.bulk.errorNoJson";
		case "invalid-json":
			return "contexts.bulk.errorInvalidJson";
		case "not-an-array":
			return "contexts.bulk.errorNotArray";
		default:
			return "contexts.bulk.errorNoRows";
	}
}

/**
 * Bulk context creator. Phase one takes one high-level instruction and runs a
 * headless Kiro expansion into draft contexts. Phase two shows those drafts in
 * an editable table — context name, branch, and an optional Kiro prompt per
 * row, each with an include toggle — and creates the selected ones in a single
 * batch. Prompts are stored (prefilled into Kiro later), never auto-run.
 */
export function BulkCreateContextsModal({
	opened,
	onClose,
	project,
	onCreate,
}: BulkCreateContextsModalProps) {
	const { t } = useTranslation();
	const { status, drafts, error, generate, reset } = useBulkContextDraft(project);
	const [instruction, setInstruction] = useState("");
	const [rows, setRows] = useState<ReviewRow[]>([]);
	const [creating, setCreating] = useState(false);

	// Identity of the last drafts batch merged into rows, so a given generation
	// is applied once (not re-merged on unrelated re-renders).
	const mergedDraftsRef = useRef<BulkContextDraft[] | null>(null);

	useEffect(() => {
		if (status !== "ready") return;
		if (mergedDraftsRef.current === drafts) return;
		mergedDraftsRef.current = drafts;
		// Keep every manual row; swap all generated rows for the fresh batch.
		setRows((current) => [
			...current.filter((row) => row.origin === "manual"),
			...drafts.map(toGeneratedRow),
		]);
	}, [status, drafts]);

	const selectedCount = useMemo(() => rows.filter((row) => row.include).length, [rows]);

	function handleClose() {
		if (creating) return;
		reset();
		mergedDraftsRef.current = null;
		setInstruction("");
		setRows([]);
		onClose();
	}

	async function handleGenerate() {
		const trimmed = instruction.trim();
		if (trimmed.length === 0) return;
		await generate(trimmed);
	}

	/** Toggles a row's include flag without changing its origin. */
	function setRowInclude(rowId: string, include: boolean) {
		setRows((current) => current.map((row) => (row.rowId === rowId ? { ...row, include } : row)));
	}

	/** Edits an editable field; a generated row becomes manual once touched. */
	function editRowField(rowId: string, field: EditableRowField, value: string) {
		setRows((current) =>
			current.map((row) =>
				row.rowId === rowId ? { ...row, [field]: value, origin: "manual" } : row,
			),
		);
	}

	function addManualRow() {
		setRows((current) => [...current, emptyManualRow()]);
	}

	function removeRow(rowId: string) {
		setRows((current) => current.filter((row) => row.rowId !== rowId));
	}

	async function handleCreate() {
		const specs: BulkContextSpec[] = rows
			.filter((row) => row.include && row.contextName.trim() && row.branchName.trim())
			.map((row) => ({
				contextName: row.contextName.trim(),
				branchName: row.branchName.trim(),
				preprompt: row.preprompt?.trim() ? row.preprompt.trim() : undefined,
			}));
		if (specs.length === 0) return;

		setCreating(true);
		try {
			await onCreate(specs);
			notifications.show({
				title: t("contexts.bulk.createdTitle"),
				message: t("contexts.bulk.createdMessage", { count: specs.length }),
				color: "green",
			});
			handleClose();
		} catch (createError) {
			notifications.show({
				title: t("contexts.bulk.createFailed"),
				message: String(createError),
				color: "red",
			});
		} finally {
			setCreating(false);
		}
	}

	const hasRows = rows.length > 0;
	const isReview = hasRows;
	const isGenerating = status === "generating";

	return (
		<Modal
			opened={opened}
			onClose={handleClose}
			title={t("contexts.bulk.title")}
			size={isReview ? "80rem" : "lg"}
			closeOnClickOutside={!creating}
		>
			<Stack gap="md">
				<Stack gap="xs">
					<Textarea
						label={t("contexts.bulk.instructionLabel")}
						description={t("contexts.bulk.instructionHint")}
						placeholder={t("contexts.bulk.instructionPlaceholder")}
						value={instruction}
						onChange={(event) => setInstruction(event.currentTarget.value)}
						autosize
						minRows={isReview ? 3 : 6}
						maxRows={isReview ? 8 : 16}
						disabled={isGenerating || creating}
					/>
					{error && (
						<Text size="xs" c="red.4">
							{t(errorMessageKey(error), {
								message: error.kind === "session" ? error.message : "",
							})}
						</Text>
					)}
					<Group justify="flex-end">
						{!isReview && (
							<>
								<Button variant="subtle" onClick={handleClose} disabled={isGenerating}>
									{t("common.cancel")}
								</Button>
								<Button
									variant="light"
									leftSection={<PlusIcon size={16} />}
									onClick={addManualRow}
									disabled={isGenerating}
								>
									{t("contexts.bulk.addContext")}
								</Button>
							</>
						)}
						<Button
							leftSection={isReview ? <ArrowClockwiseIcon size={16} /> : <SparkleIcon size={16} />}
							variant={isReview ? "default" : "filled"}
							onClick={handleGenerate}
							loading={isGenerating}
							disabled={instruction.trim().length === 0 || creating}
						>
							{isGenerating
								? t("contexts.bulk.generating")
								: isReview
									? t("contexts.bulk.regenerate")
									: t("contexts.bulk.generate")}
						</Button>
					</Group>
				</Stack>

				{isReview && (
					<Stack gap="sm">
						<Group justify="space-between" align="flex-end">
							<Stack gap={2}>
								<Text fw={600}>{t("contexts.bulk.reviewTitle", { count: rows.length })}</Text>
								<Text size="xs" c="dimmed">
									{t("contexts.bulk.reviewHint")}
								</Text>
							</Stack>
							<Button
								variant="light"
								size="xs"
								leftSection={<PlusIcon size={14} />}
								onClick={addManualRow}
								disabled={creating || isGenerating}
							>
								{t("contexts.bulk.addContext")}
							</Button>
						</Group>

						<Stack gap="xs">
							{rows.map((row, index) => (
								<Box
									key={row.rowId}
									p="xs"
									style={{
										border: "1px solid var(--mantine-color-default-border)",
										borderRadius: "var(--mantine-radius-sm)",
										opacity: row.include ? 1 : 0.5,
									}}
								>
									<Stack gap="xs">
										<Group gap="sm" align="flex-end" wrap="nowrap">
											<Checkbox
												checked={row.include}
												onChange={(event) => setRowInclude(row.rowId, event.currentTarget.checked)}
												aria-label={t("contexts.bulk.columnInclude")}
											/>
											<TextInput
												label={index === 0 ? t("contexts.bulk.columnContextName") : undefined}
												value={row.contextName}
												onChange={(event) =>
													editRowField(row.rowId, "contextName", event.currentTarget.value)
												}
												disabled={!row.include}
												style={{ flex: 1 }}
												size="xs"
											/>
											<TextInput
												label={index === 0 ? t("contexts.bulk.columnBranchName") : undefined}
												value={row.branchName}
												onChange={(event) =>
													editRowField(row.rowId, "branchName", event.currentTarget.value)
												}
												disabled={!row.include}
												style={{ flex: 1 }}
												size="xs"
											/>
											{row.origin === "generated" && (
												<Tooltip label={t("contexts.bulk.generatedTooltip")} withArrow>
													<Badge
														size="sm"
														variant="light"
														color="grape"
														leftSection={<SparkleIcon size={12} />}
													>
														{t("contexts.bulk.generatedBadge")}
													</Badge>
												</Tooltip>
											)}
											<Tooltip label={t("contexts.bulk.removeContext")} withArrow>
												<ActionIcon
													variant="subtle"
													color="red"
													onClick={() => removeRow(row.rowId)}
													disabled={creating}
													aria-label={t("contexts.bulk.removeContext")}
												>
													<TrashIcon size={16} />
												</ActionIcon>
											</Tooltip>
										</Group>
										<Textarea
											label={index === 0 ? t("contexts.bulk.columnPreprompt") : undefined}
											placeholder={t("contexts.bulk.prepromptPlaceholder")}
											value={row.preprompt ?? ""}
											onChange={(event) =>
												editRowField(row.rowId, "preprompt", event.currentTarget.value)
											}
											disabled={!row.include}
											autosize
											minRows={2}
											maxRows={6}
											size="xs"
										/>
									</Stack>
								</Box>
							))}
						</Stack>

						<Group justify="flex-end">
							<Button variant="subtle" onClick={handleClose} disabled={creating}>
								{t("common.cancel")}
							</Button>
							<Button onClick={handleCreate} loading={creating} disabled={selectedCount === 0}>
								{selectedCount === 0
									? t("contexts.bulk.nothingSelected")
									: t("contexts.bulk.createSelected", { count: selectedCount })}
							</Button>
						</Group>
					</Stack>
				)}
			</Stack>
		</Modal>
	);
}
