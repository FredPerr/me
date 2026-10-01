import { NavLink, Stack, Title } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { WorkItemGroupId } from "@/domain/work-tracking/identifiers";
import type { WorkItemGroup } from "@/domain/work-tracking/WorkItemGroup";
import type { PagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";
import { LoadMoreButton } from "./LoadMoreButton";
import { RemoteListState } from "./RemoteListState";

type WorkItemGroupListProps = {
	list: PagedRemoteList<WorkItemGroup>;
	selectedGroupId: WorkItemGroupId | null;
	onSelect: (groupId: WorkItemGroupId) => void;
	onReplaceKey: () => void;
};

export function WorkItemGroupList({
	list,
	selectedGroupId,
	onSelect,
	onReplaceKey,
}: WorkItemGroupListProps) {
	const { t } = useTranslation();

	return (
		<Stack gap="xs">
			<Title order={3}>{t("workTracking.groups.title")}</Title>
			<RemoteListState
				status={list.status}
				error={list.error}
				itemCount={list.items.length}
				emptyText={t("workTracking.groups.empty")}
				onRetry={list.retry}
				onReplaceKey={onReplaceKey}
			>
				<Stack component="ul" gap={2} p={0} m={0} style={{ listStyle: "none" }}>
					{list.items.map((group) => {
						const selected = group.id === selectedGroupId;
						return (
							<li key={group.id}>
								<NavLink
									component="button"
									type="button"
									active={selected}
									aria-current={selected ? "true" : undefined}
									label={group.name}
									onClick={() => onSelect(group.id)}
								/>
							</li>
						);
					})}
				</Stack>
				<LoadMoreButton list={list} />
			</RemoteListState>
		</Stack>
	);
}
