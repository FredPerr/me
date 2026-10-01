import { useCallback, useEffect, useReducer, useRef } from "react";
import type { Page } from "@/domain/work-tracking/Page";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import {
	createInitialPagedListState,
	hasMorePages,
	PagedListActionType,
	PagedListStatus,
	pagedListReducer,
} from "./pagedListState";

type FetchPage<T> = (cursor?: string) => Promise<Page<T>>;

type FailedRequest = { cursor: string | undefined };

export type PagedRemoteList<T> = {
	items: readonly T[];
	status: PagedListStatus;
	error: WorkTrackingError | null;
	hasMore: boolean;
	loadMore: () => void;
	retry: () => void;
};

function toWorkTrackingError(error: unknown): WorkTrackingError {
	return error instanceof WorkTrackingError ? error : WorkTrackingError.fromUnknown(error);
}

/**
 * Loads a cursor-paged remote list. Only a `resetKey` change (or `loadMore`/`retry`) triggers a
 * request; `fetchPage` is read from a ref so callers can pass inline lambdas. `fetchPage === null`
 * means nothing is selected and the list stays idle.
 */
export function usePagedRemoteList<T>(
	fetchPage: FetchPage<T> | null,
	resetKey: string,
): PagedRemoteList<T> {
	const [state, dispatch] = useReducer(
		pagedListReducer<T>,
		undefined,
		createInitialPagedListState<T>,
	);
	const fetchPageRef = useRef(fetchPage);
	fetchPageRef.current = fetchPage;
	const requestSequenceRef = useRef(0);
	const lastFailedRequestRef = useRef<FailedRequest | null>(null);

	const runRequest = useCallback(async (cursor: string | undefined) => {
		const currentFetchPage = fetchPageRef.current;
		if (currentFetchPage === null) return;
		requestSequenceRef.current += 1;
		const sequence = requestSequenceRef.current;
		const append = cursor !== undefined;
		lastFailedRequestRef.current = null;
		dispatch({
			type: append ? PagedListActionType.LoadMoreStarted : PagedListActionType.LoadStarted,
		});
		try {
			const page = await currentFetchPage(cursor);
			if (sequence !== requestSequenceRef.current) return;
			dispatch({ type: PagedListActionType.PageLoaded, page, append });
		} catch (error) {
			if (sequence !== requestSequenceRef.current) return;
			lastFailedRequestRef.current = { cursor };
			dispatch({ type: PagedListActionType.LoadFailed, error: toWorkTrackingError(error) });
		}
	}, []);

	// biome-ignore lint/correctness/useExhaustiveDependencies: resetKey is the only reload trigger by design
	useEffect(() => {
		lastFailedRequestRef.current = null;
		dispatch({ type: PagedListActionType.Reset });
		void runRequest(undefined);
		return () => {
			// Invalidates any in-flight response for the previous key (or after unmount).
			requestSequenceRef.current += 1;
		};
	}, [resetKey, runRequest]);

	const { nextCursor, status } = state;

	const loadMore = useCallback(() => {
		if (status !== PagedListStatus.Ready || nextCursor === undefined) return;
		void runRequest(nextCursor);
	}, [nextCursor, runRequest, status]);

	const retry = useCallback(() => {
		const failedRequest = lastFailedRequestRef.current;
		if (failedRequest === null) return;
		void runRequest(failedRequest.cursor);
	}, [runRequest]);

	return {
		items: state.items,
		status,
		error: state.error,
		hasMore: hasMorePages(state),
		loadMore,
		retry,
	};
}
