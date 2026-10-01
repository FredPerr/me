# Open single-repository folders directly in the IDE

The change makes both IDE entry points ("Open in IDE" on a context card and "Open in IDE" on the project tab) open the repository folder when exactly one repository applies, and keeps opening the parent folder otherwise. The rule is modeled on the `Project` aggregate as ordered candidate path lists (`resolveContextIdePathCandidates`, `resolveRootIdePathCandidates`). The existence-check fallback lives in one hook method, `useOpenInIde.openFirstExisting`. Components only choose which candidate list to request. The paths match the Rust worktree layout (`project_dir/.worktrees/<context>/<repo.name>` in `git.rs`), and the zero-repo, multi-repo and missing-folder cases fall back the way the task asks.

Watch for: the default-context single-repo path is built from `relPath` as plain text, while the project-root button uses `resolveAbsolutePath`. So `~`/absolute relPaths on the default context miss and fall back to the parent (possible, non-blocking). `openFirstExisting` has no unit tests (confirmed, non-blocking).

**Verdict**: APPROVED

## High-level view

For a context, the repository count is the number of the context's branches that resolve to a known effective repository. The count does not come from the filesystem. With exactly one, the first candidate is that repository's worktree. The context folder `<project>/.worktrees/<name>` and then the project path follow. With zero or several, the chain starts at the context folder, which is exactly the previous behavior.

The project-root button now counts `effectiveRepositories()`. With exactly one configured repository it tries the resolved repository folder first, then the project path. With zero configured repos, the synthetic `"."` repository dedupes to the project path, so nothing changes there.

`openFirstExisting` walks the candidates with the existing `check_path_exists` command. If none exist, it opens the last candidate (the project path), which keeps the old unconditional fallback. `getWorktreePath` now returns the bare project path for `"."` on the default context instead of `<project>/.`. The other callers (`usePullContexts`, `useContextDiffStats`) get an equivalent path.

<details>
<summary>Issues (2)</summary>

1. **Default-context path ignores `~`/absolute relPaths** (possible): `Context.getWorktreePath` joins `relPath` textually. A default-context single repo with `relPath` `~/x` or `/abs/x` produces a path that doesn't exist, so the IDE opens the parent instead of the repo. This is a pre-existing limitation that now affects IDE opening. The fallback keeps it safe. Optional fix: use `resolveAbsolutePath` for the default context, which would make the method async.
2. **`openFirstExisting` untested** (confirmed): the fallback chain (first existing wins, last candidate when none exist, no-op without an IDE command) has no test. Consider a small hook test that mocks `invoke`.

</details>

<details>
<summary>Details</summary>

### Where the rule lives

The previous inline `projectBase`/`worktreePath` computation and the `check_path_exists` call are gone from `ContextCard`. `getContextFolderPath` now feeds both `isActive` and the candidate list, so the active-workspace key and the opened folder can't drift apart. The `Project` aggregate owns "which folders, in which order", and the hook owns "which exists". This split keeps Tauri I/O out of the model and duplication out of the components, so it matches the user's DDD preference without adding a speculative abstraction.

### Edge cases

Zero repositories: a context with no branches, or only branches with unknown repository ids, skips the repo candidate (confirmed by the tests). A project with no configured repositories uses the synthetic repo named after the project, so a non-default context targets `.worktrees/<ctx>/<projectName>`. That matches what Rust creates for the synthetic input.

Missing single-repo worktree: the chain continues to the context folder, then the project path. When nothing exists, the project path still opens. Duplicate entries are removed through `Set`, and both trailing-slash forms are normalized (tested).

Default context: for a single `./api` repo it opens `<project>/api`. For the synthetic root repo it dedupes to `[project, .worktrees/main]`, which opens the project as before. The coder documented the order difference from the plan.

Inconsistency (possible): `resolveRootIdePathCandidates` resolves the repo with `resolveAbsolutePath`, which handles `~` and absolute paths. `getWorktreePath` for the default context does a textual join, which yields `<project>//abs/x` for an absolute relPath. Rust uses `project_dir.join(rel_path)`, which would honor the absolute path. The existence check catches the mismatch and falls back to the parent. The user loses the single-repo behavior in that setup, but nothing breaks.

### Hook behavior

`openFirstExisting` returns early when no IDE command is configured. The buttons are already disabled in that case. Paths are checked one at a time, which costs at most three IPC round trips. If `check_path_exists` rejects, the error propagates the same way it did in the old `ContextCard` code.

### Test coverage

There are 12 new model tests covering single, multiple, zero and unknown repositories, the synthetic repo, the default context, trailing-slash normalization, and the three root-button cardinalities. The coder reports that lint, the 114 tests, the typecheck/build, and knip (no new findings) all pass. Not tested: `openFirstExisting`, and the default context with `~`/absolute relPaths. Manual `tauri dev` verification was not run.

</details>

<details>
<summary>File map</summary>

- `src/models/Project.ts`: adds `normalizeFolderPath`, `Context.getContextFolderPath`, the `"."` handling in `getWorktreePath`, and `Project.resolveContextIdePathCandidates` / `resolveRootIdePathCandidates`.
- `src/models/Project.test.ts`: adds the candidate-resolution tests and a `buildContext` helper.
- `src/hooks/useOpenInIde.ts`: adds `openFirstExisting`.
- `src/components/contexts/ContextCard.tsx`: uses domain candidates and drops the inline path logic and the `invoke` import.
- `src/components/project-tabs/ProjectTabPanel.tsx`: the root button uses `resolveRootIdePathCandidates`.

Full diff: `git diff` in the working tree.

</details>
