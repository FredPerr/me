import { describe, expect, it } from "vitest";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { WorkTrackingErrorKind } from "@/domain/work-tracking/WorkTrackingErrorKind";
import {
	createInitialPagedListState,
	hasMorePages,
	PagedListActionType,
	type PagedListState,
	PagedListStatus,
	pagedListReducer,
} from "./pagedListState";

function loadedState(items: string[], nextCursor?: string): PagedListState<string> {
	const loading = pagedListReducer(createInitialPagedListState<string>(), {
		type: PagedListActionType.LoadStarted,
	});
	return pagedListReducer(loading, {
		type: PagedListActionType.PageLoaded,
		page: { items, ...(nextCursor !== undefined && { nextCursor }) },
		append: false,
	});
}

const NETWORK_ERROR = new WorkTrackingError(WorkTrackingErrorKind.Network, "offline");

describe("pagedListReducer", () => {
	it("starts idle and empty", () => {
		const state = createInitialPagedListState<string>();

		expect(state).toEqual({
			items: [],
			nextCursor: undefined,
			status: PagedListStatus.Idle,
			error: null,
		});
		expect(hasMorePages(state)).toBe(false);
	});

	it("replaces the items when a first page loads", () => {
		const first = loadedState(["a", "b"], "2");

		const reloaded = pagedListReducer(
			pagedListReducer(first, { type: PagedListActionType.LoadStarted }),
			{ type: PagedListActionType.PageLoaded, page: { items: ["c"] }, append: false },
		);

		expect(reloaded.items).toEqual(["c"]);
		expect(reloaded.status).toBe(PagedListStatus.Ready);
		expect(reloaded.nextCursor).toBeUndefined();
	});

	it("clears the items when a first-page load starts", () => {
		const loading = pagedListReducer(loadedState(["a"], "2"), {
			type: PagedListActionType.LoadStarted,
		});

		expect(loading.items).toEqual([]);
		expect(loading.status).toBe(PagedListStatus.Loading);
		expect(hasMorePages(loading)).toBe(false);
	});

	it("appends the next page and keeps the items while loading more", () => {
		const loadingMore = pagedListReducer(loadedState(["a", "b"], "2"), {
			type: PagedListActionType.LoadMoreStarted,
		});
		expect(loadingMore.items).toEqual(["a", "b"]);
		expect(loadingMore.status).toBe(PagedListStatus.LoadingMore);

		const appended = pagedListReducer(loadingMore, {
			type: PagedListActionType.PageLoaded,
			page: { items: ["c"], nextCursor: "3" },
			append: true,
		});

		expect(appended.items).toEqual(["a", "b", "c"]);
		expect(appended.nextCursor).toBe("3");
		expect(appended.status).toBe(PagedListStatus.Ready);
	});

	it("reports more pages only when a cursor is left", () => {
		expect(hasMorePages(loadedState(["a"], "2"))).toBe(true);
		expect(hasMorePages(loadedState(["a"]))).toBe(false);
	});

	it("keeps the loaded items and cursor when a load fails", () => {
		const failed = pagedListReducer(
			pagedListReducer(loadedState(["a"], "2"), { type: PagedListActionType.LoadMoreStarted }),
			{ type: PagedListActionType.LoadFailed, error: NETWORK_ERROR },
		);

		expect(failed.status).toBe(PagedListStatus.Error);
		expect(failed.error).toBe(NETWORK_ERROR);
		expect(failed.items).toEqual(["a"]);
		expect(failed.nextCursor).toBe("2");
		expect(hasMorePages(failed)).toBe(false);
	});

	it("clears the error when the request is retried", () => {
		const failed = pagedListReducer(loadedState(["a"], "2"), {
			type: PagedListActionType.LoadFailed,
			error: NETWORK_ERROR,
		});

		const retrying = pagedListReducer(failed, { type: PagedListActionType.LoadMoreStarted });

		expect(retrying.error).toBeNull();
		expect(retrying.status).toBe(PagedListStatus.LoadingMore);
	});

	it("resets from Error to Idle with no items and no error", () => {
		const failed = pagedListReducer(loadedState(["a"], "2"), {
			type: PagedListActionType.LoadFailed,
			error: NETWORK_ERROR,
		});

		const reset = pagedListReducer(failed, { type: PagedListActionType.Reset });

		expect(reset).toEqual(createInitialPagedListState());
	});
});
