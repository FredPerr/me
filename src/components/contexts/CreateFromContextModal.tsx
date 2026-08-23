import { Button, Group, Modal, Stack, Text, TextInput } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Context, Project } from "@/models/Project";

type CreateFromContextModalProps = {
	opened: boolean;
	onClose: () => void;
	sourceContext: Context;
	project: Project;
	onCreate: (params: { name: string; branchName: string; baseBranches: Record<string, string> }) => Promise<void>;
};

export function CreateFromContextModal({
	opened,
	onClose,
	sourceContext,
	project,
	onCreate,
}: CreateFromContextModalProps) {
	const { t } = useTranslation();
	const [branchName, setBranchName] = useState("");
	const [baseBranches, setBaseBranches] = useState<Record<string, string>>(() =>
		Object.fromEntries(
			sourceContext.branches.map((cb) => [cb.repositoryId, cb.branch]),
		),
	);
	const [loading, setLoading] = useState(false);

	function handleBaseBranchChange(repositoryId: string, value: string) {
		setBaseBranches((prev) => ({ ...prev, [repositoryId]: value }));
	}

	async function handleSubmit() {
		if (!branchName.trim()) return;

		setLoading(true);
		try {
			await onCreate({
				name: branchName.trim(),
				branchName: branchName.trim(),
				baseBranches,
			});
			setBranchName("");
			onClose();
		} finally {
			setLoading(false);
		}
	}

	return (
		<Modal opened={opened} onClose={onClose} title={t("contexts.createFromContext")}>
			<Stack gap="md">
				<Text size="sm" c="dimmed">
					{t("contexts.createFromContextDescription", { name: sourceContext.name })}
				</Text>
				<TextInput
					label={t("contexts.newBranchName")}
					placeholder="feature/my-feature"
					value={branchName}
					onChange={(e) => setBranchName(e.currentTarget.value)}
					required
				/>
				{project.repositories.map((repo) => {
					const currentBase = baseBranches[repo.id] ?? "";
					return (
						<TextInput
							key={repo.id}
							label={`${t("contexts.baseBranch")} — ${repo.name}`}
							value={currentBase}
							onChange={(e) => handleBaseBranchChange(repo.id, e.currentTarget.value)}
							size="xs"
						/>
					);
				})}
				<Group justify="flex-end">
					<Button variant="subtle" onClick={onClose}>
						{t("common.cancel")}
					</Button>
					<Button onClick={handleSubmit} loading={loading} disabled={!branchName.trim()}>
						{t("common.create")}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}
