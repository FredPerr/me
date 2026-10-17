import {
	ActionIcon,
	Autocomplete,
	Box,
	Button,
	Group,
	Modal,
	Select,
	Stack,
	Text,
	TextInput,
	Tooltip,
} from "@mantine/core";
import { PaintBrushHouseholdIcon } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	type ContextView,
	commands,
	type ProjectView,
	type RepositoryBranchInput,
} from "@/bindings";
import { useBranchNameSuggestion } from "@/hooks/workspace/useBranchNameSuggestion";
import { useCreateContext } from "@/hooks/workspace/useContextMutations";
import { findContextBranch, findDefaultBranch } from "@/models/projectView";
import { fuzzySelectFilter } from "@/utils/fuzzySelectFilter";
import { BranchNameFromTaskPopover } from "./BranchNameFromTaskPopover";

type CreateFromContextModalProps = {
	opened: boolean;
	onClose: () => void;
	sourceContext: ContextView;
	project: ProjectView;
};

type RepositoryFormState = {
	/** An existing branch to check out, or a new name; empty falls back to the suggested name. */
	branchName: string;
	/** Branch a new branch starts from (ignored when the branch exists). */
	baseBranch: string;
};

function initialRepositoryStates(
	project: ProjectView,
	sourceContext: ContextView,
): Record<string, RepositoryFormState> {
	return Object.fromEntries(
		project.effectiveRepositories.map((repository) => {
			const baseBranch =
				findContextBranch(sourceContext, repository.id)?.branch ??
				findDefaultBranch(project, repository.id) ??
				"";
			return [repository.id, { branchName: "", baseBranch }];
		}),
	);
}

