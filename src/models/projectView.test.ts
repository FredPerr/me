import { describe, expect, it } from "vitest";
import {
	API_REPOSITORY_ID,
	buildContext,
	buildContextBranch,
	buildDefaultContext,
	buildProject,
	buildRepository,
	MAIN_BRANCH,
	PROJECT_PATH,
	WEB_REPOSITORY_ID,
} from "@/test-utils/workspaceBuilders";
import {
	collectWorktreePaths,
	findDefaultBranch,
	isCreatedFromTask,
	newBranchesFrom,
} from "./projectView";

const CONNECTION_ID = "teamwork:acme.teamwork.com";
const TASK_ID = "42";
const OTHER_TASK_ID = "43";
const NEW_BRANCH = "feature/login";
const BASE_BRANCH = "feature/auth";
const API_WORKTREE = `${PROJECT_PATH}/.worktrees/feature-login/api`;

describe("isCreatedFromTask", () => {
	const contextFromTask = buildContext({
		origin: { kind: "fromTask", connectionId: CONNECTION_ID, taskId: TASK_ID },
	});

	it("recognises the task a context was created from", () => {
		expect(
			isCreatedFromTask(contextFromTask, { connectionId: CONNECTION_ID, taskId: TASK_ID }),
		).toBe(true);
	});

	it("tells another task apart", () => {
		expect(
			isCreatedFromTask(contextFromTask, { connectionId: CONNECTION_ID, taskId: OTHER_TASK_ID }),
		).toBe(false);
	});

	it("tells a context made by hand apart", () => {
		expect(
			isCreatedFromTask(buildContext(), { connectionId: CONNECTION_ID, taskId: TASK_ID }),
		).toBe(false);
	});
});

describe("findDefaultBranch", () => {
	it("is the default context's branch in the repository", () => {
		const project = buildProject({
			contexts: [buildDefaultContext([buildContextBranch({ branch: MAIN_BRANCH })])],
		});

		expect(findDefaultBranch(project, API_REPOSITORY_ID)).toBe(MAIN_BRANCH);
	});
});

describe("newBranchesFrom", () => {
	const repositories = [
		buildRepository(),
		buildRepository({ id: WEB_REPOSITORY_ID, name: "web", path: `${PROJECT_PATH}/web` }),
	];
	const defaultContext = buildDefaultContext([
		buildContextBranch({ repositoryId: API_REPOSITORY_ID }),
		buildContextBranch({ repositoryId: WEB_REPOSITORY_ID }),
	]);
	const baseContext = buildContext({
		branches: [buildContextBranch({ repositoryId: API_REPOSITORY_ID, branch: BASE_BRANCH })],
	});
	const project = buildProject({ repositories, contexts: [defaultContext, baseContext] });

	it("starts each new branch from the base context's branch", () => {
		const [api] = newBranchesFrom(project, baseContext, NEW_BRANCH);

		expect(api).toEqual({
			repositoryId: API_REPOSITORY_ID,
			branch: { kind: "newBranch", name: NEW_BRANCH, baseBranch: BASE_BRANCH },
		});
	});

	it("falls back to the default branch where the base context has none", () => {
		const [, web] = newBranchesFrom(project, baseContext, NEW_BRANCH);

		expect(web.branch).toEqual({ kind: "newBranch", name: NEW_BRANCH, baseBranch: MAIN_BRANCH });
	});

	it("starts from the default branches without a base context", () => {
		const branches = newBranchesFrom(project, undefined, NEW_BRANCH);

		expect(branches.map(({ branch }) => branch)).toEqual([
			{ kind: "newBranch", name: NEW_BRANCH, baseBranch: MAIN_BRANCH },
			{ kind: "newBranch", name: NEW_BRANCH, baseBranch: MAIN_BRANCH },
		]);
	});
});

describe("collectWorktreePaths", () => {
	it("lists every context checkout once", () => {
		const checkout = buildContextBranch({ worktreePath: API_WORKTREE });
		const project = buildProject({
			contexts: [
				buildContext({ branches: [checkout] }),
				buildContext({ id: "context-2", branches: [checkout] }),
			],
		});

		expect(collectWorktreePaths(project)).toEqual([API_WORKTREE]);
	});

	it("skips branches of repositories no longer in the project", () => {
		const project = buildProject({
			contexts: [buildContext({ branches: [buildContextBranch({ worktreePath: null })] })],
		});

		expect(collectWorktreePaths(project)).toEqual([]);
	});
});
