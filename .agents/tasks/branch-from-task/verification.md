# Verification — branch-from-task (iteration 2)

review.json existed (CHANGES_REQUESTED), so this is a later iteration.

## Findings addressed

- `escape-closes-modal` (blocking): fixed. `BranchNameFromTaskPopover` is now controlled (`opened` + `onOpenedChange` props). `CreateFromContextModal` owns `isBranchPopoverOpened`, passes `closeOnEscape={!isBranchPopoverOpened}` to `Modal`, and resets the flag when the modal closes (so a stale "open" flag can't disable Escape on the next open). Mantine 9.5.1 `useModal` reads `closeOnEscape` through `useEffectEvent`, so the window capture listener sees the current value. With the popover open, Escape is handled by the popover only; once it closes, Escape closes the modal as before.
- `context-name-with-prefix-slash` (non-blocking, optional): not changed. Applying only the slug to the context name would change user-visible behavior, and the plan scopes it out. Left as a follow-up.

## Commands

| Command | Result |
| --- | --- |
| `pnpm exec tsc --noEmit` | exit 0, no errors |
| `pnpm lint` (`biome check .`) | "Checked 92 files … No fixes applied.", exit 0 |
| `pnpm test` (`vitest run`) | 14 files, 158 tests passed |
| `pnpm knip` | exit 1 from pre-existing findings only; same list as the iteration-1 baseline, no new findings |

Knip baseline (unchanged): 3 unused files (PullRequestButton.tsx, useBranches.ts, usePushWorktree.ts), 3 unused deps (plugin-global-shortcut, plugin-shell, plugin-window-state), 7 unused exports (scripts/analysis/config.ts ×3, isCodeComment, fetchIssueComments, i18n default, listGitProviders), 2 unused exported types (ModuleDependency, AuthStatus).

Not verified: running the app manually (`pnpm tauri dev`). The Escape fix was traced through the installed Mantine source (`ModalBase/use-modal.mjs`, `@mantine/hooks` `useWindowEvent`), not exercised in the running app.

## Files

- New: `src/models/BranchNaming.ts`, `src/models/BranchNaming.test.ts`, `src/components/settings/BranchPrefixFormList.tsx`, `src/components/contexts/BranchNameFromTaskPopover.tsx`
- Modified: `src/models/Project.ts` (additive `branchNaming` field), `src/models/Project.test.ts`, `src/utils/slugify.test.ts`, `src/hooks/useContexts.ts` (forwards `branchNaming`), `src/components/settings/ProjectForm.tsx`, `src/components/contexts/CreateFromContextModal.tsx`, `src/i18n/en.ts`
- Iteration 2 touched only `BranchNameFromTaskPopover.tsx` and `CreateFromContextModal.tsx`.
- `ProjectDirectory.ts` needed no change: it persists `project.toJSON()` and loads via `Project.fromJSON`.
- `CreateFromBranchesModal.tsx` unchanged: it only selects existing branches.
