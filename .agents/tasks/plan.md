# Implementation Plan: open single-repository folders directly in the IDE

Work on the current branch in `/Users/fredperr/Projects/me`. Do NOT commit. Biome config (`biome.json`) uses tabs and lineWidth 100; it wins over the 2-space preference.

## Decisions

- Repository count comes from the model, not the filesystem. A context's repositories are `context.branches` resolved through `project.findEffectiveRepository(repositoryId)` (unknown ids dropped). That is the set of worktree folders the context actually created, which can be a subset of the project's repositories. The project-root button counts `project.effectiveRepositories()`. Zero configured repositories yields one synthetic repo with `relPath "."`, which resolves to the project path, so the result is the same as today.
- Folder paths come from existing domain methods. The single-repo worktree path is `context.getWorktreePath(project.path, repository)`, the same method `useContextDiffStats` and `usePullContexts` already use. The project-root path is `repository.resolveAbsolutePath(project.path)`, which handles `~`, absolute, `./x` and `.`. Listing directories is not needed and would be wrong when stray folders exist.
- The rule lives in the domain (`Project` aggregate) as ordered "candidate paths". The fallback chain lives in one place in the hook: `useOpenInIde.openFirstExisting(candidatePaths)` checks each path with the existing `check_path_exists` Tauri command and opens the first one that exists. If none exist, it opens the last candidate (the project path), which matches today's unconditional fallback. Components only pick candidates and call the hook.
- Fallback order for a context: single-repo worktree folder → context folder `<project>/.worktrees/<context.name>` → `project.path`. Multiple or zero repos skip the first candidate, which is exactly today's behavior.
- `getWorktreePath` is fixed so the default context with `relPath "."` returns the project path instead of `<project>/.`. That lets candidates dedupe cleanly and stops the IDE from receiving a trailing `/.`. Existing callers get an equivalent path.

## Steps

- [x] 1. Add domain methods in `src/models/Project.ts`, with unit tests.
      - `Context.getContextFolderPath(projectPath: string): string` returns `${normalizedProject}/.worktrees/${this.name}`, normalized the same way as `getWorktreePath`.
      - In `Context.getWorktreePath`, when the default-context normalized relPath is `"."` or `""`, return `normalizedProject`.
      - `Project.resolveContextIdePathCandidates(context: Context): string[]`:
        ```ts
        const contextRepositories = context.branches
          .map((branch) => this.findEffectiveRepository(branch.repositoryId))
          .filter((repository): repository is Repository => repository !== undefined);
        const candidates: string[] = [];
        if (contextRepositories.length === 1) candidates.push(context.getWorktreePath(this.path, contextRepositories[0]));
        candidates.push(context.getContextFolderPath(this.path), this.path);
        return [...new Set(candidates)];
        ```
      - `Project.resolveRootIdePathCandidates(): Promise<string[]>`. When `effectiveRepositories().length === 1`, return deduped `[await repository.resolveAbsolutePath(this.path), this.path]`. Otherwise return `[this.path]`.
      - Extend `src/models/Project.test.ts` using the existing `buildProject` / `buildRepository` helpers. Add a helper that builds a `Context` from repository ids and a default flag. Cover:
        - context with 1 branch: `[.../.worktrees/<ctx>/<repoName>, .../.worktrees/<ctx>, projectPath]`
        - context with 2 branches: `[.../.worktrees/<ctx>, projectPath]`
        - context with 0 branches, and a branch whose repositoryId is unknown: no repo candidate
        - project with zero configured repos plus a non-default context: worktree path uses the project name
        - default context with a single `./api` repo: first candidate is `<project>/api`
        - default context with the synthetic `"."` repo: deduped to `[<project>/.worktrees/<ctx>, projectPath]`
        - `getWorktreePath` with `"."` on the default context returns the project path
        - `resolveRootIdePathCandidates`: single `./api` → `[<project>/api, project]`; two repos → `[project]`; zero repos → `[project]` (deduped)
        - trailing-slash project path is normalized
      Files: src/models/Project.ts, src/models/Project.test.ts
      Verify: `pnpm test` passes, including the new tests.

- [x] 2. Add `openFirstExisting` to `src/hooks/useOpenInIde.ts`.
      `openFirstExisting(candidatePaths: string[]): Promise<void>` returns early when the list is empty or `settings?.ideCommand` is unset. It loops over the candidates, calling `invoke<boolean>("check_path_exists", { path })`, and calls `open` on the first one that exists. If none exist, it calls `open` on the last candidate. Wrap it in `useCallback` with `[open]` and return `{ open, openFirstExisting, isAvailable }`. Keep `open` exported in the return value.
      Files: src/hooks/useOpenInIde.ts
      Verify: `pnpm build` (runs `tsc && vite build`) succeeds.

- [x] 3. Wire the components (depends on 1 and 2).
      - `src/components/contexts/ContextCard.tsx`:
        - Remove the inline `projectBase` / `worktreePath` computation and replace it with `const contextFolderPath = context.getContextFolderPath(project.path);`, still used for `isActive(contextFolderPath)`.
        - Change `handleOpenInIDE` to `await openFirstExisting(project.resolveContextIdePathCandidates(context));`.
        - Drop the now-unused `invoke` import.
      - `src/components/project-tabs/ProjectTabPanel.tsx`: replace `onClick={() => open(project.path)}` with a `handleOpenRootInIde` async function that runs `await openFirstExisting(await project.resolveRootIdePathCandidates())`, and destructure `openFirstExisting` instead of `open`.
      Files: src/components/contexts/ContextCard.tsx, src/components/project-tabs/ProjectTabPanel.tsx
      Verify: `pnpm lint` (`biome check .`), `pnpm test` (`vitest run`) and `pnpm build` (`tsc && vite build`) all pass. Optionally run `pnpm knip` to confirm no unused exports were introduced.

## Final verification (from CONTRIBUTING.md)

```sh
pnpm lint
pnpm test
pnpm build
```

Behavior check, manual and only if the app can be run with `pnpm tauri dev`:
- On a context with one repository, "Open all in IDE" opens `.worktrees/<ctx>/<repo>`.
- With two or more repositories, it opens `.worktrees/<ctx>`.
- "Open root in IDE" on a single-repo project opens the repository folder. On a multi-repo project it opens the project root.

No Rust changes are needed; `check_path_exists` and `open_in_ide` already exist.
## Verification note (iteration 1)
Ran from the repo root after implementing steps 1–3:
- `pnpm lint` (`biome check .`): passed, 88 files checked, no diagnostics.
- `pnpm test` (`vitest run`): 13 files, 114 tests passed (12 new in `src/models/Project.test.ts`).
- `pnpm build` (`tsc && vite build`): typecheck and build passed (only the pre-existing chunk-size warning).
- `pnpm knip`: exits 1, but its findings are identical with the changes stashed (pre-existing: 3 unused files, 3 deps, 7 exports, 2 types); no new unused exports.
Deviation from the plan's test list: for the default context of the synthetic `"."` repo, the candidates are `[projectPath, <project>/.worktrees/<ctx>]` (the single-repo worktree path equals the project path and comes first after dedupe). It opens the project path, which is today's behavior.
Manual `pnpm tauri dev` behavior check was not run.
