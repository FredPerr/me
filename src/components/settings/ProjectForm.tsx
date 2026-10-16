import { ActionIcon, Button, Group, Input, Stack, TextInput, Tooltip } from "@mantine/core";
import { FolderIcon, GitBranchIcon, PlusIcon, TrashIcon } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	commands,
	type ProjectSettingsInput,
	type ProjectView,
	type RemoteProjectLinkView,
	type RepositoryInput,
} from "@/bindings";
import { IconPicker } from "@/components/shared/IconPicker";
import { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";
import { pickFolder } from "@/hooks/useFolderPicker";
import { resolvePath } from "@/utils/resolvePath";
import { BranchPrefixFormList } from "./BranchPrefixFormList";
import { RemoteProjectLinksField } from "./RemoteProjectLinksField";
import { RepositoryFormList } from "./RepositoryFormList";

const DEFAULT_BRANCH_PREFIXES = ["feature", "fix", "chore", "refactor"];

type ProjectFormProps = {
	initialProject?: ProjectView;
	submitting: boolean;
	onSubmit: (projectTag: string, settings: ProjectSettingsInput) => void;
	onCancel: () => void;
};

function toRepositoryInputs(project: ProjectView | undefined): RepositoryInput[] {
	return (project?.repositories ?? []).map(({ id, name, relativePath, postCheckoutCommand }) => ({
		id,
		name,
		relativePath,
		postCheckoutCommand,
	}));
}

function toRemoteProjectLinkViews(links: RemoteProjectLinks): RemoteProjectLinkView[] {
	return links
		.toJSON()
		.map((link) => ({ ...link, remoteProjectUrl: link.remoteProjectUrl ?? null }));
}

export function ProjectForm({ initialProject, submitting, onSubmit, onCancel }: ProjectFormProps) {
	const { t } = useTranslation();
	const [name, setName] = useState(initialProject?.name ?? "");
	const [tag, setTag] = useState(initialProject?.tag ?? "");
	const [path, setPath] = useState(initialProject?.path ?? "");
	const [icon, setIcon] = useState(initialProject?.icon ?? "");
	const [repositories, setRepositories] = useState(() => toRepositoryInputs(initialProject));
	const [isGitRoot, setIsGitRoot] = useState<boolean | null>(null);
	const [repositoriesValid, setRepositoriesValid] = useState(true);
	const [symlinks, setSymlinks] = useState<string[]>(initialProject?.symlinks ?? []);
	const [branchPrefixes, setBranchPrefixes] = useState<string[]>(
		initialProject?.branchNaming.prefixes ?? DEFAULT_BRANCH_PREFIXES,
	);
	const [allowNoBranchPrefix, setAllowNoBranchPrefix] = useState(
		initialProject?.branchNaming.allowsNoPrefix ?? true,
	);
	const [remoteProjectLinks, setRemoteProjectLinks] = useState(() =>
		RemoteProjectLinks.fromJSON(initialProject?.remoteProjectLinks),
	);

	const isMultiRepo =
		repositories.length > 1 || (repositories.length === 1 && repositories[0].relativePath !== ".");
	const isEditing = !!initialProject;
	const canSubmit = (isMultiRepo ? repositoriesValid : isGitRoot !== false) && !submitting;

	useEffect(() => {
		if (path) {
			resolvePath(path, {}).then((resolved) => {
				commands.checkIsGitRepository(resolved).then(setIsGitRoot);
			});
		} else {
			setIsGitRoot(null);
		}
	}, [path]);

	async function handlePickFolder() {
		const result = await pickFolder();
		if (result) {
			setPath(result.path);
			if (!name) setName(result.suggestedName);
			if (!tag) setTag(result.suggestedName);
		}
	}

	async function handleSubmit(event: React.FormEvent) {
		event.preventDefault();
		const resolvedPath = await resolvePath(path, { isFolder: true });
		onSubmit(tag.trim(), {
			name,
			path: resolvedPath,
			icon: icon || null,
			repositories,
			symlinks: symlinks.filter((symlink) => symlink.trim()),
			branchNaming: { prefixes: branchPrefixes, allowNoPrefix: allowNoBranchPrefix },
			remoteProjectLinks: toRemoteProjectLinkViews(remoteProjectLinks),
		});
	}

	return (
		<form onSubmit={handleSubmit}>
			<Stack gap="sm">
				<IconPicker value={icon} onChange={setIcon} />
				<TextInput
					label={t("settings.projectForm.parentPath")}
					description={t("settings.projectForm.parentPathDescription")}
					placeholder="~/Projects/my-project"
					value={path}
					onChange={(event) => setPath(event.currentTarget.value)}
					required
					rightSectionWidth={50}
					rightSection={
						<Group gap={6} wrap="nowrap">
							{!isMultiRepo && isGitRoot !== null && (
								<Tooltip
									label={
										isGitRoot
											? t("settings.projectForm.gitRepoRoot")
											: t("settings.projectForm.notGitRepo")
									}
								>
									<GitBranchIcon
										size={14}
										color={
											isGitRoot ? "var(--mantine-color-green-5)" : "var(--mantine-color-red-5)"
										}
									/>
								</Tooltip>
							)}
							<Tooltip label={t("settings.projectForm.browseFolderPath")}>
								<ActionIcon
									variant="transparent"
									size="xs"
									onClick={handlePickFolder}
									aria-label="Browse folder"
								>
									<FolderIcon size={14} />
								</ActionIcon>
							</Tooltip>
						</Group>
					}
				/>
				<TextInput
					label={t("settings.projectForm.name")}
					description={t("settings.projectForm.nameDescription")}
					placeholder="My Project"
					value={name}
					onChange={(event) => setName(event.currentTarget.value)}
					required
				/>
				<TextInput
					label={t("settings.projectForm.tag")}
					description={t("settings.projectForm.tagDescription")}
					placeholder="my-project"
					value={tag}
					onChange={(event) => setTag(event.currentTarget.value)}
					disabled={isEditing}
					required
				/>
				<RepositoryFormList
					repositories={repositories}
					onChange={setRepositories}
					projectName={name}
					projectPath={path}
					onValidityChange={setRepositoriesValid}
				/>
				<Stack gap="xs">
					<div>
						<Input.Label>{t("settings.projectForm.symlinks")}</Input.Label>
						<Input.Description>{t("settings.projectForm.symlinksDescription")}</Input.Description>
					</div>
					{symlinks.map((symlink, index) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: symlink paths have no stable ID
						<Group key={index} gap="xs">
							<TextInput
								placeholder={t("settings.projectForm.symlinkPlaceholder")}
								value={symlink}
								onChange={(e) => {
									const updated = [...symlinks];
									updated[index] = e.currentTarget.value;
									setSymlinks(updated);
								}}
								size="xs"
								style={{ flex: 1 }}
							/>
							<ActionIcon
								variant="subtle"
								color="red"
								size="sm"
								onClick={() => setSymlinks(symlinks.filter((_, i) => i !== index))}
								aria-label="Remove symlink"
							>
								<TrashIcon size={14} />
							</ActionIcon>
						</Group>
					))}
					<Group>
						<Button
							variant="outline"
							size="xs"
							color="gray"
							leftSection={<PlusIcon size={14} />}
							onClick={() => setSymlinks([...symlinks, ""])}
						>
							{t("settings.projectForm.addSymlink")}
						</Button>
					</Group>
				</Stack>
				<BranchPrefixFormList
					prefixes={branchPrefixes}
					allowNoPrefix={allowNoBranchPrefix}
					onPrefixesChange={setBranchPrefixes}
					onAllowNoPrefixChange={setAllowNoBranchPrefix}
				/>
				<RemoteProjectLinksField value={remoteProjectLinks} onChange={setRemoteProjectLinks} />
				<Group justify="flex-end">
					<Button variant="subtle" onClick={onCancel}>
						{t("common.cancel")}
					</Button>
					<Button type="submit" disabled={!canSubmit} loading={submitting}>
						{isEditing ? t("common.save") : t("common.add")}
					</Button>
				</Group>
			</Stack>
		</form>
	);
}
