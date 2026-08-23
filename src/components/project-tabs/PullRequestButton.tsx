import { ActionIcon, Tooltip } from "@mantine/core";
import { GitPullRequestIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";

type PullRequestButtonProps = {
	prUrl: string | null;
	remoteUrl: string | undefined;
};

export function PullRequestButton({ prUrl, remoteUrl }: PullRequestButtonProps) {
	const { t } = useTranslation();
	const isGitHub = !!prUrl;
	const tooltipLabel = isGitHub
		? t("project.createPullRequest")
		: remoteUrl
			? t("project.prNotAvailable")
			: t("project.noRemoteConfigured");

	function handleClick() {
		if (prUrl) {
			openUrl(prUrl);
		}
	}

	return (
		<Tooltip label={tooltipLabel}>
			<ActionIcon
				variant="subtle"
				size="sm"
				onClick={handleClick}
				disabled={!isGitHub}
				aria-label="Create pull request"
			>
				<GitPullRequestIcon size={16} />
			</ActionIcon>
		</Tooltip>
	);
}
