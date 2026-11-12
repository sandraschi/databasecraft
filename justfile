set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

# DatabaseCraft Automation (Pure-Rust ArtCraft pattern)

default:
    @just --list

# Strict ArtCraft lint gate across all workspace members
check:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo clippy --workspace --all-targets --all-features -- -D warnings

# Unit + integration tests
test:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo test --workspace

# Formatting gate
fmt-check:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo fmt --all --check

# Start MCP server locally over stdio
run-mcp:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p databasecraft-mcp -- mcp

# CLI smoke: status + seed demo + query
run-cli:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p databasecraft-mcp -- status; cargo run -p databasecraft-mcp -- seed-demo --name ecommerce; cargo run -p databasecraft-mcp -- query --sql "SELECT sku, title, price FROM products;"

# Production build
build-release:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo build --release --workspace

# Headless check: app and CLI smoke from scripts/craft.config.psd1 (throwaway database).
smoke:
    powershell -NoProfile -ExecutionPolicy Bypass -File start.ps1 -Smoke

# Build and stage dist/ (plain exes, installer, launcher zip, SHA256SUMS). Nothing is published.
release-dry:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release.ps1

# Same, then tag, push the tag and upload to GitHub (clean, pushed trunk required).
release:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release.ps1 -Publish
