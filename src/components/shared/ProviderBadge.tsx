import { Badge } from "@mantine/core";
import type { GitProviderId } from "@/models/git-provider/GitProvider";
import { getGitProvider } from "@/models/git-provider/GitProviderRegistry";

/** Mantine color used for each provider's tag. */
const PROVIDER_COLOR: Record<GitProviderId, string> = {
	github: "gray",
	bitbucket: "blue",
};

type ProviderBadgeProps = {
	providerId: GitProviderId;
};

/** A small tag identifying which git provider a pull request belongs to. */
export function ProviderBadge({ providerId }: ProviderBadgeProps) {
	return (
		<Badge variant="light" size="sm" color={PROVIDER_COLOR[providerId]}>
			{getGitProvider(providerId).displayName}
		</Badge>
	);
}
