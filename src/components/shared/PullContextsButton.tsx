import { ActionIcon, Tooltip } from "@mantine/core";
import { CloudArrowDownIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { useGitRemote } from "@/hooks/useGitRemote";
import { usePullContexts } from "@/hooks/usePullContexts";
import type { Project } from "@/models/Project";

type PullContextsButtonProps = {
	project: Project;
};

export function PullContextsButton({ project }: PullContextsButtonProps) {
	const { t } = useTranslation();
	const { remoteUrl } = useGitRemote(project.path);
	const { pullAll, pulling } = usePullContexts(project);

	if (!remoteUrl) {
		return null;
	}

	return (
		<Tooltip label={t("project.pullContexts")}>
			<ActionIcon
				variant="subtle"
				size="sm"
				loading={pulling}
				onClick={pullAll}
				aria-label="Pull all context branches"
			>
				<CloudArrowDownIcon size={16} />
			</ActionIcon>
		</Tooltip>
	);
}
