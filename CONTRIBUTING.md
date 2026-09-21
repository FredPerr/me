# Contributing to me

Thanks for taking the time to contribute. This guide covers how to run the project locally and how to submit a pull request.

## Prerequisites

`me` is a [Tauri](https://tauri.app/) desktop app with a React + TypeScript frontend and a Rust backend.

Tooling (Node.js, pnpm, and Rust) is managed with [mise](https://mise.jdx.dev/). The pinned versions live in [`mise.toml`](./mise.toml).

- **mise** — install it by following the [mise getting-started guide](https://mise.jdx.dev/getting-started.html)
- **Tauri system dependencies** for your OS — follow the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)

## Getting started

1. Install the pinned toolchain (Node, pnpm, Rust) from `mise.toml`:
   ```sh
   mise install
   ```

2. Install JS dependencies:
   ```sh
   mise run install
   ```

3. Run the app in development mode (starts Vite and the Tauri window with hot reload):
   ```sh
   mise run dev
   ```

   To run only the frontend in the browser (without the desktop shell):
   ```sh
   pnpm dev
   ```
   This serves on http://localhost:1420.

## Available scripts

| Command | Description |
| --- | --- |
| `pnpm tauri dev` | Run the full desktop app with hot reload |
| `pnpm dev` | Run the frontend only (Vite dev server) |
| `pnpm build` | Type-check and build the frontend |
| `pnpm tauri build` | Build the production desktop bundle |
| `pnpm preview` | Preview the built frontend |
| `pnpm lint` | Check formatting and lint rules (Biome) |
| `pnpm lint:fix` | Auto-fix lint and formatting issues |
| `pnpm test` | Run the test suite once (Vitest) |
| `pnpm test:watch` | Run tests in watch mode |
| `pnpm knip` | Check for unused files, dependencies, and exports |

## Before you open a pull request

Run these checks locally and make sure they pass:

```sh
pnpm lint
pnpm test
pnpm build
```

## Submitting a pull request

1. Create a branch off the default branch:
   ```sh
   git checkout -b feat/short-description
   ```
2. Make your changes, keeping commits focused
3. Push your branch and open a pull request against the main repository.
4. Fill out the PR template and confirm the checklist

A maintainer will review your PR. Please keep the description clear about what changed and why.

## License

By contributing, you agree that your contributions are licensed under the
[PolyForm Noncommercial License 1.0.0](./LICENSE), the same license as the project.