export function CreateFromContextModal({
	opened,
	onClose,
	sourceContext,
	project,
}: CreateFromContextModalProps) {
	const { t } = useTranslation();
	const createContext = useCreateContext();
	const repositories = project.effectiveRepositories;
	const [contextName, setContextName] = useState("");
	const [repositoryStates, setRepositoryStates] = useState(() =>
		initialRepositoryStates(project, sourceContext),
	);
	const [syncBranchNames, setSyncBranchNames] = useState(true);
	const [isBranchPopoverOpened, setIsBranchPopoverOpened] = useState(false);
	const [branchOptions, setBranchOptions] = useState<Record<string, string[]>>({});
	const suggestedBranchName = useBranchNameSuggestion(
		project.tag,
		contextName,
		project.branchNaming.defaultPrefix,
	);

	useEffect(() => {
		async function fetchBranches() {
			const options: Record<string, string[]> = {};
			for (const repository of repositories) {
				options[repository.id] = await commands.listBranches(repository.path).catch(() => []);
			}
			setBranchOptions(options);
		}
		if (opened) {
			fetchBranches();
		}
	}, [opened, repositories]);

	useEffect(() => {
		if (!opened) setIsBranchPopoverOpened(false);
	}, [opened]);

	function updateRepositoryState(repositoryId: string, patch: Partial<RepositoryFormState>) {
		setRepositoryStates((current) => ({
			...current,
			[repositoryId]: { ...current[repositoryId], ...patch },
		}));
	}

	function branchNameFor(repositoryId: string): string {
		if (syncBranchNames) return suggestedBranchName;
		return repositoryStates[repositoryId].branchName.trim() || suggestedBranchName;
	}

	function isExistingBranch(repositoryId: string, branchName: string): boolean {
		return (branchOptions[repositoryId] ?? []).includes(branchName);
	}

	function applyGeneratedBranchName(branchName: string) {
		setSyncBranchNames(false);
		setRepositoryStates((current) =>
			Object.fromEntries(
				Object.entries(current).map(([repositoryId, state]) => [
					repositoryId,
					{ ...state, branchName },
				]),
			),
		);
		if (!contextName.trim()) {
			setContextName(branchName);
		}
	}

	function toggleSync() {
		setSyncBranchNames((enabled) => !enabled);
	}

	const branchChoices: RepositoryBranchInput[] = repositories.map((repository) => {
		const name = branchNameFor(repository.id);
		const branch = isExistingBranch(repository.id, name)
			? { kind: "existingBranch" as const, name }
			: {
					kind: "newBranch" as const,
					name,
					baseBranch: repositoryStates[repository.id].baseBranch,
				};
		return { repositoryId: repository.id, branch };
	});
	const canSubmit =
		contextName.trim() !== "" &&
		branchChoices.every(
			({ branch }) =>
				(branch.kind === "pullRequest" || branch.name !== "") &&
				(branch.kind !== "newBranch" || branch.baseBranch !== ""),
		);

	async function handleSubmit() {
		if (!canSubmit) return;
		try {
			await createContext.mutateAsync({
				projectTag: project.tag,
				name: contextName.trim(),
				baseContextId: sourceContext.id,
				origin: { kind: "fromContext" },
				status: null,
				agentPrompt: null,
				branches: branchChoices,
			});
		} catch {
			return;
		}
		setContextName("");
		setRepositoryStates(initialRepositoryStates(project, sourceContext));
		onClose();
	}

	return (
		<Modal
			opened={opened}
			onClose={onClose}
			closeOnEscape={!isBranchPopoverOpened}
			title={t("contexts.createFromContext")}
		>
			<Stack gap="md">
				<Text size="sm" c="dimmed">
					{t("contexts.createFromContextDescription", {
						name: sourceContext.isDefault ? t("common.default") : sourceContext.name,
					})}
				</Text>
				<TextInput
					label={t("contexts.contextName")}
					placeholder="feature/my-feature"
					value={contextName}
					onChange={(event) => setContextName(event.currentTarget.value)}
					required
					rightSectionWidth={60}
					rightSection={
						<Group gap={4} wrap="nowrap">
							<BranchNameFromTaskPopover
								project={project}
								opened={isBranchPopoverOpened}
								onOpenedChange={setIsBranchPopoverOpened}
								onApply={applyGeneratedBranchName}
							/>
							<Tooltip
								label={
									syncBranchNames ? t("contexts.syncBranchesOn") : t("contexts.syncBranchesOff")
								}
							>
								<ActionIcon
									variant={syncBranchNames ? "filled" : "subtle"}
									size="sm"
									onClick={toggleSync}
									aria-label={t("contexts.syncBranchNames")}
									aria-pressed={syncBranchNames}
								>
									<PaintBrushHouseholdIcon size={16} />
								</ActionIcon>
							</Tooltip>
						</Group>
					}
				/>
				{repositories.map((repository) => {
					const state = repositoryStates[repository.id];
					const branches = branchOptions[repository.id] ?? [];
					const branchName = branchNameFor(repository.id);
					const branchExists = isExistingBranch(repository.id, branchName);
					return (
						<Box
							key={repository.id}
							p="xs"
							style={{
								border: "1px solid var(--mantine-color-default-border)",
								borderRadius: "var(--mantine-radius-sm)",
							}}
						>
							<Stack gap="xs">
								<Text size="sm" fw={500}>
									{repository.name}
								</Text>
								<Group gap="xs" grow align="flex-start">
									<Select
										label={t("contexts.baseBranch")}
										value={state.baseBranch || null}
										onChange={(value) =>
											updateRepositoryState(repository.id, { baseBranch: value ?? "" })
										}
										data={branches}
										placeholder={t("contexts.selectExistingBranch")}
										nothingFoundMessage={t("contexts.noBranchesFound")}
										disabled={branchExists}
										searchable
										size="xs"
									/>
									<Autocomplete
										label={t("contexts.contextBranch")}
										description={
											branchName
												? branchExists
													? t("contexts.branchExistsHint")
													: t("contexts.branchNewHint")
												: undefined
										}
										placeholder={suggestedBranchName || t("contexts.selectExistingBranch")}
										value={syncBranchNames ? suggestedBranchName : state.branchName}
										onChange={(value) =>
											updateRepositoryState(repository.id, { branchName: value })
										}
										data={branches}
										readOnly={syncBranchNames}
										filter={fuzzySelectFilter}
										size="xs"
									/>
								</Group>
							</Stack>
						</Box>
					);
				})}
				<Group justify="flex-end">
					<Button variant="subtle" onClick={onClose}>
						{t("common.cancel")}
					</Button>
					<Button onClick={handleSubmit} loading={createContext.isPending} disabled={!canSubmit}>
						{t("common.create")}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}
