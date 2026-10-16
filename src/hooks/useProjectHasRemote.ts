import type { ProjectView } from "@/bindings";
import { useProjectRemotes } from "@/hooks/useProjectRemotes";

export function useProjectHasRemote(project: ProjectView) {
	const { remotes } = useProjectRemotes(project);
	return { hasRemote: remotes.length > 0 };
}
