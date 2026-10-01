import { Button } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { PagedListStatus } from "@/hooks/work-tracking/pagedListState";
import type { PagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";

type LoadMoreButtonProps<T> = {
	list: PagedRemoteList<T>;
};

export function LoadMoreButton<T>({ list }: LoadMoreButtonProps<T>) {
	const { t } = useTranslation();
	if (!list.hasMore) return null;
	const loadingMore = list.status === PagedListStatus.LoadingMore;

	return (
		<Button
			variant="subtle"
			size="xs"
			onClick={list.loadMore}
			loading={loadingMore}
			disabled={loadingMore}
		>
			{t("workTracking.loadMore")}
		</Button>
	);
}
