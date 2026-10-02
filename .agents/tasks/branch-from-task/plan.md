# Implementation Plan — Branch name from task

Repo: /Users/fredperr/Projects/me. Work in the current tree, no commits. Biome uses tabs (lineWidth 100). Commands: `pnpm test` (vitest), `pnpm lint` (biome check), `pnpm build` (tsc + vite), `pnpm knip` (unused exports). Only locale is `src/i18n/en.ts`.

## Findings that shape the design

- `src/utils/slugify.ts` already implements the exact slug rules (NFD + strip diacritics, lowercase, non-alphanumeric runs → `-`, trim `-`). Reuse it; do not write a second slugger.
- Only `CreateFromContextModal.tsx` (opened from `ContextCard.tsx` via `ContextsGrid.tsx`, wired to `useContexts.createContext`) lets the user type NEW branch names. `CreateFromBranchesModal.tsx` only selects existing branches (its free text is the context name, not a branch) → skipped. No other context-creation modal exists (`ProjectTabPanel` only opens `CreateFromBranchesModal`; `RepositoryBranchesModal`/`ShareForReviewModal` are not context creation).
- `Project` is constructed positionally in `Project.addContext`, `Project.removeContext`, `Project.fromJSON` and `useContexts.savePullRequestDrafts` (`src/hooks/useContexts.ts` ~line 158). Every one of these must forward the new field, or saving drafts / adding a context would silently reset the project's branch config.
- `ProjectForm.tsx` builds a `ProjectData` and calls `Project.fromJSON`; `ProjectSettings.tsx` persists with `ProjectDirectory.saveProject(project)` → `project.toJSON()`. So persistence only needs `fromJSON`/`toJSON` changes; `ProjectDirectory.ts` needs no change (verify during implementation).

## Decisions

