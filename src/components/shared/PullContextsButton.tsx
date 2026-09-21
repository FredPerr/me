import { ActionIcon, Tooltip } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { CloudArrowDownIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { RepositoryBranchesModal } from "@/components/shared/RepositoryBranchesModal";
import { useProjectHasRemote } from "@/hooks/useProjectHasRemote";
import type { Project } from "@/models/Project";

type PullContextsButtonProps = {
	project: Project;
};

export function PullContextsButton({ project }: PullContextsButtonProps) {
	const { t } = useTranslation();
	const { hasRemote } = useProjectHasRemote(project);
	const [opened, { open, close }] = useDisclosure(false);

	if (!hasRemote) {
		return null;
	}

	return (
		<>
			<Tooltip label={t("branches.title")}>
				<ActionIcon variant="subtle" size="sm" onClick={open} aria-label={t("branches.title")}>
					<CloudArrowDownIcon size={16} />
				</ActionIcon>
			</Tooltip>
			<RepositoryBranchesModal opened={opened} onClose={close} project={project} />
		</>
	);
}
