import { QueryClient } from "@tanstack/react-query";
import { beforeEach, describe, expect, it } from "vitest";
import type { ProjectListView } from "@/bindings";
import { buildProject, PROJECT_TAG } from "@/test-utils/workspaceBuilders";
import { forgetProject, storeProject } from "./projectCache";
import { PROJECT_LIST_QUERY_KEY, projectQueryKey } from "./workspaceQueryKeys";

const OTHER_PROJECT_TAG = "billing";
const RENAMED_PROJECT_NAME = "Website (renamed)";

function buildListing(overrides?: Partial<ProjectListView>): ProjectListView {
	return {
		projects: [buildProject(), buildProject({ tag: OTHER_PROJECT_TAG })],
		unreadableProjects: [],
		...overrides,
	};
}

function listedTags(queryClient: QueryClient): string[] | undefined {
	return queryClient
		.getQueryData<ProjectListView>(PROJECT_LIST_QUERY_KEY)
		?.projects.map((project) => project.tag);
}

describe("storeProject", () => {
	let queryClient: QueryClient;

	beforeEach(() => {
		queryClient = new QueryClient();
		queryClient.setQueryData(PROJECT_LIST_QUERY_KEY, buildListing());
	});

	it("replaces the project in the project list", () => {
		storeProject(queryClient, buildProject({ name: RENAMED_PROJECT_NAME }));

		const listing = queryClient.getQueryData<ProjectListView>(PROJECT_LIST_QUERY_KEY);
		expect(listing?.projects[0].name).toBe(RENAMED_PROJECT_NAME);
		expect(listedTags(queryClient)).toEqual([PROJECT_TAG, OTHER_PROJECT_TAG]);
	});

	it("caches the project under its own key", () => {
		const project = buildProject({ name: RENAMED_PROJECT_NAME });

		storeProject(queryClient, project);

		expect(queryClient.getQueryData(projectQueryKey(PROJECT_TAG))).toEqual(project);
	});

	it("adds a new project to the list", () => {
		queryClient.setQueryData(PROJECT_LIST_QUERY_KEY, buildListing({ projects: [] }));

		storeProject(queryClient, buildProject());

		expect(listedTags(queryClient)).toEqual([PROJECT_TAG]);
	});
});

describe("forgetProject", () => {
	it("removes the project from the list and from its own key", () => {
		const queryClient = new QueryClient();
		queryClient.setQueryData(PROJECT_LIST_QUERY_KEY, buildListing());
		queryClient.setQueryData(projectQueryKey(PROJECT_TAG), buildProject());

		forgetProject(queryClient, PROJECT_TAG);

		expect(listedTags(queryClient)).toEqual([OTHER_PROJECT_TAG]);
		expect(queryClient.getQueryData(projectQueryKey(PROJECT_TAG))).toBeUndefined();
	});
});
