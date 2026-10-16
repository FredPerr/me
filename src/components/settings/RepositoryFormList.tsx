import { ActionIcon, Alert, Button, Group, Stack, Switch, TextInput, Tooltip } from "@mantine/core";
import { FolderIcon, GitBranchIcon, PlusIcon, TrashIcon, WarningIcon } from "@phosphor-icons/react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type RepositoryInput } from "@/bindings";
import { pickRepositoryFolder } from "@/hooks/useFolderPicker";
import { resolvePath } from "@/utils/resolvePath";

type RepositoryFormListProps = {
	repositories: RepositoryInput[];
	onChange: (repositories: RepositoryInput[]) => void;
	projectName: string;
	projectPath: string;
	onValidityChange?: (allValid: boolean) => void;
};

export function RepositoryFormList({
	repositories,
	onChange,
	projectName,
	projectPath,
	onValidityChange,
}: RepositoryFormListProps) {
	const { t } = useTranslation();
	const [multiRepo, setMultiRepo] = useState(
		repositories.length > 1 || (repositories.length === 1 && repositories[0].relativePath !== "."),
	);
	const [gitStatus, setGitStatus] = useState<Record<string, boolean>>({});

	useEffect(() => {
		if (repositories.length > 1) {
			setMultiRepo(true);
		}
	}, [repositories.length]);

	const resolveAndCheckGit = useCallback(
		async (repoId: string, relativePath: string) => {
			try {
				const resolved = await resolvePath(relativePath, { basePath: projectPath });
				const isGit = await commands.checkIsGitRepository(resolved);
				setGitStatus((prev) => ({ ...prev, [repoId]: isGit }));
			} catch {
				setGitStatus((prev) => ({ ...prev, [repoId]: false }));
			}
		},
		[projectPath],
	);

	useEffect(() => {
		if (!multiRepo) return;
		for (const repo of repositories) {
			if (repo.relativePath) {
				resolveAndCheckGit(repo.id, repo.relativePath);
			}
		}
	}, [multiRepo, repositories, resolveAndCheckGit]);

	useEffect(() => {
		if (!onValidityChange) return;
		if (!multiRepo) {
			onValidityChange(true);
			return;
		}
		const allHavePath = repositories.every((r) => r.relativePath);
		const allValid =
			allHavePath && repositories.length > 0 && repositories.every((r) => gitStatus[r.id] === true);
		onValidityChange(allValid);
	}, [multiRepo, repositories, gitStatus, onValidityChange]);

	function handleMultiRepoToggle(checked: boolean) {
		setMultiRepo(checked);
		if (checked) {
			if (repositories.length === 0) {
				onChange([newRepository(".")]);
			} else if (repositories.length === 1 && repositories[0].relativePath === ".") {
				onChange([{ ...repositories[0], relativePath: "" }]);
			}
		}
		if (!checked) {
			onChange([
				{
					...newRepository("."),
					id: repositories[0]?.id ?? crypto.randomUUID(),
					name: projectName,
				},
			]);
		}
	}

	function handleAdd() {
		onChange([...repositories, newRepository("")]);
	}

	function handleRemove(index: number) {
		if (repositories.length <= 1) return;
		onChange(repositories.filter((_, i) => i !== index));
	}

	function handleUpdate(
		index: number,
		field: "name" | "relativePath" | "postCheckoutCommand",
		value: string,
	) {
		const updated = repositories.map((repo, i) =>
			i === index ? { ...repo, [field]: value } : repo,
		);
		onChange(updated);
	}

	async function handlePickRepoFolder(index: number) {
		const result = await pickRepositoryFolder();
		if (result) {
			const updated = repositories.map((repo, i) => {
				if (i !== index) return repo;
				return {
					...repo,
					relativePath: result.path,
					name: repo.name || result.suggestedName,
				};
			});
			onChange(updated);
			setGitStatus((prev) => ({ ...prev, [repositories[index].id]: result.isGitRepository }));
		}
	}

	const reposShareParent =
		multiRepo && repositories.length > 1 && checkReposShareParent(repositories);

	return (
		<Stack gap="xs">
			<Switch
				label={t("settings.repositories.multipleRepos")}
				checked={multiRepo}
				onChange={(e) => handleMultiRepoToggle(e.currentTarget.checked)}
				size="xs"
			/>

			{multiRepo && (
				<>
					{!reposShareParent && repositories.length > 1 && (
						<Alert icon={<WarningIcon size={16} />} color="orange" variant="light" p="xs">
							{t("settings.repositories.notSameParentWarning")}
						</Alert>
					)}
					<Group justify="flex-end">
						<Button
							variant="outline"
							size="xs"
							color="gray"
							leftSection={<PlusIcon size={14} />}
							onClick={handleAdd}
						>
							{t("settings.repositories.addRepository")}
						</Button>
					</Group>
					{repositories.map((repository, index) => (
						<Stack key={repository.id} gap={2} bd="3px solid dark.6" p="md" bdrs={"lg"}>
							<Group gap="xs" align="flex-end">
								<TextInput
									label={t("settings.repositories.name")}
									placeholder="backend"
									required
									value={repository.name}
									onChange={(e) => handleUpdate(index, "name", e.currentTarget.value)}
									style={{ flex: 1 }}
									size="xs"
								/>
								<TextInput
									label={t("settings.repositories.path")}
									placeholder="./backend"
									required
									value={repository.relativePath}
									onChange={(e) => handleUpdate(index, "relativePath", e.currentTarget.value)}
									style={{ flex: 2 }}
									size="xs"
									rightSectionWidth={75}
									rightSection={
										<Group gap={4} wrap="nowrap">
											{repository.relativePath && gitStatus[repository.id] !== undefined && (
												<Tooltip
													label={
														gitStatus[repository.id]
															? t("settings.repositories.validGitRepo")
															: t("settings.repositories.notGitRepo")
													}
												>
													<GitBranchIcon
														size={14}
														color={
															gitStatus[repository.id]
																? "var(--mantine-color-green-5)"
																: "var(--mantine-color-red-5)"
														}
													/>
												</Tooltip>
											)}
											<Tooltip label={t("settings.projectForm.browseFolderPath")}>
												<ActionIcon
													variant="transparent"
													size="xs"
													onClick={() => handlePickRepoFolder(index)}
													aria-label="Browse folder"
												>
													<FolderIcon size={14} />
												</ActionIcon>
											</Tooltip>

											<ActionIcon
												variant="subtle"
												color="red"
												size="sm"
												onClick={() => handleRemove(index)}
												disabled={repositories.length <= 1}
												aria-label="Remove repository"
											>
												<TrashIcon size={14} />
											</ActionIcon>
										</Group>
									}
								/>
							</Group>
							<TextInput
								label={t("settings.repositories.postCheckoutCommandLabel")}
								placeholder={t("settings.repositories.postCheckoutCommandPlaceholder")}
								value={repository.postCheckoutCommand ?? ""}
								onChange={(e) => handleUpdate(index, "postCheckoutCommand", e.currentTarget.value)}
								size="xs"
							/>
						</Stack>
					))}
				</>
			)}
		</Stack>
	);
}

function newRepository(relativePath: string): RepositoryInput {
	return { id: crypto.randomUUID(), name: "", relativePath, postCheckoutCommand: null };
}

function checkReposShareParent(repositories: RepositoryInput[]): boolean {
	const paths = repositories.map((r) => r.relativePath).filter((p) => p && p !== ".");

	if (paths.length <= 1) return true;

	const parents = paths.map((p) => {
		const normalized = p.replace(/\/$/, "");
		const lastSlash = normalized.lastIndexOf("/");
		return lastSlash > 0 ? normalized.slice(0, lastSlash) : ".";
	});

	return parents.every((parent) => parent === parents[0]);
}
