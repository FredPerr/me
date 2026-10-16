import {
	ActionIcon,
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
import { commands, type ProjectView, type RepositoryBranchInput } from "@/bindings";
import { useBranchNameSuggestion } from "@/hooks/workspace/useBranchNameSuggestion";
import { useCreateContext } from "@/hooks/workspace/useContextMutations";
import { findDefaultBranch } from "@/models/projectView";
import { fuzzySelectFilter } from "@/utils/fuzzySelectFilter";

type CreateFromBranchesModalProps = {
	opened: boolean;
	onClose: () => void;
	project: ProjectView;
};

export function CreateFromBranchesModal({
	opened,
	onClose,
	project,
}: CreateFromBranchesModalProps) {
	const { t } = useTranslation();
	const createContext = useCreateContext();
	const repositories = project.effectiveRepositories;
	const isSingleRepo = repositories.length === 1;
	const [contextName, setContextName] = useState("");
	const [selectedBranches, setSelectedBranches] = useState<Record<string, string | null>>({});
	const [branchOptions, setBranchOptions] = useState<Record<string, string[]>>({});
	const [loadingBranches, setLoadingBranches] = useState(false);
	const suggestedBranchName = useBranchNameSuggestion(
		project.tag,
		contextName,
		project.branchNaming.defaultPrefix,
	);

	useEffect(() => {
		if (!opened) return;

		async function fetchBranches() {
			setLoadingBranches(true);
			const options: Record<string, string[]> = {};
			for (const repository of repositories) {
				options[repository.id] = await commands.listBranches(repository.path).catch(() => []);
			}
			setBranchOptions(options);
			setLoadingBranches(false);
		}

		fetchBranches();
	}, [opened, repositories]);

	function selectBranch(repositoryId: string, branch: string | null) {
		setSelectedBranches((prev) => ({ ...prev, [repositoryId]: branch }));
	}

	function copyBranchToName(repositoryId: string) {
		const branch = selectedBranches[repositoryId];
		if (branch) {
			setContextName(branch);
		}
	}

	const branchChoices: RepositoryBranchInput[] = repositories.map((repository) => {
		const selectedBranch = selectedBranches[repository.id];
		const branch = selectedBranch
			? { kind: "existingBranch" as const, name: selectedBranch }
			: {
					kind: "newBranch" as const,
					name: suggestedBranchName,
					baseBranch: findDefaultBranch(project, repository.id) ?? "",
				};
		return { repositoryId: repository.id, branch };
	});
	const canSubmit =
		!!contextName.trim() &&
		branchChoices.every(
			({ branch }) =>
				branch.name !== "" && (branch.kind !== "newBranch" || branch.baseBranch !== ""),
		);

	function reset() {
		setContextName("");
		setSelectedBranches({});
	}

	async function handleSubmit() {
		if (!canSubmit) return;
		try {
			await createContext.mutateAsync({
				projectTag: project.tag,
				name: contextName.trim(),
				baseContextId: null,
				origin: { kind: "fromBranches" },
				status: null,
				agentPrompt: null,
				branches: branchChoices,
			});
		} catch {
			return;
		}
		reset();
		onClose();
	}

	function handleClose() {
		reset();
		onClose();
	}

	return (
		<Modal opened={opened} onClose={handleClose} title={t("contexts.createFromBranches")}>
			<Stack gap="md">
				<Text size="sm" c="dimmed">
					{t("contexts.createFromBranchesDescription")}
				</Text>
				{repositories.map((repo) => {
					const branch = selectedBranches[repo.id] ?? null;
					return (
						<Box
							key={repo.id}
							p="xs"
							style={{
								border: "1px solid var(--mantine-color-default-border)",
								borderRadius: "var(--mantine-radius-sm)",
							}}
						>
							<Group gap="xs" align="flex-end" wrap="nowrap">
								<Select
									label={isSingleRepo ? t("contexts.branch") : repo.name}
									placeholder={
										loadingBranches
											? t("contexts.loadingBranches")
											: t("contexts.selectExistingBranch")
									}
									value={branch}
									onChange={(value) => selectBranch(repo.id, value)}
									data={branchOptions[repo.id] ?? []}
									nothingFoundMessage={t("contexts.noBranchesFound")}
									disabled={loadingBranches}
									searchable
									clearable
									filter={fuzzySelectFilter}
									size="xs"
									style={{ flex: 1 }}
								/>
								<Tooltip label={t("contexts.copyBranchToName")}>
									<ActionIcon
										variant="subtle"
										size="lg"
										onClick={() => copyBranchToName(repo.id)}
										disabled={!branch}
										aria-label={t("contexts.copyBranchToName")}
									>
										<PaintBrushHouseholdIcon size={16} />
									</ActionIcon>
								</Tooltip>
							</Group>
							{!branch && (
								<Text size="xs" c="dimmed" mt={4}>
									{t("contexts.repoGetsNewBranch", {
										branch: suggestedBranchName || t("contexts.branchFromContextName"),
										base: findDefaultBranch(project, repo.id) ?? "—",
									})}
								</Text>
							)}
						</Box>
					);
				})}
				<TextInput
					label={t("contexts.contextName")}
					placeholder="feature/my-feature"
					value={contextName}
					onChange={(e) => setContextName(e.currentTarget.value)}
					required
				/>
				<Group justify="flex-end">
					<Button variant="subtle" onClick={handleClose}>
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
