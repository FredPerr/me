import { invoke } from "@tauri-apps/api/core";
import type {
	DeviceAuthorization,
	DevicePollResult,
	GitProvider,
	ProviderRepository,
	ProviderUser,
} from "./GitProvider";

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

	disconnect(): Promise<void> {
		return invoke<void>("github_disconnect");
	}
}
