import { Alert, Stack, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { GIT_HOSTS } from "@/models/git-hosting/gitHosts";
import type { PullRequestsNotices as Notices } from "@/models/git-hosting/repositoryPullRequests";

type PullRequestsNoticesProps = {
	notices: Notices;
};

const LIST_SEPARATOR = ", ";

/** Says which repositories' pull requests are missing or cut short, and why. */
export function PullRequestsNotices({ notices }: PullRequestsNoticesProps) {
	const { t } = useTranslation();
	const { notConnected, failures, truncatedLabels } = notices;
	if (notConnected.length === 0 && failures.length === 0 && truncatedLabels.length === 0) {
		return null;
	}
	return (
		<Stack gap="xs">
			{notConnected.map(({ host, labels }) => (
				<Alert key={host} color="yellow">
					{t("pullRequests.notices.notConnected", {
						host: t(GIT_HOSTS[host].displayNameKey),
						repositories: labels.join(LIST_SEPARATOR),
					})}
				</Alert>
			))}
			{failures.length > 0 && (
				<Alert color="red" title={t("pullRequests.notices.failedTitle")}>
					<Stack gap={4}>
						{failures.map(({ label, message }) => (
							<Text key={label} size="sm" style={{ wordBreak: "break-word" }}>
								{t("pullRequests.notices.failed", { repository: label, message })}
							</Text>
						))}
					</Stack>
				</Alert>
			)}
			{truncatedLabels.length > 0 && (
				<Text size="xs" c="dimmed">
					{t("pullRequests.notices.truncated", {
						repositories: truncatedLabels.join(LIST_SEPARATOR),
					})}
				</Text>
			)}
		</Stack>
	);
}
