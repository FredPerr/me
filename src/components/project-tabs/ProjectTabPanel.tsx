import {
	Button,
	Checkbox,
	Divider,
	Group,
	Loader,
	Stack,
	TabsPanel,
	Text,
	Tooltip,
} from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import {
	AppWindowIcon,
	GitBranchIcon,
	GitPullRequestIcon,
	ShareNetworkIcon,
	SparkleIcon,
} from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { BulkCreateContextsModal } from "@/components/contexts/BulkCreateContextsModal";
import { CreateFromBranchesModal } from "@/components/contexts/CreateFromBranchesModal";
import { CreateFromPullRequestsModal } from "@/components/contexts/CreateFromPullRequestsModal";
import { KanbanBoard } from "@/components/contexts/KanbanBoard";
import { SearchContextInput } from "@/components/contexts/SearchContextInput";
import { ShareForReviewModal } from "@/components/project-tabs/ShareForReviewModal";
import { GitRemoteLink } from "@/components/shared/GitRemoteLink";
import { PullContextsButton } from "@/components/shared/PullContextsButton";
import { translateError } from "@/components/work-tracking/translateError";
import { useContextKiroSessions } from "@/hooks/useContextKiroSessions";
import type { BulkContextSpec } from "@/hooks/useContexts";
import { useContexts } from "@/hooks/useContexts";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import { useLinkedProjectTasks } from "@/hooks/work-tracking/useLinkedProjectTasks";
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
		createContextFromPullRequests,
		createContextsBulk,
		deleteContext,
		setContextStatus,
		setContextExpanded,
	} = useContexts(project);
	const { runKiro, killKiro, isRunning, getConversation } = useContextKiroSessions(
		project,
		setContextStatus,
	);
	const [searchFilter, setSearchFilter] = useState("");
	const [showTasks, setShowTasks] = useState(false);
	const hasLinkedProjects = project.remoteProjectLinks.toArray().length > 0;
	const linkedTasks = useLinkedProjectTasks(
		project.remoteProjectLinks,
		showTasks && hasLinkedProjects,
	);

	async function handleCreateFromTask(spec: BulkContextSpec, baseContext: Context) {
		await createContextsBulk([spec], baseContext);
	}

	async function handleRunKiro(context: Context, prompt: string) {
		await runKiro({ context, prompt });
	}
	const [fromBranchesOpened, { open: openFromBranches, close: closeFromBranches }] =
		useDisclosure(false);
	const [fromPullRequestsOpened, { open: openFromPullRequests, close: closeFromPullRequests }] =
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
					<Group gap="md">
						<SearchContextInput value={searchFilter} onChange={setSearchFilter} />
						<Tooltip label={t("contexts.kanban.showTasksHint")}>
							<Checkbox
								size="xs"
								label={t("contexts.kanban.showTasks")}
								checked={showTasks}
								onChange={(event) => setShowTasks(event.currentTarget.checked)}
							/>
						</Tooltip>
						{linkedTasks.loading && <Loader size="xs" />}
						{showTasks && !hasLinkedProjects && (
							<Text size="xs" c="dimmed">
								{t("contexts.kanban.noLinkedProjects")}
							</Text>
						)}
						{showTasks && linkedTasks.error && (
							<Text size="xs" c="red">
								{t("contexts.kanban.tasksLoadError", {
									message: translateError(t, linkedTasks.error),
								})}
							</Text>
						)}
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
				<KanbanBoard
					contexts={contexts}
					project={project}
					onDelete={deleteContext}
					onCreate={createContext}
					onStatusChange={setContextStatus}
					onExpandedChange={setContextExpanded}
					onRunKiro={handleRunKiro}
					onKillKiro={killKiro}
					isKiroRunning={isRunning}
					getKiroConversation={getConversation}
					searchFilter={searchFilter}
					tasks={linkedTasks.tasks}
					onCreateFromTask={handleCreateFromTask}
				/>
			</Stack>
			<CreateFromBranchesModal
				opened={fromBranchesOpened}
				onClose={closeFromBranches}
				project={project}
				onCreate={createContextFromBranches}
			/>
			<CreateFromPullRequestsModal
				opened={fromPullRequestsOpened}
				onClose={closeFromPullRequests}
				project={project}
				onCreate={createContextFromPullRequests}
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
