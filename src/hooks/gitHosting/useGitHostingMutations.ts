import { useMutation, useQueryClient } from "@tanstack/react-query";
import { commands, type HostKindView } from "@/bindings";
import { SHOWS_ITS_OWN_ERROR } from "@/infra/queryClient";
import { GIT_HOSTING_QUERY_KEY } from "./gitHostingQueryKeys";

type TokenSignIn = {
	host: HostKindView;
	username: string;
	token: string;
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
