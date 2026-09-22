import { invoke } from "@tauri-apps/api/core";
import type {
	DeviceAuthorization,
	DevicePollResult,
	GitProvider,
	ProviderPullRequest,
	ProviderRepository,
	ProviderUser,
	RepositoryRef,
} from "./GitProvider";

const GITHUB_HOST = "github.com";

/**
 * GitHub implementation of the {@link GitProvider} port. All network calls and
 * token storage happen in the Rust backend; this class only marshals the Tauri
 * commands so no access token ever crosses into the JS layer.
 */
export class GitHubProvider implements GitProvider {
	readonly id = "github" as const;

	startDeviceAuthorization(): Promise<DeviceAuthorization> {
		return invoke<DeviceAuthorization>("github_start_device_authorization");
	}

	pollForAccessToken(deviceCode: string): Promise<DevicePollResult> {
		return invoke<DevicePollResult>("github_poll_for_access_token", { deviceCode });
	}

	isConnected(): Promise<boolean> {
		return invoke<boolean>("github_is_connected");
	}

	getAuthenticatedUser(): Promise<ProviderUser> {
		return invoke<ProviderUser>("github_get_authenticated_user");
	}

	listRepositories(): Promise<ProviderRepository[]> {
		return invoke<ProviderRepository[]>("github_list_repositories");
	}

	listPullRequests(repository: RepositoryRef): Promise<ProviderPullRequest[]> {
		return invoke<ProviderPullRequest[]>("github_list_pull_requests", {
			owner: repository.owner,
			repo: repository.repo,
		});
	}

	parseRepositoryRef(remoteUrl: string): RepositoryRef | null {
		return parseGitHubRepositoryRef(remoteUrl);
	}

	disconnect(): Promise<void> {
		return invoke<void>("github_disconnect");
	}
}

/**
 * Extract `owner`/`repo` from a GitHub remote URL. Accepts the normalized
 * `https://github.com/owner/repo` form the backend produces, as well as raw
 * HTTPS/SSH URLs with an optional `.git` suffix. Returns null for non-GitHub
 * URLs or URLs missing an owner/repo pair.
 */
export function parseGitHubRepositoryRef(remoteUrl: string): RepositoryRef | null {
	const trimmed = remoteUrl.trim();
	if (!trimmed.includes(GITHUB_HOST)) {
		return null;
	}

	const afterHost = trimmed.split(GITHUB_HOST)[1];
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
