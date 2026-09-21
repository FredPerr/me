import { Button, Divider, Group, Stack, TabsPanel } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { AppWindowIcon, GitBranchIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ContextsGrid } from "@/components/contexts/ContextsGrid";
import { CreateFromBranchesModal } from "@/components/contexts/CreateFromBranchesModal";
import { SearchContextInput } from "@/components/contexts/SearchContextInput";
import { GitRemoteLink } from "@/components/shared/GitRemoteLink";
import { PullContextsButton } from "@/components/shared/PullContextsButton";
import { useContexts } from "@/hooks/useContexts";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Project } from "@/models/Project";

type ProjectTabPanelProps = {
	project: Project;
};

export function ProjectTabPanel({ project }: ProjectTabPanelProps) {
	const { t } = useTranslation();
	const { open, isAvailable } = useOpenInIde();
	const { contexts, createContext, createContextFromBranches, deleteContext } =
		useContexts(project);
	const [searchFilter, setSearchFilter] = useState("");
	const [fromBranchesOpened, { open: openFromBranches, close: closeFromBranches }] =
		useDisclosure(false);

	return (
		<TabsPanel value={project.tag}>
			<Stack gap="sm" py="md">
				<Group justify="space-between">
					<SearchContextInput value={searchFilter} onChange={setSearchFilter} />
					<Group gap="xs">
						<GitRemoteLink project={project} />
						<PullContextsButton project={project} />
						<Button
							variant="default"
							leftSection={<GitBranchIcon size={16} />}
							size="xs"
							onClick={openFromBranches}
						>
							{t("contexts.createFromBranches")}
						</Button>
						<Button
							leftSection={<AppWindowIcon size={16} />}
							size="xs"
							onClick={() => open(project.path)}
							disabled={!project.path || !isAvailable}
						>
							{t("project.openInIde")}
						</Button>
					</Group>
				</Group>
				<Divider />
				<ContextsGrid
					contexts={contexts}
					project={project}
					onDelete={deleteContext}
					onCreate={createContext}
					searchFilter={searchFilter}
				/>
			</Stack>
			<CreateFromBranchesModal
				opened={fromBranchesOpened}
				onClose={closeFromBranches}
				project={project}
				onCreate={createContextFromBranches}
			/>
		</TabsPanel> // group under a tag for features
	);
}
