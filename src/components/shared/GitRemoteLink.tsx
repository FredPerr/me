import { ActionIcon, Tooltip } from "@mantine/core";
import { GithubLogoIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useGitRemote } from "@/hooks/useGitRemote";

type GitRemoteLinkProps = {
	path: string | undefined;
};

export function GitRemoteLink({ path }: GitRemoteLinkProps) {
	const { remoteUrl } = useGitRemote(path);

	if (!remoteUrl) {
		return null;
	}

	return (
		<Tooltip label="Open repository">
			<ActionIcon variant="subtle" size="sm" onClick={() => openUrl(remoteUrl)} aria-label="Open repository">
				<GithubLogoIcon size={16} />
			</ActionIcon>
		</Tooltip>
	);
}
