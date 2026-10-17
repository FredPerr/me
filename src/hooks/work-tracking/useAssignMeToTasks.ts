import { useMutation } from "@tanstack/react-query";
import { type AssignMeResultView, commands, type TaskToAssignInput } from "@/bindings";

/**
 * Best-effort assignment of the current user to each task, called once by the
 * create-contexts-from-tasks flow after the contexts are created. The result
 * lists the tasks that could not be assigned; the mutation itself never fails
 * for a tracker hiccup, so the modal always closes on created contexts.
 */
export function useAssignMeToTasks() {
	return useMutation<AssignMeResultView, unknown, readonly TaskToAssignInput[]>({
		mutationFn: (tasks) => commands.workTrackingAssignMeToTasks([...tasks]),
	});
}
