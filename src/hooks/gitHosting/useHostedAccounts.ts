import { useQuery } from "@tanstack/react-query";
import { commands, type HostKindView, type SignedInHostView } from "@/bindings";
import { HOSTED_ACCOUNTS_QUERY_KEY } from "./gitHostingQueryKeys";

const NO_SIGNED_IN_HOSTS: SignedInHostView[] = [];

/** The Git hosts signed in to, each with its account once Rust could read it. */
export function useHostedAccounts() {
	const query = useQuery({
		queryKey: HOSTED_ACCOUNTS_QUERY_KEY,
		queryFn: () => commands.gitHostingListAccounts(),
	});
	const signedInHosts = query.data ?? NO_SIGNED_IN_HOSTS;
	return {
		signedInHosts,
		findSignedInHost: (host: HostKindView) =>
			signedInHosts.find((signedIn) => signedIn.host === host) ?? null,
		loading: query.isPending,
		error: query.error,
	};
}
