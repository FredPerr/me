import type { Page } from "@/domain/work-tracking/Page";
import type { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";

export enum PagedListStatus {
	Idle = "idle",
	Loading = "loading",
	LoadingMore = "loadingMore",
	Ready = "ready",
	Error = "error",
}

export enum PagedListActionType {
	Reset = "reset",
	LoadStarted = "loadStarted",
	LoadMoreStarted = "loadMoreStarted",
	PageLoaded = "pageLoaded",
	LoadFailed = "loadFailed",
}

export type PagedListState<T> = {
	readonly items: readonly T[];
	readonly nextCursor: string | undefined;
	readonly status: PagedListStatus;
	readonly error: WorkTrackingError | null;
};

type PagedListAction<T> =
	| { type: PagedListActionType.Reset }
	| { type: PagedListActionType.LoadStarted }
	| { type: PagedListActionType.LoadMoreStarted }
	| { type: PagedListActionType.PageLoaded; page: Page<T>; append: boolean }
	| { type: PagedListActionType.LoadFailed; error: WorkTrackingError };

export function createInitialPagedListState<T>(): PagedListState<T> {
	return { items: [], nextCursor: undefined, status: PagedListStatus.Idle, error: null };
}

export function pagedListReducer<T>(
	state: PagedListState<T>,
	action: PagedListAction<T>,
): PagedListState<T> {
	switch (action.type) {
		case PagedListActionType.Reset:
			return createInitialPagedListState();
		case PagedListActionType.LoadStarted:
			return { items: [], nextCursor: undefined, status: PagedListStatus.Loading, error: null };
		case PagedListActionType.LoadMoreStarted:
			return { ...state, status: PagedListStatus.LoadingMore, error: null };
		case PagedListActionType.PageLoaded:
			return {
				items: action.append ? [...state.items, ...action.page.items] : action.page.items,
				nextCursor: action.page.nextCursor,
				status: PagedListStatus.Ready,
				error: null,
			};
		case PagedListActionType.LoadFailed:
			return { ...state, status: PagedListStatus.Error, error: action.error };
	}
}

export function hasMorePages<T>(state: PagedListState<T>): boolean {
	const settled =
		state.status === PagedListStatus.Ready || state.status === PagedListStatus.LoadingMore;
	return settled && state.nextCursor !== undefined;
}
