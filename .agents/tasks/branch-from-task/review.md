# Branch name generated from a task name in context creation (pass 2)

The context-creation modal (`CreateFromContextModal`) now has a magic-wand button that opens a popover. In it the user types a task name such as "[WAFR] Do something here", picks a prefix, and applies `prefix/slug` (or just the slug) to the context name and the new-branch fields. Prefixes and an "allow no prefix" flag are stored per project as a `BranchNamingPolicy` value object on the `Project` aggregate. They are edited in `ProjectForm` through a new `BranchPrefixFormList`, and projects saved before this change load with the defaults. This pass checks the fix for last pass's blocking Escape issue: the popover is now controlled, and the modal gets `closeOnEscape={!isBranchPopoverOpened}`.

Watch for: with sync on, the context name becomes `feature/slug` and creates a nested worktree folder (confirmed, non-blocking, carried over from pass 1 and scoped out by the plan).

**Verdict**: APPROVED

## High-level view

Domain logic sits in `src/models/BranchNaming.ts` and reuses the existing `slugify`, which already covers the spec's rules (NFD, diacritics stripped, lowercase, non-alphanumeric runs collapsed into `-`, edge hyphens trimmed). `parseBranchPrefix` checks prefixes against git ref rules. `composeBranchName` rejects a null prefix when the policy forbids it and rejects any prefix the policy doesn't list. Components only build UI options and call the policy, which fits the DDD placement requirement.

Persistence is additive. `branchNaming` is an optional key on `ProjectData`, and a missing key or missing fields fall back to `["feature","fix","chore","refactor"]` with no prefix allowed. Every `new Project(` call (`addContext`, `removeContext`, `fromJSON`, `useContexts.savePullRequestDrafts`) forwards the policy, and tests cover the add/remove round-trip.

The popover's Select hides the "No prefix" option when the policy disallows it and preselects the first prefix in that case. The result goes into ordinary field values, so it stays editable. When the modal is not syncing branch names, only repos marked create-branch get the branch name, and the context name is filled only if it's empty. `CreateFromBranchesModal` only picks existing branches, so leaving it out matches requirement (5).

The pass-1 Escape problem is fixed. While the popover is open, the modal ignores Escape and the popover handles it. A reset effect clears the flag when the modal closes, so a stale value can't disable Escape the next time the modal opens. This was checked by reading the code, not by running the app.

<details>
<summary>Issues (2)</summary>

1. **Prefixed context name creates nested worktree folder** (confirmed, non-blocking): with sync on, applying `feature/slug` sets the context name to `feature/slug`, which creates `.worktrees/feature/slug`. As a follow-up, consider setting the context name to the slug only and keeping `prefix/slug` for the branch fields.
2. **No component-level coverage for the popover flow** (confirmed, non-blocking): no test covers apply with sync on or off, the hidden no-prefix option, or Escape staying inside the popover. Add a React Testing Library test for `BranchNameFromTaskPopover`/`applyGeneratedBranchName` if the project adopts component tests.

</details>

<details>
<summary>Details</summary>

### Escape handling between popover and modal

`CreateFromContextModal.tsx:56,79-81,206-212` owns `isBranchPopoverOpened`, passes it to the popover (`opened`/`onOpenedChange`), and sets `closeOnEscape={!isBranchPopoverOpened}` on `Modal`. Mantine's modal listens for Escape on the window in the capture phase, and according to verification.md it reads `closeOnEscape` through `useEffectEvent`. So the listener sees the current value, and Escape closes only the popover. The popover's target ActionIcon toggles through `open`/`close` and doesn't rely on `Popover.Target`'s own click handler, which is not attached in controlled mode, so clicks don't toggle twice. `open()` resets the task name and selected prefix each time, so a policy changed in settings is picked up on the next open (`BranchNameFromTaskPopover.tsx:55-59`). If the stored selection is no longer offered, `effectivePrefixValue` falls back to the default, which keeps `composeBranchName` from throwing on a stale prefix.

### Context name receives the prefixed branch name

`applyGeneratedBranchName` (`CreateFromContextModal.tsx:123-136`) with sync on goes through `handleContextNameChange(branchName)`. The context name becomes `feature/wafr-do-something-here`, so the worktree folder is nested under `feature/`. Hand-typed names already behave this way and the plan lists it as out of scope, so this doesn't block.

### Prefix settings validation

`BranchPrefixFormList` marks rows as invalid according to `parseBranchPrefix` and blocks submit through `onValidityChange` when there are invalid rows, or when no prefix is valid and the switch is off. Empty rows are ignored, which matches how symlinks are handled. `ProjectForm` initializes the switch from the raw `toJSON().allowNoPrefix` rather than the derived `allowsNoPrefix` getter. This keeps an empty-list project from showing the switch as on when the stored setting is off. Remove buttons and prefix inputs have aria-labels, and the required-prefix error uses `role="alert"`.

### Test coverage

The domain is well covered (`BranchNaming.test.ts`, `slugify.test.ts` with the spec example and a diacritics case, `Project.test.ts` for legacy loading, round-trip and context add/remove). Not tested: the popover/modal interaction, `applyGeneratedBranchName` with sync on and off, and settings-form validity propagation. verification.md reports tsc, lint and 158 tests green, and knip shows no new findings against the baseline. None of these were re-run for this review.

</details>

<details>
<summary>File map</summary>

- `src/models/BranchNaming.ts`: new `BranchNamingPolicy` value object, `parseBranchPrefix`, defaults
- `src/models/BranchNaming.test.ts`: domain unit tests
- `src/utils/slugify.test.ts`: task-name and diacritics cases
- `src/models/Project.ts`: `branchNaming` field, forwarded in add/remove/fromJSON/toJSON
- `src/models/Project.test.ts`: legacy load, round-trip, preservation tests
- `src/hooks/useContexts.ts`: forwards `branchNaming` in `savePullRequestDrafts`
- `src/components/contexts/BranchNameFromTaskPopover.tsx`: new controlled popover
- `src/components/contexts/CreateFromContextModal.tsx`: popover wiring, apply logic, Escape gating
- `src/components/settings/BranchPrefixFormList.tsx`: new settings list and switch
- `src/components/settings/ProjectForm.tsx`: state, validity, persistence of branch naming
- `src/i18n/en.ts`: `contexts.*` and `settings.branchNaming.*` strings

Full diff: `git diff` plus the untracked files above.

</details>
