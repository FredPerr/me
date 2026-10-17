import {
	ActionIcon,
	Badge,
	Box,
	Button,
	Checkbox,
	Group,
	Modal,
	Select,
	Stack,
	Text,
	Textarea,
	TextInput,
	Tooltip,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { ArrowClockwiseIcon, PlusIcon, SparkleIcon, TrashIcon, XIcon } from "@phosphor-icons/react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	type AiAssistErrorKind,
	type ContextDraftView,
	commands,
	type ProjectView,
} from "@/bindings";
import { useCreateContexts } from "@/hooks/workspace/useCreateContexts";
import { isCommandError } from "@/infra/commandError";
import type { BulkContextSpec } from "@/models/bulk/BulkContextSpec";
import { findContext } from "@/models/projectView";

type BulkCreateContextsModalProps = {
	opened: boolean;
	onClose: () => void;
	project: ProjectView;
};

type RowOrigin = "generated" | "manual";

type ReviewRow = {
	rowId: string;
	origin: RowOrigin;
	include: boolean;
	contextName: string;
	branchName: string;
	prompt?: string;
};

type EditableRowField = "contextName" | "branchName" | "prompt";

/** Phase of the one-shot suggestion run. */
type GenerationPhase = "idle" | "generating";

function toGeneratedRow(draft: ContextDraftView): ReviewRow {
	return {
		rowId: crypto.randomUUID(),
		origin: "generated",
		include: true,
		contextName: draft.contextName,
		branchName: draft.branchName,
		prompt: draft.prompt ?? undefined,
	};
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

/**
 * The i18n key for an AI-assist error kind. The four parse kinds keep the same
 * messages the frontend parser mapped to before suggestions moved to Rust.
 */
function errorMessageKey(kind: AiAssistErrorKind): string {
	switch (kind) {
		case "noListFound":
			return "contexts.bulk.errorNoJson";
		case "malformed":
			return "contexts.bulk.errorInvalidJson";
		case "notAList":
			return "contexts.bulk.errorNotArray";
		case "noValidEntries":
			return "contexts.bulk.errorNoRows";
		case "cancelled":
			return "contexts.bulk.errorCancelled";
		case "timedOut":
			return "contexts.bulk.errorTimedOut";
		default:
			return "contexts.bulk.errorSession";
	}
}

/**
 * Bulk context creator. Phase one takes one high-level instruction and runs a
 * read-only Rust suggestion into draft contexts (named, with a valid branch and
 * an optional prompt). Phase two shows those drafts in an editable table —
 * context name, branch, and an optional Kiro prompt per row, each with an
 * include toggle — and creates the selected ones in a single batch. Prompts are
 * stored (prefilled into Kiro later), never auto-run.
 */
export function BulkCreateContextsModal({
	opened,
	onClose,
	project,
}: BulkCreateContextsModalProps) {
	const { t } = useTranslation();
	const [phase, setPhase] = useState<GenerationPhase>("idle");
	const [errorMessage, setErrorMessage] = useState<string | null>(null);
	const [instruction, setInstruction] = useState("");
	const [rows, setRows] = useState<ReviewRow[]>([]);
	const [baseContextId, setBaseContextId] = useState<string | null>(null);
	const { createContexts, creating } = useCreateContexts(project);

	// Existing non-default contexts a new batch can branch off of.
	const baseContextOptions = useMemo(
		() => project.contexts.filter((context) => !context.isDefault),
		[project.contexts],
	);

	const selectedCount = useMemo(() => rows.filter((row) => row.include).length, [rows]);

	function handleClose() {
		if (creating) return;
		void commands.aiAssistCancelContexts();
		setPhase("idle");
		setErrorMessage(null);
		setInstruction("");
		setRows([]);
		setBaseContextId(null);
		onClose();
	}

	async function handleGenerate() {
		const trimmed = instruction.trim();
		if (trimmed.length === 0) return;
		setPhase("generating");
		setErrorMessage(null);
		try {
			const drafts = await commands.aiAssistSuggestContexts(project.tag, trimmed);
			setRows((current) => [
				...current.filter((row) => row.origin === "manual"),
				...drafts.map(toGeneratedRow),
			]);
		} catch (error) {
			const kind = isCommandError(error) ? (error.kind as AiAssistErrorKind) : "failed";
			const message = isCommandError(error) ? error.message : String(error);
			// Cancel leaves the user where they were, silently.
			if (kind !== "cancelled") {
				setErrorMessage(t(errorMessageKey(kind), { message }));
			}
		} finally {
			setPhase("idle");
		}
	}

	function handleCancelGeneration() {
		void commands.aiAssistCancelContexts();
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
				agentPrompt: row.prompt?.trim() ? row.prompt.trim() : undefined,
			}));
		if (specs.length === 0) return;

		const baseContext = baseContextId ? findContext(project, baseContextId) : undefined;
		try {
			await createContexts(specs, baseContext);
		} catch {
			return;
		}
		notifications.show({
			title: t("contexts.bulk.createdTitle"),
			message: t("contexts.bulk.createdMessage", { count: specs.length }),
			color: "green",
		});
		handleClose();
	}

	const hasRows = rows.length > 0;
	const isReview = hasRows;
	const isGenerating = phase === "generating";

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
					{baseContextOptions.length > 0 && (
						<Select
							label={t("contexts.bulk.baseContextLabel")}
							description={t("contexts.bulk.baseContextHint")}
							placeholder={t("contexts.bulk.baseContextDefault")}
							data={baseContextOptions.map((context) => ({
								value: context.id,
								label: context.name,
							}))}
							value={baseContextId}
							onChange={setBaseContextId}
							disabled={isGenerating || creating}
							clearable
							searchable
						/>
					)}
					{errorMessage && (
						<Text size="xs" c="red.4">
							{errorMessage}
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
						{isGenerating && (
							<Button
								variant="subtle"
								color="red"
								leftSection={<XIcon size={16} />}
								onClick={handleCancelGeneration}
							>
								{t("contexts.bulk.cancelGeneration")}
							</Button>
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
											value={row.prompt ?? ""}
											onChange={(event) =>
												editRowField(row.rowId, "prompt", event.currentTarget.value)
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
