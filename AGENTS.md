# DatabaseCraft - AGENTS.md

**Parent Hub:** [`mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md`](../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)
**Containment:** [`mcp-central-docs/standards/rust-artcraft/CONTAINMENT_AND_CRATES.md`](../mcp-central-docs/standards/rust-artcraft/CONTAINMENT_AND_CRATES.md)
**Command Bus:** [`mcp-central-docs/standards/rust-artcraft/CRAFT_COMMAND_BUS.md`](../mcp-central-docs/standards/rust-artcraft/CRAFT_COMMAND_BUS.md)

## Role in the Storytold ArtCraft Suite

DatabaseCraft is the pure-Rust relational database, schema engine, and data merge keystone for the ArtCraft creative suite:
- **Registered Control Port:** `14113`
- **File Container:** `.dbcraft` (SQLite-backed relational bundle with embedded schema metadata, query definitions, and form layouts)
- **Identity:** Relational data, schemas, and dynamic forms for creative pipelines
- **Color:** Deep Emerald / Obsidian Cyan (`#108B7E`)

## 5-Layer Containment Harness

1. **Layer 1 (Zero-Panic Policy):** All crate roots declare `clippy::deny(unwrap_used, expect_used, panic, unimplemented, todo)`. All errors return `Result<T, DatabaseCraftError>`.
2. **Layer 2 (Three-Tier Workspace Seam):**
   - `crates/databasecraft-store`: L1 storage engine wrapping `rusqlite` (WAL mode, foreign keys ON).
   - `crates/databasecraft-core`: L1 models, cross-craft exports, demo seeds, and universal `Command::dispatch`.
   - `crates/databasecraft-mcp`: L2 CLI-first binary (`databasecraft-cli`) + MCP `stdio` server (`MCP 2024-11-05`).
   - `crates/databasecraft-app`: L3 desktop app calling `databasecraft-core::dispatch` in-process with zero WebView2.
3. **Layer 3 (The Universal Command Bus):** CLI subcommands, MCP `tools/call`, and UI actions all route through `Command::dispatch`.
4. **Layer 4 (Deterministic Demo Datasets):** `ecommerce`, `film`, and `kpi` deterministic datasets provide immediate verifiable schemas.
5. **Layer 5 (Definition of Done):** `just check`, `just test`, `just fmt-check` must be green.

## Stdio Invariant for MCP

In `databasecraft-cli mcp` mode:
- `stdout` is **100% pure line-delimited JSON-RPC**.
- Every log line or diagnostic MUST go to `stderr` (`eprintln!`).

## Multi-agent working rules (several agents and IDEs)

Several agents (Claude Code, OpenCode, Cursor, Windsurf and others) work on this repo from
different IDEs, sometimes at the same time. These rules exist because that went wrong once:
one agent ran a commit-all and swept another agent's half-finished manifest edits into a
release and a pushed tag, leaving the pushed tree unbuildable.

Fleet standard with the full reasoning: `mcp-central-docs/standards/MULTI_AGENT_CONCURRENCY.md`.

### Before you start

1. Run `git status` and `git log -5 --format="%h %ad %s" --date=iso`. If the tree is dirty and
   you did not make those changes, someone else is working here. Leave their files alone:
   do not reset, clean, stash or revert them.
2. Prefer your own branch (`agent/<tool>-<topic>`) or a worktree
   (`git worktree add ..\<repo>-<topic> -b agent/<tool>-<topic>`). Merge to the default branch
   only when the work is finished and checked.

### Committing

- Never `git add -A`, `git add .`, `git commit -a` or `git commit -am`. Name your paths:
  `git add <path> ...` then `git commit -m "..." -- <path> ...`.
- Before committing run `git diff --cached --stat` and check every file is yours.
- A manifest (`Cargo.toml`, `package.json`, `pyproject.toml`) and the files it points at go in the
  same commit. Never commit a manifest that names something not in the commit.
- Do not tag or release from a dirty tree, or from a `WIP-Auto` commit made by the nightly sweep.
  Before tagging, a clean clone of the exact commit must build and pass tests.
- Do not move or delete a published tag; add a new one. Do not force-push a shared branch.
- Do not push another agent's commits you did not read.

### Hooks

`.githooks/pre-commit` rejects a commit whose Cargo manifests name workspace members or path
dependencies that are not tracked. Enable it once per clone:

    git config core.hooksPath .githooks

Do not bypass it with `--no-verify`. If it blocks you, the commit really is broken.

### Leave a trail

- Commit messages say what and why. Agent commits end with the tool's own trailer.
- Long-running work: note it in the repo's worklog or plan file so the next agent sees it.
- At the end of a session, say what you left uncommitted and whose it appears to be.
