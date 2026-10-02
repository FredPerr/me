import { invoke } from "@tauri-apps/api/core";
import type {
	CreatedPullRequest,
	CreatePullRequestInput,
	GitProvider,
	ProviderPullRequest,
	ProviderRepository,
	ProviderUser,
	RepositoryRef,
	TokenAuth,
	TokenCredentials,
} from "./GitProvider";

const BITBUCKET_HOST = "bitbucket.org";

/**
 * Bitbucket Cloud implementation of the {@link GitProvider} port.
 *
 * Bitbucket has no OAuth device flow, so the user supplies an App Password
 * (username + token) that is validated and stored by the Rust backend. All
 * network calls and credential storage happen there; this class only marshals
 * the Tauri commands so the credential never crosses into the JS layer.
 *
 * On Bitbucket a repository is addressed by `workspace/repo_slug`; those map
 * onto the shared {@link RepositoryRef} `owner`/`repo` fields respectively.
 */
export class BitbucketProvider implements GitProvider {
	readonly id = "bitbucket" as const;

	readonly displayName = "Bitbucket";

	readonly auth: TokenAuth = {
		authKind: "token",
		async connectWithToken(credentials: TokenCredentials) {
			await invoke<void>("bitbucket_connect_with_token", {
				username: credentials.username,
				token: credentials.token,
			});
		},
	};

	isConnected(): Promise<boolean> {
		return invoke<boolean>("bitbucket_is_connected");
	}

	getAuthenticatedUser(): Promise<ProviderUser> {
		return invoke<ProviderUser>("bitbucket_get_authenticated_user");
	}

	listRepositories(): Promise<ProviderRepository[]> {
		return invoke<ProviderRepository[]>("bitbucket_list_repositories");
	}

	listPullRequests(repository: RepositoryRef): Promise<ProviderPullRequest[]> {
		return invoke<ProviderPullRequest[]>("bitbucket_list_pull_requests", {
			workspace: repository.owner,
			repoSlug: repository.repo,
		});
	}

	createPullRequest(
		repository: RepositoryRef,
		input: CreatePullRequestInput,
	): Promise<CreatedPullRequest> {
		return invoke<CreatedPullRequest>("bitbucket_create_pull_request", {
			workspace: repository.owner,
			repoSlug: repository.repo,
			title: input.title,
			body: input.body,
			head: input.head,
			base: input.base,
			draft: input.draft,
		});
	}

	parseRepositoryRef(remoteUrl: string): RepositoryRef | null {
		return parseBitbucketRepositoryRef(remoteUrl);
	}

	disconnect(): Promise<void> {
		return invoke<void>("bitbucket_disconnect");
	}
}

/**
 * Extract `workspace`/`repo_slug` from a Bitbucket remote URL, returned as
 * `owner`/`repo`. Accepts the normalized `https://bitbucket.org/workspace/repo`
 * form as well as raw HTTPS/SSH URLs with an optional `.git` suffix. Returns
 * null for non-Bitbucket URLs or URLs missing a workspace/repo pair.
 */
export function parseBitbucketRepositoryRef(remoteUrl: string): RepositoryRef | null {
	const trimmed = remoteUrl.trim();
	if (!trimmed.includes(BITBUCKET_HOST)) {
		return null;
	}

	const afterHost = trimmed.split(BITBUCKET_HOST)[1];
	if (!afterHost) {
		return null;
	}

	// Strip the leading separator (":" for SSH, "/" for HTTPS) and any trailing
	// ".git" or slashes, then take the first two path segments.
	const path = afterHost
		.replace(/^[:/]+/, "")
		.replace(/\.git$/, "")
		.replace(/\/+$/, "");
	const segments = path.split("/").filter((segment) => segment.length > 0);
	if (segments.length < 2) {
		return null;
	}

	return { owner: segments[0], repo: segments[1] };
}
