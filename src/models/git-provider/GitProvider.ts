/**
 * Port for authenticating with a git hosting provider and reading the
 * resources the authenticated user can access.
 *
 * GitHub is the only implementation today. Adding GitLab or Bitbucket later is
 * a matter of providing another implementation of this interface — the UI and
 * hooks depend only on the port, not on any concrete provider.
 */

/** Stable identifier for a supported provider. */
export type GitProviderId = "github" | "bitbucket";

/**
 * How a provider authenticates the user.
 *
 * - `deviceFlow`: OAuth Device Flow — the user authorizes in the browser and we
 *   poll for a token (GitHub).
 * - `token`: the user supplies a credential directly, e.g. a Bitbucket App
 *   Password entered as `username` + `token` (Bitbucket).
 */
export type AuthKind = "deviceFlow" | "token";

/** Credentials entered by the user for a `token` auth provider. */
export type TokenCredentials = {
	/** Account username the token belongs to (used as the Basic-auth user). */
	username: string;
	/** The app password / API token acting as the Basic-auth password. */
	token: string;
};

/**
 * Verification details returned when starting the OAuth device flow. The user
 * enters `userCode` at `verificationUri` to authorize the app.
 */
export type DeviceAuthorization = {
	deviceCode: string;
	userCode: string;
	verificationUri: string;
	/** Lifetime of the device code, in seconds. */
	expiresIn: number;
	/** Minimum seconds to wait between polls. */
	interval: number;
};

/** Outcome of a single poll for the access token during the device flow. */
export type DevicePollResult =
	| { status: "authorized" }
	| { status: "pending" }
	| { status: "slowDown"; interval: number }
	| { status: "expired" }
	| { status: "denied" };

/** The authenticated user, as shown in the connected-account UI. */
export type ProviderUser = {
	login: string;
	name: string | null;
	avatarUrl: string | null;
};

/** A repository the authenticated user has access to. */
export type ProviderRepository = {
	fullName: string;
	name: string;
	private: boolean;
	cloneUrl: string;
	defaultBranch: string;
};

/** Identifies a repository on a provider by owner and name. */
export type RepositoryRef = {
	owner: string;
	repo: string;
};

/** An open pull request as shown on the pull-requests page. */
export type ProviderPullRequest = {
	number: number;
	title: string;
	htmlUrl: string;
	author: string | null;
	headBranch: string;
	baseBranch: string;
	draft: boolean;
	updatedAt: string;
};

/** The details needed to open a new pull request. */
export type CreatePullRequestInput = {
	title: string;
	body: string;
	/** Branch containing the changes. Must already be pushed to the remote. */
	head: string;
	/** Branch the changes are merged into. */
	base: string;
	draft: boolean;
};

/** A pull request that was just created, used to link to it in the UI. */
export type CreatedPullRequest = {
	number: number;
	htmlUrl: string;
};

/**
 * A provider that authenticates via OAuth Device Flow (GitHub). The user
 * authorizes in the browser and we poll for the resulting access token.
 */
export interface DeviceFlowAuth {
	readonly authKind: "deviceFlow";

	/** Begin the device flow, returning the code to show the user. */
	startDeviceAuthorization(): Promise<DeviceAuthorization>;

	/** Poll once for the access token tied to a device code. */
	pollForAccessToken(deviceCode: string): Promise<DevicePollResult>;
}

/**
 * A provider that authenticates with a credential the user supplies directly
 * (Bitbucket App Password). The credential is validated by fetching the
 * authenticated user before it is stored.
 */
export interface TokenAuth {
	readonly authKind: "token";

	/**
	 * Validate and store the supplied credentials. Rejects if the credentials
	 * are invalid (the provider could not identify the user).
	 */
	connectWithToken(credentials: TokenCredentials): Promise<void>;
}

/** The auth capability of a provider, discriminated by {@link AuthKind}. */
export type ProviderAuth = DeviceFlowAuth | TokenAuth;

/** Narrow a provider's auth capability to the device flow. */
export function isDeviceFlowAuth(auth: ProviderAuth): auth is DeviceFlowAuth {
	return auth.authKind === "deviceFlow";
}

/** Narrow a provider's auth capability to direct token entry. */
export function isTokenAuth(auth: ProviderAuth): auth is TokenAuth {
	return auth.authKind === "token";
}

export interface GitProvider {
	readonly id: GitProviderId;

	/** Display name shown in the connected-accounts UI. */
	readonly displayName: string;

	/** How this provider authenticates (device flow vs. direct token). */
	readonly auth: ProviderAuth;

	/** Whether an access token is currently stored for this provider. */
	isConnected(): Promise<boolean>;

	/** Fetch the authenticated user. Rejects if not connected. */
	getAuthenticatedUser(): Promise<ProviderUser>;

	/** List repositories accessible to the authenticated user. */
	listRepositories(): Promise<ProviderRepository[]>;

	/** List open pull requests for a repository. */
	listPullRequests(repository: RepositoryRef): Promise<ProviderPullRequest[]>;

	/** Open a new pull request. The head branch must already be pushed. */
	createPullRequest(
		repository: RepositoryRef,
		input: CreatePullRequestInput,
	): Promise<CreatedPullRequest>;

	/**
	 * Parse a normalized remote URL into an owner/repo ref, or null if the URL
	 * does not belong to this provider.
	 */
	parseRepositoryRef(remoteUrl: string): RepositoryRef | null;

	/** Remove the stored credentials for this provider. */
	disconnect(): Promise<void>;
}
