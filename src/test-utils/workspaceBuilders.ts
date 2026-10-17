import type { ContextBranchView, ContextView, ProjectView, RepositoryView } from "@/bindings";

export const PROJECT_TAG = "website";
export const PROJECT_PATH = "/projects/website";
export const API_REPOSITORY_ID = "api";
export const WEB_REPOSITORY_ID = "web";
const DEFAULT_CONTEXT_ID = "context-default";
export const MAIN_BRANCH = "main";

export function buildRepository(overrides?: Partial<RepositoryView>): RepositoryView {
	return {
		id: API_REPOSITORY_ID,
		name: "api",
		relativePath: "./api",
		postCheckoutCommand: null,
		path: `${PROJECT_PATH}/api`,
		...overrides,
	};
}

export function buildContextBranch(overrides?: Partial<ContextBranchView>): ContextBranchView {
	return {
		repositoryId: API_REPOSITORY_ID,
		branch: MAIN_BRANCH,
		checkout: "worktree",
		worktreePath: `${PROJECT_PATH}/api`,
		pullRequestDraft: null,
		pullRequestMatched: null,
		...overrides,
	};
}

export function buildContext(overrides?: Partial<ContextView>): ContextView {
	const folder = overrides?.folder ?? "feature-login";
	return {
		id: "context-1",
		name: "Feature login",
		folder,
		folderPath: `${PROJECT_PATH}/.worktrees/${folder}`,
		idePath: `${PROJECT_PATH}/.worktrees/${folder}`,
		status: "ready",
		isDefault: false,
		isStatic: false,
		baseContextId: null,
		branches: [],
		origin: { kind: "manual" },
		agentWork: null,
		...overrides,
	};
}

export function buildDefaultContext(branches: ContextBranchView[]): ContextView {
	return buildContext({
		id: DEFAULT_CONTEXT_ID,
		name: "default",
		folder: "default",
		folderPath: PROJECT_PATH,
		idePath: PROJECT_PATH,
		status: "idle",
		isDefault: true,
		isStatic: true,
		branches,
	});
}

export function buildProject(overrides?: Partial<ProjectView>): ProjectView {
	const repositories = overrides?.repositories ?? [buildRepository()];
	return {
		tag: PROJECT_TAG,
		name: "Website",
		path: PROJECT_PATH,
		icon: null,
		repositories,
		effectiveRepositories: repositories,
		contexts: [],
		symlinks: [],
		remoteProjectLinks: [],
		branchNaming: { prefixes: [], allowsNoPrefix: true, defaultPrefix: null },
		rootIdePath: PROJECT_PATH,
		...overrides,
	};
}
