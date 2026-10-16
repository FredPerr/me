import { useMutation, useQueryClient } from "@tanstack/react-query";
import { commands, type HostKindView, type NewPullRequestInput } from "@/bindings";
import { SHOWS_ITS_OWN_ERROR } from "@/infra/queryClient";
import { GIT_HOSTING_QUERY_KEY } from "./gitHostingQueryKeys";
import { contextPullRequestsQueryKey } from "./useContextPullRequests";

type TokenSignIn = {
	host: HostKindView;
	username: string;
	token: string;
};

type PullRequestCreation = {
	repositoryId: string;
	input: NewPullRequestInput;
};

function useInvalidateGitHosting() {
	const queryClient = useQueryClient();
	return () => queryClient.invalidateQueries({ queryKey: GIT_HOSTING_QUERY_KEY });
}

/** The form shows a refused sign-in itself. */
export function useSignInWithToken() {
	const invalidateGitHosting = useInvalidateGitHosting();
	return useMutation({
		mutationFn: ({ host, username, token }: TokenSignIn) =>
			commands.gitHostingSignInWithToken(host, username, token),
		onSuccess: invalidateGitHosting,
		meta: { [SHOWS_ITS_OWN_ERROR]: true },
	});
}

export function useSignOut() {
	const invalidateGitHosting = useInvalidateGitHosting();
	return useMutation({
		mutationFn: (host: HostKindView) => commands.gitHostingSignOut(host),
		onSuccess: invalidateGitHosting,
	});
}

/**
 * Pushes a context branch and opens its pull request in one call. Rust already
 * forgot the branch's cached lookup, so refetching shows the new pull request
 * on the board. The modal shows a failure itself.
 */
export function useCreatePullRequest(projectTag: string) {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: ({ repositoryId, input }: PullRequestCreation) =>
			commands.gitHostingCreatePullRequest(projectTag, repositoryId, input),
		onSuccess: () =>
			queryClient.invalidateQueries({ queryKey: contextPullRequestsQueryKey(projectTag) }),
		meta: { [SHOWS_ITS_OWN_ERROR]: true },
	});
}
