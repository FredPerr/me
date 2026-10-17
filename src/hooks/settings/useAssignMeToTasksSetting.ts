import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { type AppSettingsView, commands } from "@/bindings";
import { SETTINGS_QUERY_KEY } from "./settingsQueryKeys";

/** Checked by default so assigning yourself is the easy path (spec #69, story 13). */
const DEFAULT_ASSIGN_ME_TO_TASKS = true;

/**
 * The remembered "Assign me to these tasks" preference, with a setter that
 * writes it back so it is remembered next time. Falls back to the default while
 * loading or when the store cannot be read, so the checkbox always has a value.
 */
export function useAssignMeToTasksSetting() {
	const queryClient = useQueryClient();
	const query = useQuery({
		queryKey: SETTINGS_QUERY_KEY,
		queryFn: () => commands.settingsGet(),
	});

	const mutation = useMutation({
		mutationFn: (value: boolean) => commands.settingsSetAssignMeToTasks(value),
		onSuccess: (updated: AppSettingsView) => {
			queryClient.setQueryData(SETTINGS_QUERY_KEY, updated);
		},
	});

	return {
		assignMeToTasks: query.data?.assignMeToTasks ?? DEFAULT_ASSIGN_ME_TO_TASKS,
		loading: query.isPending,
		setAssignMeToTasks: mutation.mutate,
	};
}
