import type { TaskRef } from "@/models/projectView";

/**
 * One context to create in a bulk operation. Every repository gets the same
 * new branch, started from the base context's branch (or the repository's
 * default branch). Without `branchName` the branch is named after the context.
 */
export type BulkContextSpec = {
	contextName: string;
	branchName?: string;
	agentPrompt?: string;
	task?: TaskRef;
};
