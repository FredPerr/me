import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { Context, Project } from "@/models/Project";

export type RepositoryDiffStats = {
  filesAdded: number;
  filesModified: number;
  filesDeleted: number;
  filesRenamed: number;
  insertions: number;
  deletions: number;
};

type WorktreeStatusResponse = {
  files_added: number;
  files_modified: number;
  files_deleted: number;
  files_renamed: number;
  insertions: number;
  deletions: number;
  pr_url: string | null;
};

export function useContextDiffStats(context: Context, project: Project, allContexts: Context[]) {
  const [statsByRepository, setStatsByRepository] = useState<Record<string, RepositoryDiffStats | null>>({});

  const hasBaseContext = !!context.baseContextName;

  useEffect(() => {
    if (context.isDefault || !hasBaseContext) return;

    let cancelled = false;

    async function fetchStats() {
      const results: Record<string, RepositoryDiffStats | null> = {};

      const baseContext = allContexts.find((c) => c.name === context.baseContextName);

      for (const branch of context.branches) {
        const repository = project.findRepository(branch.repositoryId);
        if (!repository) continue;

        const path = context.getWorktreePath(project.path, repository);
        const baseBranch = baseContext?.getBranchForRepository(branch.repositoryId) ?? null;

        if (!baseBranch) {
          results[branch.repositoryId] = null;
          continue;
        }

        try {
          const status = await invoke<WorktreeStatusResponse>("get_worktree_status", {
            path,
            baseBranch,
          });

          results[branch.repositoryId] = {
            filesAdded: status.files_added,
            filesModified: status.files_modified,
            filesDeleted: status.files_deleted,
            filesRenamed: status.files_renamed,
            insertions: status.insertions,
            deletions: status.deletions,
          };
        } catch {
          results[branch.repositoryId] = null;
        }
      }

      if (!cancelled) {
        setStatsByRepository(results);
      }
    }

    fetchStats();

    return () => {
      cancelled = true;
    };
  }, [context, project, allContexts, hasBaseContext]);

  return { statsByRepository, hasBaseContext };
}
