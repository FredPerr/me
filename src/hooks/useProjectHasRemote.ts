import { useProjectRemotes } from "@/hooks/useProjectRemotes";
import type { Project } from "@/models/Project";

export function useProjectHasRemote(project: Project) {
	const { remotes } = useProjectRemotes(project);
	return { hasRemote: remotes.length > 0 };
}