1. Domain location: new value object `BranchNamingPolicy` in `src/models/BranchNaming.ts` (models already hold domain logic; slug primitive stays in `src/utils/slugify.ts`). Prefixes are stored WITHOUT trailing slash (`"feature"`), displayed as `feature/`, composed as `${prefix}/${slug}`.
2. Persistence shape: nested optional `branchNaming?: { prefixes: string[]; allowNoPrefix: boolean }` on `ProjectData` (one value object, one key). Missing key / missing fields → defaults `["feature","fix","chore","refactor"]` and `allowNoPrefix: true` (preserves today's freedom for existing projects).
3. If the prefix list is empty, "no prefix" is effectively allowed regardless of the flag (domain never gets into an unusable state); the settings form additionally blocks saving "no prefixes + no-prefix disallowed".
4. Default selection in the generator: no prefix when allowed, otherwise the first prefix. "No prefix" option listed first when present.
5. UX (CreateFromContextModal only): an ActionIcon (phosphor `MagicWandIcon`, aria-label + Tooltip) added to the context-name TextInput `rightSection` next to the existing sync icon, opening a Mantine `Popover` (pattern from `src/components/shared/IconPicker.tsx`) containing: task-name TextInput (autofocus, Enter applies), prefix Select, live preview of the generated branch, Apply button (disabled when the slug is empty). On apply:
   - if `syncBranchNames` is on → call the existing `handleContextNameChange(branchName)` (sets context name and syncs all create-branch repos);
   - else → set `branchName` on every repo with `createBranch === true` (via `updateRepoState`, which clears their errors) and set the context name only if it is currently empty.
   Generated names remain editable (they are plain field values).
6. Select "no prefix" sentinel value `"~none"` (a `~` can never appear in a valid prefix, so no collision; avoids Mantine empty-string option issues). Select inside the popover uses `comboboxProps={{ withinPortal: false }}` so choosing an option doesn't trigger the popover's outside-click close; `allowDeselect={false}`.
7. Duplicate prefixes (after normalization) are deduplicated silently by the domain; empty rows in settings are ignored (same as symlinks); invalid non-empty rows show an error and block submit.

## Domain API (exact)

`src/models/BranchNaming.ts`:

```ts
export const DEFAULT_BRANCH_PREFIXES: readonly string[] = ["feature", "fix", "chore", "refactor"];

export type BranchNamingData = { prefixes: string[]; allowNoPrefix: boolean };

export type BranchPrefixParseResult =
	| { valid: true; prefix: string }
	| { valid: false; reason: "empty" | "invalid" };

export function parseBranchPrefix(rawPrefix: string): BranchPrefixParseResult;

export class BranchNamingPolicy {
	private constructor(readonly prefixes: string[], private readonly allowNoPrefixSetting: boolean);
	static create(prefixes: string[], allowNoPrefix: boolean): BranchNamingPolicy; // parses, drops invalid/empty, dedupes, keeps order
	static default(): BranchNamingPolicy;
	static fromJSON(data: Partial<BranchNamingData> | undefined): BranchNamingPolicy;
	get allowsNoPrefix(): boolean;        // allowNoPrefixSetting || prefixes.length === 0
	get defaultPrefix(): string | null;   // allowsNoPrefix ? null : prefixes[0]
	composeBranchName(taskName: string, prefix: string | null): string;
	toJSON(): BranchNamingData;           // stores the raw allowNoPrefix setting
}
```

`parseBranchPrefix` rules: trim; strip trailing (and leading) `/`; empty → `{valid:false, reason:"empty"}`. Invalid (`reason:"invalid"`) if: any char in `\x00-\x20`, `\x7f`, `~ ^ : ? * [ \`; contains `..`, `@{` or `//`; equals `@`; any `/`-segment starts with `.` or ends with `.lock`; ends with `.`. Otherwise `{valid:true, prefix}` (case preserved, e.g. `Feature` stays valid).

`composeBranchName`: `slug = slugify(taskName)`; empty slug → `""`. `prefix === null` → returns `slug` if `allowsNoPrefix`, else throws `Error`. Non-null prefix not in `prefixes` → throws `Error`. Otherwise `${prefix}/${slug}`.

## Plan

- [ ] 1. Create the `BranchNamingPolicy` value object and `parseBranchPrefix` with unit tests.
      Files: src/models/BranchNaming.ts, src/models/BranchNaming.test.ts, src/utils/slugify.test.ts
      Tests (vitest, style of `src/models/Project.test.ts` / `src/utils/slugify.test.ts`):
      - slugify.test.ts: add `"[WAFR] Do something here"` → `"wafr-do-something-here"`; `"Réparer l'été"` → `"reparer-l-ete"`.
      - composeBranchName: `("[WAFR] Do something here", null)` with default policy → `"wafr-do-something-here"`; with `"feature"` → `"feature/wafr-do-something-here"`; with `"fix"` → `"fix/..."`; `"  !!!  "` → `""`; null prefix when `allowNoPrefix=false` throws; unknown prefix throws.
      - parseBranchPrefix: `"feature/"` → `feature`; `" chore// "` → `chore`; `"team/feature"` valid; `""`/`"  "`/`"/"` → empty; `"my prefix"`, `"a..b"`, `"a~b"`, `"a^b"`, `"a:b"`, `"a?b"`, `"a*b"`, `"a[b"`, `"a\\b"`, `"@"`, `"a@{b"`, `".hidden"`, `"x.lock"`, `"x."`, `"a//b"` → invalid.
      - create: drops invalid/empty, normalizes trailing slash, dedupes (`["feature","feature/"]` → `["feature"]`), preserves order.
      - allowsNoPrefix/defaultPrefix: allowed → `null`; disallowed → first prefix; empty list + disallowed → allowsNoPrefix true, defaultPrefix null.
      - fromJSON: `undefined` → default prefixes + allowsNoPrefix true; `{}` → same; `{allowNoPrefix:false}` → default prefixes, disallowed; `{prefixes:["ops"]}` → `["ops"]`, allowed; `{prefixes:[]}` stays empty. toJSON round-trips.
      Verify: `pnpm test` — new tests pass; `pnpm lint` clean.

- [ ] 2. Add `branchNaming` to the Project aggregate (depends on 1).
      Add constructor param `public readonly branchNaming: BranchNamingPolicy = BranchNamingPolicy.default()` as the LAST positional param (after `symlinks`); forward it in `addContext`, `removeContext`; `fromJSON` uses `BranchNamingPolicy.fromJSON(data.branchNaming)`; `toJSON` emits `branchNaming: this.branchNaming.toJSON()`; `ProjectData` gets `branchNaming?: BranchNamingData`. Update `savePullRequestDrafts` in useContexts.ts to pass `project.branchNaming`. Leave the existing IDE-path methods and `normalizeFolderPath` usage untouched.
      Files: src/models/Project.ts, src/hooks/useContexts.ts, src/models/Project.test.ts
      Tests in Project.test.ts (new `describe("branchNaming")`): fromJSON of legacy data without `branchNaming` → default prefixes and allowsNoPrefix true; fromJSON with `{prefixes:["ops"], allowNoPrefix:false}` → preserved, toJSON round-trips it; `addContext` and `removeContext` preserve a custom `branchNaming`.
      Verify: `pnpm test` and `pnpm build` pass (the default param means tsc will NOT flag a missed forward; check every `new Project(` in src by reading it). Verify during implementation that `ProjectDirectory.ts` needs no change.

- [ ] 3. Add all i18n keys to src/i18n/en.ts (needed by 4 and 5).
      `contexts`: `branchFromTask: "Generate branch name from a task"`, `taskName: "Task name"`, `taskNamePlaceholder: "[WAFR] Do something here"`, `branchPrefix: "Prefix"`, `noBranchPrefix: "No prefix"`, `generatedBranchPreview: "Branch: {{branch}}"`, `applyBranchName: "Apply"`.
      New `settings.branchNaming`: `title: "Branch prefixes"`, `description: "Prefixes offered when generating a branch name from a task name."`, `prefixPlaceholder: "feature"`, `addPrefix: "Add prefix"`, `removePrefix: "Remove prefix"`, `allowNoPrefix: "Allow branches without a prefix"`, `invalidPrefix: "Invalid prefix. Avoid spaces, \"..\" and ~ ^ : ? * [ \\"`, `prefixRequired: "Add at least one prefix or allow branches without a prefix."`.
      Files: src/i18n/en.ts
      Verify: `pnpm build` passes.

- [ ] 4. Settings UI: `BranchPrefixFormList` + wire into ProjectForm (depends on 1–3).
      New functional component `src/components/settings/BranchPrefixFormList.tsx` with props `{ prefixes: string[]; allowNoPrefix: boolean; onPrefixesChange(prefixes: string[]); onAllowNoPrefixChange(allowed: boolean); onValidityChange(valid: boolean) }`. Layout mirrors the symlinks block in ProjectForm.tsx: `Input.Label`/`Input.Description`, one row per prefix (`TextInput` with `aria-label`, `error` = `t("settings.branchNaming.invalidPrefix")` when `parseBranchPrefix(value).reason === "invalid"`, `rightSection` text `/` optional), trash `ActionIcon` with `aria-label={t("settings.branchNaming.removePrefix")}`, "Add prefix" button, and a Mantine `Switch` (already used in RepositoryFormList) for `allowNoPrefix`. Show `prefixRequired` error text when no valid prefix and switch off. Report validity via `useEffect` → `onValidityChange` (no invalid rows AND not the prefixRequired case).
      In ProjectForm.tsx: state `branchPrefixes` (init `initialProject?.branchNaming.prefixes ?? [...DEFAULT_BRANCH_PREFIXES]`), `allowNoBranchPrefix` (init `initialProject?.branchNaming.toJSON().allowNoPrefix ?? true`, the raw setting rather than the derived getter), `branchPrefixesValid`; render the list after the symlinks block; `canSubmit` also requires `branchPrefixesValid`; in `handleSubmit` set `projectData.branchNaming = BranchNamingPolicy.create(branchPrefixes, allowNoBranchPrefix).toJSON()`.
      Files: src/components/settings/BranchPrefixFormList.tsx, src/components/settings/ProjectForm.tsx
      Verify: `pnpm build`, `pnpm lint`, `pnpm test` pass. Manual (verify during implementation if the app can run via `pnpm dev`): edit a project, change prefixes/toggle, save, reopen → values persisted; legacy project shows the 4 defaults and toggle on.

- [ ] 5. Branch-from-task popover in CreateFromContextModal (depends on 1–3).
      New functional component `src/components/contexts/BranchNameFromTaskPopover.tsx`, props `{ branchNaming: BranchNamingPolicy; onApply: (branchName: string) => void }`. Uses `useDisclosure`, `Popover` (`trapFocus`, `position="bottom-end"`, `width={320}`), target `ActionIcon` (`MagicWandIcon`, `variant="subtle"`, `size="sm"`, `aria-label={t("contexts.branchFromTask")}`, wrapped in `Tooltip`). Dropdown: `TextInput` label `contexts.taskName`, placeholder `contexts.taskNamePlaceholder`, `data-autofocus`, Enter key applies; `Select` label `contexts.branchPrefix`, data = (`allowsNoPrefix` ? [{value:"~none", label:t("contexts.noBranchPrefix")}] : []) + prefixes as `{value: prefix, label: `${prefix}/`}`, initial value = `defaultPrefix ?? "~none"`, `allowDeselect={false}`, `comboboxProps={{ withinPortal: false }}`; preview `Text size="xs" c="dimmed"` with `contexts.generatedBranchPreview`; `Button` `contexts.applyBranchName` disabled when `composeBranchName(...) === ""`. On apply: call `onApply`, reset task name and prefix to default, close. Reset the selected prefix to default when the popover opens (policy may have changed).
      In CreateFromContextModal.tsx: change the context-name `rightSection` to a `Group gap={4} wrap="nowrap"` with the new popover then the existing sync Tooltip/ActionIcon; set `rightSectionWidth={60}`. Add `applyGeneratedBranchName(branchName)` implementing Decision 5. Pass `branchNaming={project.branchNaming}`. Do not change CreateFromBranchesModal.
      Files: src/components/contexts/BranchNameFromTaskPopover.tsx, src/components/contexts/CreateFromContextModal.tsx
      Verify: `pnpm build`, `pnpm lint`, `pnpm test`, `pnpm knip` (no new unused exports) pass. Manual if runnable: "[WAFR] Do something here" + `feature/` → context name and branch fields become `feature/wafr-do-something-here` with sync on; with sync off only create-branch repos get it and an existing context name is kept; fields stay editable; project with allowNoPrefix=false shows no "No prefix" option and preselects the first prefix.

- [ ] 6. Final pass: `pnpm lint`, `pnpm build`, `pnpm test`, `pnpm knip` all green; `git status` shows only the files listed above (plus `.agents/`), and the pre-existing single-repo IDE-open code in Project.ts/useOpenInIde.ts/ContextCard.tsx/ProjectTabPanel.tsx/resolvePath.ts is unchanged apart from step 2's additive edits to Project.ts.

## Gaps / assumptions

- Context names containing `/` (e.g. `feature/x`) already produce nested `.worktrees/feature/x` folders today; behavior is unchanged and not in scope.
- No other locales exist; only en.ts is updated.
