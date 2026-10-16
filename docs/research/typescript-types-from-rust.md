# Generating TypeScript types from Rust

Research for [#15](https://github.com/FredPerr/me/issues/15), part of map [#14](https://github.com/FredPerr/me/issues/14). Checked October 2026 against the repo at `origin/main` (`5be1e03`, Tauri `2.11.5` in `Cargo.lock`, Rust `1.98.1` in `mise.toml`).

## Answer

Use **tauri-specta** (`=2.0.0-rc.25`) with **specta** (`=2.0.0-rc.25`) and **specta-typescript** (`=0.0.12`), pinned exactly. It is the only option that types both the data *and* the commands (and events), replacing `invoke("…")` strings with a generated `commands.*` object. It handles every `work_tracking` pattern, with two required code changes: `u64` fields need an explicit `number` override, and every exposed type needs `#[derive(specta::Type)]`.

The main risk is maturity: tauri-specta 2 has been a release candidate since 2024, the authors ask for exact `=` pins because RCs break, and docs.rs failed to build rc.25.

## Options compared

| | tauri-specta 2 (rc) | ts-rs 12 | Hand-written (today) |
|---|---|---|---|
| Data types | yes (`specta::Type` derive) | yes (`ts_rs::TS` derive) | manual mirror |
| Command signatures / call sites | yes, generated `commands` object | no, still hand-written `invoke` wrappers | no |
| Events | yes, typed `events` object | no | no |
| Error type in signature | yes (`Result` union or throw) | no | manual |
| Release status | `2.0.0-rc.25`, 2026-05-08, prerelease | `12.0.1`, 2026-01-31, stable | n/a |
| `u64` default | export error ("bigint forbidden") | exported as `bigint` (wrong for JSON) | n/a |

Sources: [tauri-specta versions](https://crates.io/crates/tauri-specta/versions), [ts-rs on crates.io](https://crates.io/crates/ts-rs), [ts-rs `large_int` default](https://github.com/Aleph-Alpha/ts-rs/blob/main/ts-rs/src/lib.rs).

ts-rs is the more mature crate (stable, about 18.6M downloads) and its serde compatibility covers `rename_all`, `tag`, `skip_serializing_if` and others ([README, serde compatibility](https://github.com/Aleph-Alpha/ts-rs#serde-compatibility)). But it only generates types. With ~50 commands, the string names, argument keys and return types would still be hand-kept, which is the drift the thin-frontend goal is trying to remove. Its README also says `skip_serializing_if` only makes a field optional when paired with `#[serde(default)]`; our output-only types use `skip_serializing_if` alone, so ts-rs would emit `field: T | null` while the JSON actually omits the key.

Hand-written types (current state: 29 frontend files import `invoke`, with types mirrored in files like `src/infra/work-tracking/workTracking.ts`) contradict the map's rule that TypeScript only holds *generated* mirrors of Rust.

## tauri-specta maturity and compatibility

- Latest release: [v2.0.0-rc.25](https://github.com/specta-rs/tauri-specta/releases/tag/v2.0.0-rc.25), 2026-05-08, marked prerelease. Previous: rc.24 (2026-03-30), rc.21 (2025-01-13). The repo is active (pushed October 2026, ~800 stars, 32 open issues).
- The crate docs call v2 beta and tell users to pin Specta with `=` ([lib.rs at rc.25](https://github.com/specta-rs/tauri-specta/blob/v2.0.0-rc.25/src/lib.rs)). The [rc.24 release notes](https://github.com/specta-rs/tauri-specta/releases/tag/v2.0.0-rc.24) list breaking changes between RCs.
- [docs.rs failed to build rc.25](https://docs.rs/crate/tauri-specta/2.0.0-rc.25); read the source at the tag or the [example app](https://github.com/specta-rs/tauri-specta/tree/main/examples/app) instead.
- Depends on `tauri = "^2"` with Tauri's own `specta` feature ([rc.25 dependencies](https://crates.io/api/v1/crates/tauri-specta/2.0.0-rc.25/dependencies)). Tauri 2.11.5 declares `specta = "^2.0.0-rc.16"` as an optional dependency, which rc.25 satisfies ([tauri Cargo.toml](https://docs.rs/crate/tauri/2.11.5/source/Cargo.toml.orig)).
- Edition 2024 crate, so Rust ≥ 1.85; the repo pins 1.98.1.
- `specta` itself is also only stable at 1.0.5; 2.x is `2.0.0-rc.25` ([crates.io](https://crates.io/crates/specta)).
- Tauri 3 is in alpha (`3.0.0-alpha.5`, 2026-10-14, [crates.io](https://crates.io/crates/tauri/versions)). tauri-specta only targets Tauri 2 today.

## What changes

### Rust build

```toml
[dependencies]
specta = { version = "=2.0.0-rc.25" }
specta-typescript = "=0.0.12"
tauri-specta = { version = "=2.0.0-rc.25", features = ["derive", "typescript"] }
```

Bindings are written by `builder.export(Typescript::default(), "../src/bindings.ts")`. The upstream example runs this inside `#[cfg(debug_assertions)]` at app start, so `pnpm tauri dev` regenerates the file. CI has no Cargo step today, so nothing would catch a stale `bindings.ts`. A Rust `#[test]` that exports to a string and compares it with the committed file, plus a CI job that runs `cargo test`, would close that gap.

### Writing and registering commands

Each command gets a second attribute, and the ~50-entry `generate_handler!` becomes `collect_commands!` with the same list ([lib.rs setup example](https://github.com/specta-rs/tauri-specta/blob/v2.0.0-rc.25/src/lib.rs)):

```rust
#[tauri::command]
#[specta::specta]
pub async fn work_tracking_list_projects(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
    cursor: Option<String>,
) -> Result<Page<WorkProject>, CommandError> { /* unchanged */ }

let specta_builder = tauri_specta::Builder::<tauri::Wry>::new()
    .commands(tauri_specta::collect_commands![
        greet,
        git::list_worktrees,
        work_tracking::commands::work_tracking_list_projects,
        // ...the other ~47
    ]);

tauri::Builder::default()
    .invoke_handler(specta_builder.invoke_handler())
    .setup(move |app| { specta_builder.mount_events(app); Ok(()) })
```

- `State`, `AppHandle` and `Channel` arguments are left out of the TS signature; the example app covers `AppHandle` and `Channel` ([main.rs](https://github.com/specta-rs/tauri-specta/blob/main/examples/app/src-tauri/src/main.rs)).
- Every argument and return type must implement `specta::Type`, so every DTO across `git`, `git_provider`, `ai_session` and `work_tracking` needs the derive before its command can be listed.
- Tauri allows one `invoke_handler`. Tauri's generated handler returns `false` for unknown commands ([handler.rs at tauri-v2.11.5](https://github.com/tauri-apps/tauri/blob/tauri-v2.11.5/crates/tauri-macros/src/command/handler.rs)), but it takes `Invoke` by value, so you can't just chain two handlers. Mixing old and new commands would need a dispatcher that checks the command name first. That approach is untested here. Converting all commands in one mechanical PR is simpler.
- Typed events: `#[derive(tauri_specta::Event)]` plus `collect_events!` gives a typed `events.projectChanged.listen(...)`. That fits the map's `project-changed` decision and the existing `ai_session` `app.emit` calls.

### Frontend calls

The generated `bindings.ts` exports `commands` with camelCase methods that wrap `invoke` ([example bindings.ts](https://github.com/specta-rs/tauri-specta/blob/main/examples/app/src/bindings.ts)):

```ts
import { commands } from "@/bindings";

const result = await commands.workTrackingListProjects(connectionId, cursor ?? null);
if (result.status === "error") throw WorkTrackingError.fromWire(result.error);
return result.data;
```

- Commands returning `Result<T, E>` resolve to `{ status: "ok"; data: T } | { status: "error"; error: E }` by default. `.error_handling(ErrorHandlingMode::Throw)` makes them throw instead, and the `typedError` helper can be replaced ([builder.rs](https://github.com/specta-rs/tauri-specta/blob/v2.0.0-rc.25/src/builder.rs)). `Throw` keeps today's `TauriWorkTrackingGateway.call` try/catch shape.
- Hand-written types such as `Page`, `WorkItem` and `ConnectionView` in `workTracking.ts` become re-exports of, or get replaced by, generated types. Gateways and hooks stay, but they call `commands.*`.
- `bindings.ts` lands under `src/`, which Biome lints. It needs a Biome ignore entry or a `.header(...)` lint directive.

## The `work_tracking` patterns

| Pattern | Handled? | Notes |
|---|---|---|
| `typed_id!` newtype with `#[serde(transparent)]` | yes | Add `specta::Type` to the macro's `derive` list. specta-serde reads `transparent` ([parser.rs](https://github.com/specta-rs/specta/blob/v2.0.0-rc.25/specta-serde/src/parser.rs)), so each id exports as `string`. For nominal ids in TS, `specta_typescript::branded!` renders `string & { readonly __brand: "ConnectionId" }` ([branded.rs](https://github.com/specta-rs/specta/blob/v2.0.0-rc.25/specta-typescript/src/branded.rs)). |
| `rename_all = "camelCase"` on structs and unit enums | yes | Unit enums such as `WorkItemStatus` and `ProviderKind` become string-literal unions. |
| `skip_serializing_if = "Option::is_none"` | yes | With serde phases on (the default), the `Serialize` view marks the field optional (`field?: T`) ([specta-serde lib.rs](https://github.com/specta-rs/specta/blob/v2.0.0-rc.25/specta-serde/src/lib.rs)). Types that are both input and output may get `_Serialize` / `_Deserialize` variants. |
| Generic `Page<T>` | yes | Generics are supported. |
| `Result<_, CommandError>` | yes, weakly typed | `CommandError` needs `specta::Type`. Its `&'static str` fields export as `string`, so `kind` is not a union. Turning `kind` and `reason` into serde enums would give `kind: "notConfigured" \| "invalidInput" \| …`. |
| `retry_after_seconds: Option<u64>` | needs change | specta-typescript refuses `u64` / `i64` / `usize` by default ([primitives.rs](https://github.com/specta-rs/specta/blob/v2.0.0-rc.25/specta-typescript/src/primitives.rs)). Use `#[specta(type = Option<specta_typescript::Number>)]` ([types.rs](https://github.com/specta-rs/specta/blob/v2.0.0-rc.25/specta-typescript/src/types.rs)) or narrow it to `u32`. The same applies to `git_provider` (`number: u64`, `expires_in: u64`, `interval: u64`). |
| `deny_unknown_fields` input struct | yes | Behavior is unchanged. The struct only needs the derive. |

Teamwork DTOs in `infra/teamwork/dto.rs` never cross the IPC boundary, so they need no derive.

## Not verified

No spike compiled here. Before committing to this, a short spike should check three things:

- `cargo build` with the pins above against Tauri 2.11.5.
- That `greet(name: &str)` and the `ai_session` channel commands pass `#[specta::specta]`.
- The exact TS output for `CommandError` and one `typed_id!`.
