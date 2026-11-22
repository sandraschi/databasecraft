# Storytold Beta Readiness TODO - DatabaseCraft

**Target:** Submit to https://getartcraft.com/beta (Tier A Candidate #6 - Visual SQLite/Postgres/DuckDB studio & form/report builder)
**Audited:** 2026-11-21 (17.9k LOC, 38 tests, zero external sibling path deps)
**Reference:** `mcp-central-docs/integrations/storytold-artcraft/BETA_CANDIDATES.md`

---

## Already In Place
- [x] `LICENSE-MIT`, `LICENSE-APACHE`, `AGENTS.md`
- [x] Zero external `../sibling` path dependencies (fresh-clone safe)
- [x] 17.9k LOC substantial pure-Rust engine

---

## P0 - Hard Submission Blockers (Must Complete Before Beta Form)

- [ ] **Product reframing (README + UI)**
  - Reframe away from "24/7 Executive/CFO Relational Cockpit" and fleet-specific control port `14113` into a clean, general-purpose **pure-Rust desktop relational database studio** (schema designer, SQL editor, data grid, forms/reports, CSV/JSON/Parquet import-export) that complements `WordCraft`, `GridCraft`, and `DeckCraft`.
  - Ensure zero ports are opened by default on native desktop startup.
- [ ] **Legal & governance files**
  - [ ] Add `NOTICE` (explicit non-affiliation disclaimer).
  - [ ] Add `ATTRIBUTION.md` (all dependency licenses).
  - [ ] Verify `Cargo.lock` against Storytold `standards/licences.md` SPDX allowlist.
- [ ] **Add `xtask` (`cargo xtask ci`)**: `fmt --check`, `clippy --all-targets -- -D warnings` (`#![forbid(unsafe_code)]`, `unwrap_used`, `expect_used`, `panic`), `cargo test`, layering check, and license check.
- [ ] **Add GitHub Actions CI & Release workflows (`.github/workflows/ci.yml`, `release.yml`)**
  - Matrix build + test on `ubuntu-latest`, `macos-latest`, `windows-latest`.
  - Release workflow publishes cross-platform native binaries + `SHA256SUMS.txt`.
- [ ] **Add `packaging/env.sh`**.

## P1 - Test Floor, Naming & Feature Honesty

- [ ] **Raise test floor from 38 to >= 50 tests** (T2 bar; add hostile-SQL/corrupt-DB/migration/import-export invariant tests per RFC-0071).
- [ ] **Add `docs/feature-names.md`** (RFC-0029 trademark audit; avoid third-party database-IDE brand names in UI/marketing).
- [ ] **Add `docs/target-app-parity.md` and `docs/gaps.md`** (Implemented / Partial / Planned matrix across SQLite, PostgreSQL, DuckDB, schema visual editor, query runner, forms/reports).
- [ ] **Sync `AGENTS.md`** with `storytold/craft-contrib/standards/app-agents.md` (RFC-0072).

## P2 - Polish, Accessibility & Evidence

- [ ] **Real desktop UI screenshots (dark + light modes)** in `docs/images/` linked from `README.md` (currently 0 images committed; show schema diagram + SQL query editor + data grid).
- [ ] **Accessibility check (RFC-0014)**: full keyboard navigation for SQL editor, grid cells, and dialogs; high-contrast light mode.
- [ ] **Clean fresh-clone build**: verify `cargo xtask ci` and `--smoke` on a fresh checkout.
- [ ] **Submit** 4-field form at https://getartcraft.com/beta (App name, Email, Repo URL, Description).
