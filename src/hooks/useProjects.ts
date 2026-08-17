import { useCallback, useEffect, useState } from "react";
import type { Project } from "@/models/Project";
import { ProjectDirectory } from "@/models/ProjectDirectory";

export function useProjects() {
	const [projects, setProjects] = useState<Project[]>([]);
	const [loading, setLoading] = useState(true);

	const reload = useCallback(async () => {
		const loaded = await ProjectDirectory.loadAllProjects();
		setProjects(loaded);
		setLoading(false);
	}, []);

	useEffect(() => {
		reload();
	}, [reload]);

	return { projects, loading, reload };
}
