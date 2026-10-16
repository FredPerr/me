import { Badge } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { HostKindView } from "@/bindings";
import { GIT_HOSTS } from "@/models/git-hosting/gitHosts";

const HOST_COLOR: Record<HostKindView, string> = {
	github: "gray",
	bitbucket: "blue",
};

type ProviderBadgeProps = {
	host: HostKindView;
};

/** A small tag naming the Git host a pull request belongs to. */
export function ProviderBadge({ host }: ProviderBadgeProps) {
	const { t } = useTranslation();
	return (
		<Badge variant="light" size="sm" color={HOST_COLOR[host]}>
			{t(GIT_HOSTS[host].displayNameKey)}
		</Badge>
	);
}
