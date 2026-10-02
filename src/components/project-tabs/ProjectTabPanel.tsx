import { Button, Divider, Group, Stack, TabsPanel } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { AppWindowIcon, GitBranchIcon, ShareNetworkIcon, SparkleIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { BulkCreateContextsModal } from "@/components/contexts/BulkCreateContextsModal";
import { CreateFromBranchesModal } from "@/components/contexts/CreateFromBranchesModal";
import { KanbanBoard } from "@/components/contexts/KanbanBoard";
import { SearchContextInput } from "@/components/contexts/SearchContextInput";
import { ShareForReviewModal } from "@/components/project-tabs/ShareForReviewModal";
import { GitRemoteLink } from "@/components/shared/GitRemoteLink";
import { PullContextsButton } from "@/components/shared/PullContextsButton";
import { useContextKiroSessions } from "@/hooks/useContextKiroSessions";
import { useContexts } from "@/hooks/useContexts";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Context, Project } from "@/models/Project";

type ProjectTabPanelProps = {
	project: Project;
};

export function ProjectTabPanel({ project }: ProjectTabPanelProps) {
	const { t } = useTranslation();
	const { openFirstExisting, isAvailable } = useOpenInIde();
	const {
		contexts,
		createContext,
		createContextFromBranches,
		createContextsBulk,
		deleteContext,
		setContextStatus,
	} = useContexts(project);
	const { runKiro, killKiro, isRunning, getConversation } = useContextKiroSessions(
		project,
		setContextStatus,
	);
	const [searchFilter, setSearchFilter] = useState("");

	async function handleRunKiro(context: Context, prompt: string) {
		await runKiro({ context, prompt });
	}
	const [fromBranchesOpened, { open: openFromBranches, close: closeFromBranches }] =
		useDisclosure(false);
	const [bulkOpened, { open: openBulk, close: closeBulk }] = useDisclosure(false);
	const [shareOpened, { open: openShare, close: closeShare }] = useDisclosure(false);

	async function handleOpenRootInIde() {
		await openFirstExisting(await project.resolveRootIdePathCandidates());
	}

	return (
		<TabsPanel
			value={project.tag}
			style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
		>
			<Stack gap="sm" py="md" style={{ flex: 1, minHeight: 0 }}>
				<Group justify="space-between">
					<SearchContextInput value={searchFilter} onChange={setSearchFilter} />
					<Group gap="xs">
						<GitRemoteLink project={project} />
						<PullContextsButton project={project} />
						<Button
							variant="default"
							leftSection={<ShareNetworkIcon size={16} />}
							size="xs"
							onClick={openShare}
						>
							{t("review.shareButton")}
						</Button>
						<Button
							variant="default"
							leftSection={<GitBranchIcon size={16} />}
							size="xs"
							onClick={openFromBranches}
						>
							{t("contexts.createFromBranches")}
						</Button>
						<Button
							variant="default"
							leftSection={<SparkleIcon size={16} />}
							size="xs"
							onClick={openBulk}
						>
							{t("contexts.bulk.button")}
						</Button>
						<Button
							leftSection={<AppWindowIcon size={16} />}
							size="xs"
							onClick={handleOpenRootInIde}
							disabled={!project.path || !isAvailable}
						>
							{t("project.openInIde")}
						</Button>
					</Group>
				</Group>
				<Divider />
				<KanbanBoard
					contexts={contexts}
					project={project}
					onDelete={deleteContext}
					onCreate={createContext}
					onStatusChange={setContextStatus}
					onRunKiro={handleRunKiro}
					onKillKiro={killKiro}
					isKiroRunning={isRunning}
					getKiroConversation={getConversation}
					searchFilter={searchFilter}
				/>
			</Stack>
			<CreateFromBranchesModal
				opened={fromBranchesOpened}
				onClose={closeFromBranches}
				project={project}
				onCreate={createContextFromBranches}
			/>
			<BulkCreateContextsModal
				opened={bulkOpened}
				onClose={closeBulk}
				project={project}
				onCreate={createContextsBulk}
			/>
			<ShareForReviewModal opened={shareOpened} onClose={closeShare} project={project} />
		</TabsPanel> // group under a tag for features
	);
}
