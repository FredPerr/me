import { notifications } from "@mantine/notifications";
import { MutationCache, QueryClient } from "@tanstack/react-query";
import i18n from "@/i18n";
import { describeError } from "@/infra/commandError";

/**
 * Queries call local Tauri commands, so failures are not retried and window
 * focus doesn't refetch: Rust announces every change with `project-changed`.
 * A failed change is shown as a notification.
 */
export function createQueryClient(): QueryClient {
	return new QueryClient({
		defaultOptions: {
			queries: { retry: false, refetchOnWindowFocus: false },
			mutations: { retry: false },
		},
		mutationCache: new MutationCache({
			onError: (error) => {
				notifications.show({
					title: i18n.t("workspace.changeFailed"),
					message: describeError(error),
					color: "red",
				});
			},
		}),
	});
}
