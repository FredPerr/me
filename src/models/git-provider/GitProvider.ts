/**
 * Port for authenticating with a git hosting provider and reading the
 * resources the authenticated user can access.
 *
 * GitHub is the only implementation today. Adding GitLab or Bitbucket later is
 * a matter of providing another implementation of this interface — the UI and
 * hooks depend only on the port, not on any concrete provider.
 */

/** Stable identifier for a supported provider. */
export type GitProviderId = "github";

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

export interface GitProvider {
	readonly id: GitProviderId;

	/** Begin the device flow, returning the code to show the user. */
	startDeviceAuthorization(): Promise<DeviceAuthorization>;

	/** Poll once for the access token tied to a device code. */
	pollForAccessToken(deviceCode: string): Promise<DevicePollResult>;

	/** Whether an access token is currently stored for this provider. */
	isConnected(): Promise<boolean>;

	/** Fetch the authenticated user. Rejects if not connected. */
	getAuthenticatedUser(): Promise<ProviderUser>;

	/** List repositories accessible to the authenticated user. */
	listRepositories(): Promise<ProviderRepository[]>;

	/** Remove the stored credentials for this provider. */
	disconnect(): Promise<void>;
}
