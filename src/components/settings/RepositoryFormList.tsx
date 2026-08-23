import { ActionIcon, Alert, Button, Group, Stack, Switch, TextInput, Tooltip } from "@mantine/core";
import { FolderIcon, GitBranchIcon, PlusIcon, TrashIcon, WarningIcon } from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { IconPicker } from "@/components/shared/IconPicker";
import { pickRepositoryFolder } from "@/hooks/useFolderPicker";
import type { RepositoryData } from "@/models/Project";
import { resolvePath } from "@/utils/resolvePath";

type RepositoryFormListProps = {
	repositories: RepositoryData[];
	onChange: (repositories: RepositoryData[]) => void;
	projectName: string;
	projectPath: string;
};

export function RepositoryFormList({ repositories, onChange, projectName, projectPath }: RepositoryFormListProps) {
	const { t } = useTranslation();
	const [multiRepo, setMultiRepo] = useState(repositories.length > 1 || (repositories.length === 1 && repositories[0].relPath !== "."));
	const [gitStatus, setGitStatus] = useState<Record<string, boolean>>({});

	useEffect(() => {
		if (repositories.length > 1) {
			setMultiRepo(true);
		}
	}, [repositories.length]);

	useEffect(() => {
		if (!multiRepo) return;
		for (const repo of repositories) {
			if (repo.relPath) {
				resolveAndCheckGit(repo.id, repo.relPath);
			}
		}
	}, [multiRepo, repositories, resolveAndCheckGit]);

	async function resolveAndCheckGit(repoId: string, relPath: string) {
		try {
			const resolved = await resolvePath(relPath, { basePath: projectPath });
			const isGit = await invoke<boolean>("check_is_git_repository", { path: resolved });
			setGitStatus((prev) => ({ ...prev, [repoId]: isGit }));
		} catch {
			setGitStatus((prev) => ({ ...prev, [repoId]: false }));
		}
	}

	function handleMultiRepoToggle(checked: boolean) {
		setMultiRepo(checked);
		if (checked) {
			if (repositories.length === 0) {
				onChange([{ id: crypto.randomUUID(), name: "", relPath: "." }]);
			} else if (repositories.length === 1 && repositories[0].relPath === ".") {
				onChange([{ ...repositories[0], relPath: "" }]);
			}
		}
		if (!checked) {
			onChange([{ id: repositories[0]?.id ?? crypto.randomUUID(), name: projectName, relPath: "." }]);
		}
	}

	function handleAdd() {
		onChange([...repositories, { id: crypto.randomUUID(), name: "", relPath: "" }]);
	}

	function handleRemove(index: number) {
		if (repositories.length <= 1) return;
		onChange(repositories.filter((_, i) => i !== index));
	}

	function handleUpdate(index: number, field: keyof RepositoryData, value: string) {
		const updated = repositories.map((repo, i) => (i === index ? { ...repo, [field]: value } : repo));
		onChange(updated);
	}

	async function handlePickRepoFolder(index: number) {
		const result = await pickRepositoryFolder();
		if (result) {
			const updated = repositories.map((repo, i) => {
				if (i !== index) return repo;
				return {
					...repo,
					relPath: result.path,
					name: repo.name || result.suggestedName,
				};
			});
			onChange(updated);
			setGitStatus((prev) => ({ ...prev, [repositories[index].id]: result.isGitRepository }));
		}
	}

	const reposShareParent = multiRepo && repositories.length > 1 && checkReposShareParent(repositories);

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
						<Button variant="outline" size="xs" color="gray" leftSection={<PlusIcon size={14} />} onClick={handleAdd}>
							{t("settings.repositories.addRepository")}
						</Button>
					</Group>
					{repositories.map((repository, index) => (
						<Stack key={repository.id} gap={2}>
							<Group gap="xs" align="flex-end">
								<IconPicker
									value={repository.icon ?? ""}
									onChange={(value) => handleUpdate(index, "icon", value)}
								/>
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
									value={repository.relPath}
									onChange={(e) => handleUpdate(index, "relPath", e.currentTarget.value)}
									style={{ flex: 2 }}
									size="xs"
									rightSectionWidth={50}
									rightSection={
										<Group gap={4} wrap="nowrap">
											{repository.relPath && gitStatus[repository.id] !== undefined && (
												<Tooltip label={gitStatus[repository.id] ? t("settings.repositories.validGitRepo") : t("settings.repositories.notGitRepo")}>
													<GitBranchIcon
														size={14}
														color={gitStatus[repository.id] ? "var(--mantine-color-green-5)" : "var(--mantine-color-red-5)"}
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
										</Group>
									}
								/>
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
						</Stack>
					))}
				</>
			)}
		</Stack>
	);
}

function checkReposShareParent(repositories: RepositoryData[]): boolean {
	const paths = repositories
		.map((r) => r.relPath)
		.filter((p) => p && p !== ".");

	if (paths.length <= 1) return true;

	const parents = paths.map((p) => {
		const normalized = p.replace(/\/$/, "");
		const lastSlash = normalized.lastIndexOf("/");
		return lastSlash > 0 ? normalized.slice(0, lastSlash) : ".";
	});

	return parents.every((parent) => parent === parents[0]);
}
