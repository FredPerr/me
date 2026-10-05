import { Badge, Group, Paper, Stack, Text } from "@mantine/core";
import { CubeIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { PullRequestLookup } from "@/hooks/useContextPullRequestStatus";
import type { CreateContextParams } from "@/hooks/useContexts";
import type { Context, Project } from "@/models/Project";
import { ContextCard } from "./ContextCard";

export type FlowBoardStaticData = {
	contexts: Context[];
	project: Project;
	allContexts: Context[];
	pullRequestLookup?: PullRequestLookup;
	onDelete: (contextId: string) => void;
	onCreate: (params: CreateContextParams) => Promise<void>;
	onExpandedChange: (contextId: string, expanded: boolean) => void;
	onStaticChange: (contextId: string, isStatic: boolean) => void;
	onOpenKiro: (context: Context) => void;
	isKiroRunning: (contextId: string) => boolean;
	getKiroHistoryCount: (contextId: string) => number;
};

export function FlowBoardStaticNode({ data }: { data: FlowBoardStaticData }) {
	const { t } = useTranslation();
	const {
		contexts,
		project,
		allContexts,
		pullRequestLookup,
		onDelete,
		onCreate,
		onExpandedChange,
		onStaticChange,
		onOpenKiro,
		isKiroRunning,
		getKiroHistoryCount,
	} = data;

	return (
		<Stack gap="xs" style={{ width: 720 }}>
			<Group gap="xs" px={4}>
				<CubeIcon size={14} color="var(--mantine-color-teal-4)" />
				<Text size="xs" fw={700} tt="uppercase" c="teal.4">
					{t("contexts.static.columnTitle")}
				</Text>
				<Badge size="sm" variant="light" color="teal">
					{contexts.length}
				</Badge>
			</Group>
			<Paper
				withBorder
				radius="md"
				p={8}
				style={{
					borderTop: "2px solid var(--mantine-color-teal-6)",
					backgroundColor: "var(--mantine-color-dark-7)",
				}}
			>
				{contexts.length === 0 ? (
					<Text size="xs" c="dimmed" ta="center" py="md">
						{t("contexts.static.columnEmpty")}
					</Text>
				) : (
					<Group align="flex-start" gap={8} wrap="wrap">
						{contexts.map((context) => (
							<div key={context.id} style={{ width: 340 }}>
								<ContextCard
									context={context}
									project={project}
									allContexts={allContexts}
									onDelete={onDelete}
									onCreate={onCreate}
									pullRequestLookup={pullRequestLookup}
									onExpandedChange={onExpandedChange}
									onStaticChange={onStaticChange}
									onOpenKiro={onOpenKiro}
									isKiroRunning={isKiroRunning(context.id)}
									hasKiroHistory={getKiroHistoryCount(context.id) > 0}
								/>
							</div>
						))}
					</Group>
				)}
			</Paper>
		</Stack>
	);
}
