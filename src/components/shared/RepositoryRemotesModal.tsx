import { ActionIcon, Group, Modal, Stack, Text, Tooltip } from "@mantine/core";
import { ArrowSquareOutIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import type { RepositoryRemote } from "@/hooks/useProjectRemotes";

type RepositoryRemotesModalProps = {
	opened: boolean;
	onClose: () => void;
	remotes: RepositoryRemote[];
};

export function RepositoryRemotesModal({ opened, onClose, remotes }: RepositoryRemotesModalProps) {
	const { t } = useTranslation();

	return (
		<Modal opened={opened} onClose={onClose} title={t("project.openRepository")} size="md">
			<Stack gap="xs">
				{remotes.map(({ repository, remoteUrl }) => (
					<Group
						key={repository.id}
						justify="space-between"
						wrap="nowrap"
						px="xs"
						py={6}
						style={{
							border: "1px solid var(--mantine-color-default-border)",
							borderRadius: "var(--mantine-radius-sm)",
						}}
					>
						<Group gap="xs" wrap="nowrap" style={{ minWidth: 0 }}>
							<GithubLogoIcon size={16} />
							<Stack gap={0} style={{ minWidth: 0 }}>
								<Text size="sm" fw={500} truncate>
									{repository.name}
								</Text>
								<Text size="xs" c="dimmed" truncate>
									{remoteUrl}
								</Text>
							</Stack>
						</Group>
						<Tooltip label={t("project.openRepository")}>
							<ActionIcon
								variant="subtle"
								onClick={() => openUrl(remoteUrl)}
								aria-label={`${t("project.openRepository")} — ${repository.name}`}
							>
								<ArrowSquareOutIcon size={16} />
							</ActionIcon>
						</Tooltip>
					</Group>
				))}
			</Stack>
		</Modal>
	);
}
