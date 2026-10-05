import { ActionIcon, Tooltip } from "@mantine/core";
import { GitMergeIcon, GitPullRequestIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import {
	type ProviderPullRequest,
	type PullRequestStatus,
	pullRequestStatus,
} from "@/models/git-provider/GitProvider";

const STATUS_COLOR: Record<PullRequestStatus, string> = {
	merged: "var(--mantine-color-grape-5)",
	open: "var(--mantine-color-green-5)",
	draft: "var(--mantine-color-gray-5)",
	closed: "var(--mantine-color-red-5)",
};

type PullRequestStatusIconProps = {
	pullRequest: ProviderPullRequest;
};

/**
 * GitHub-style status glyph for a pull request. Color follows GitHub's
 * conventions (merged purple, open green, draft gray, closed red) and clicking
 * opens the pull request in the browser.
 */
export function PullRequestStatusIcon({ pullRequest }: PullRequestStatusIconProps) {
	const { t } = useTranslation();
	const status = pullRequestStatus(pullRequest);
	const Icon = status === "merged" ? GitMergeIcon : GitPullRequestIcon;
	const label = t(`contexts.pullRequestStatus.${status}`, { number: pullRequest.number });

	return (
		<Tooltip label={label}>
			<ActionIcon
				variant="subtle"
				size="sm"
				radius={2}
				bg="transparent"
				onClick={() => openUrl(pullRequest.htmlUrl)}
				aria-label={label}
			>
				<Icon size={12} color={STATUS_COLOR[status]} weight="fill" />
			</ActionIcon>
		</Tooltip>
	);
}
