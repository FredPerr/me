import { ActionIcon, Tooltip } from "@mantine/core";
import { GitPullRequestIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";

type PullRequestButtonProps = {
	prUrl: string | null;
	remoteUrl: string | undefined;
};

export function PullRequestButton({ prUrl, remoteUrl }: PullRequestButtonProps) {
	const isGitHub = !!prUrl;
	const tooltipLabel = isGitHub
		? "Create pull request"
		: remoteUrl
			? "PR creation only available for GitHub"
			: "No remote configured";

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
