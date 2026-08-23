import { Button, Group, Select, TextInput } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { PlusIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useBranches } from "@/hooks/useBranches";
import type { Project } from "@/models/Project";

type CreateContextFormProps = {
	project: Project;
	onCreate: (params: { name: string; branchName: string; baseBranches: Record<string, string> }) => Promise<void>;
};

export function CreateContextForm({ project, onCreate }: CreateContextFormProps) {
	const { t } = useTranslation();
	const primaryRepo = project.repositories[0];
	const { branches } = useBranches(primaryRepo ? `${project.path}/${primaryRepo.relPath.replace("./", "")}` : undefined);

	const [branchName, setBranchName] = useState("");
	const [baseBranch, setBaseBranch] = useState<string | null>(null);
	const [loading, setLoading] = useState(false);

	const branchOptions = branches.map((b) => ({ value: b, label: b }));
	const defaultBase = branches.find((b) => b === "main") ?? branches.find((b) => b === "master") ?? branches[0];

	async function handleCreate() {
		if (!branchName.trim()) return;

		const base = baseBranch ?? defaultBase;
		if (!base) {
			notifications.show({ title: t("common.error"), message: t("contexts.noBaseBranch"), color: "red" });
			return;
		}

		setLoading(true);
		try {
			const baseBranches: Record<string, string> = {};
			for (const repo of project.repositories) {
				baseBranches[repo.id] = base;
			}

			await onCreate({
				name: branchName.trim(),
				branchName: branchName.trim(),
				baseBranches,
			});

			notifications.show({
				title: t("contexts.created"),
				message: t("contexts.createdMessage", { name: branchName.trim() }),
				color: "green",
			});
			setBranchName("");
		} catch (error) {
			notifications.show({
				title: t("contexts.createFailed"),
				message: String(error),
				color: "red",
			});
		} finally {
			setLoading(false);
		}
	}

	return (
		<Group gap="xs" align="flex-end">
			<Select
				label={t("contexts.baseBranch")}
				placeholder="Select base branch"
				data={branchOptions}
				value={baseBranch ?? defaultBase ?? null}
				onChange={setBaseBranch}
				searchable
				size="xs"
				style={{ flex: 1 }}
			/>
			<TextInput
				label={t("contexts.newBranchName")}
				placeholder="feature/my-feature"
				value={branchName}
				onChange={(e) => setBranchName(e.currentTarget.value)}
				size="xs"
				style={{ flex: 2 }}
			/>
			<Button
				size="xs"
				leftSection={<PlusIcon size={14} />}
				onClick={handleCreate}
				loading={loading}
				disabled={!branchName.trim()}
			>
				{t("common.create")}
			</Button>
		</Group>
	);
}
