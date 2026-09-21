import { ActionIcon, Tooltip } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { GithubLogoIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import { RepositoryRemotesModal } from "@/components/shared/RepositoryRemotesModal";
import { useProjectRemotes } from "@/hooks/useProjectRemotes";
import type { Project } from "@/models/Project";

type GitRemoteLinkProps = {
	project: Project;
};

export function GitRemoteLink({ project }: GitRemoteLinkProps) {
	const { t } = useTranslation();
	const { remotes } = useProjectRemotes(project);
	const [opened, { open, close }] = useDisclosure(false);

	if (remotes.length === 0) {
		return null;
	}

	function handleClick() {
		if (remotes.length === 1) {
			openUrl(remotes[0].remoteUrl);
		} else {
			open();
		}
	}

	return (
		<>
			<Tooltip label={t("project.openRepository")}>
				<ActionIcon
					variant="subtle"
					size="sm"
					onClick={handleClick}
					aria-label={t("project.openRepository")}
				>
					<GithubLogoIcon size={16} />
				</ActionIcon>
			</Tooltip>
			<RepositoryRemotesModal opened={opened} onClose={close} remotes={remotes} />
		</>
	);
}
