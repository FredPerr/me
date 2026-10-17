import { Button, Divider, Group, Stack, TabsPanel, Tooltip } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import {
	AppWindowIcon,
	GitBranchIcon,
	GitPullRequestIcon,
	ListChecksIcon,
	ShareNetworkIcon,
	SparkleIcon,
} from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProjectView } from "@/bindings";
import { BulkCreateContextsModal } from "@/components/contexts/BulkCreateContextsModal";
import { CreateContextsFromTasksModal } from "@/components/contexts/CreateContextsFromTasksModal";
import { CreateFromBranchesModal } from "@/components/contexts/CreateFromBranchesModal";
import { CreateFromPullRequestsModal } from "@/components/contexts/CreateFromPullRequestsModal";
import { FlowBoard } from "@/components/contexts/FlowBoard";
import { KanbanBoard } from "@/components/contexts/KanbanBoard";
import { SearchContextInput } from "@/components/contexts/SearchContextInput";
import { ShareForReviewModal } from "@/components/project-tabs/ShareForReviewModal";
import { GitRemoteLink } from "@/components/shared/GitRemoteLink";
import { PullContextsButton } from "@/components/shared/PullContextsButton";
import { useContextAgentRunSessions } from "@/hooks/agent-runs/useContextAgentRunSessions";
import { useAppSettings } from "@/hooks/useAppSettings";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import { useProject } from "@/hooks/workspace/useProject";

type ProjectTabPanelProps = {
	projectTag: string;
};

export function ProjectTabPanel({ projectTag }: ProjectTabPanelProps) {
	const { data: project } = useProject(projectTag);
	if (!project) return null;
	return <LoadedProjectTabPanel project={project} />;
}

function LoadedProjectTabPanel({ project }: { project: ProjectView }) {
	const { t } = useTranslation();
	const { settings } = useAppSettings();
	const { open: openInIde, isAvailable } = useOpenInIde();
	const { runAgent, stopAgentRun, isRunning, liveRunFor } = useContextAgentRunSessions(project);
	const [searchFilter, setSearchFilter] = useState("");
	const [fromBranchesOpened, { open: openFromBranches, close: closeFromBranches }] =
		useDisclosure(false);
	const [fromPullRequestsOpened, { open: openFromPullRequests, close: closeFromPullRequests }] =
		useDisclosure(false);
	const [bulkOpened, { open: openBulk, close: closeBulk }] = useDisclosure(false);
	const [shareOpened, { open: openShare, close: closeShare }] = useDisclosure(false);
	const [fromTasksOpened, { open: openFromTasks, close: closeFromTasks }] = useDisclosure(false);

	const hasLinkedProjects = project.remoteProjectLinks.length > 0;

	async function handleOpenRootInIde() {
		await openInIde(project.rootIdePath);
	}

	return (
		<TabsPanel
			value={project.tag}
			style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
		>
			<Stack gap="sm" py="md" style={{ flex: 1, minHeight: 0 }}>
				<Group justify="space-between">
					<Group gap="md">
						<SearchContextInput value={searchFilter} onChange={setSearchFilter} />
						<Tooltip label={t("contexts.fromTasks.buttonHint")}>
							<Button
								variant="default"
								leftSection={<ListChecksIcon size={16} />}
								size="xs"
								onClick={openFromTasks}
							>
								{t("contexts.fromTasks.button")}
							</Button>
						</Tooltip>
					</Group>
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
							leftSection={<GitPullRequestIcon size={16} />}
							size="xs"
							onClick={openFromPullRequests}
						>
							{t("contexts.createFromPullRequests")}
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
				{settings?.useFlowBoard ? (
					<FlowBoard
						project={project}
						onRunAgent={runAgent}
						onStopAgent={stopAgentRun}
						isRunning={isRunning}
						liveRunFor={liveRunFor}
						searchFilter={searchFilter}
						onCreateFromTasks={hasLinkedProjects ? openFromTasks : undefined}
						onBulkCreate={openBulk}
						onShareForReview={openShare}
						onCreateFromBranches={openFromBranches}
						onCreateFromPullRequests={openFromPullRequests}
					/>
				) : (
					<KanbanBoard
						project={project}
						onRunAgent={runAgent}
						onStopAgent={stopAgentRun}
						isRunning={isRunning}
						liveRunFor={liveRunFor}
						searchFilter={searchFilter}
						onCreateFromTasks={hasLinkedProjects ? openFromTasks : undefined}
					/>
				)}
			</Stack>
			<CreateFromBranchesModal
				opened={fromBranchesOpened}
				onClose={closeFromBranches}
				project={project}
			/>
			<CreateFromPullRequestsModal
				opened={fromPullRequestsOpened}
				onClose={closeFromPullRequests}
				project={project}
			/>
			<BulkCreateContextsModal opened={bulkOpened} onClose={closeBulk} project={project} />
			<ShareForReviewModal opened={shareOpened} onClose={closeShare} project={project} />
			{fromTasksOpened && (
				<CreateContextsFromTasksModal project={project} onClose={closeFromTasks} />
			)}
		</TabsPanel>
	);
}
