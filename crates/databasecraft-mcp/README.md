# `databasecraft-mcp` (L2 Dual-Mode CLI & MCP Stdio Server)

Headless CLI and line-delimited `stdio` Model Context Protocol (`MCP 2026-07-28`) server for **DatabaseCraft** on registered control port `14113`.

```powershell
cargo run -p databasecraft-mcp -- status
cargo run -p databasecraft-mcp -- watchdog
cargo run -p databasecraft-mcp -- notifications
cargo run -p databasecraft-mcp -- seed-demo --name ecommerce
cargo run -p databasecraft-mcp -- schema-relations --table categories
cargo run -p databasecraft-mcp -- query --sql "SELECT id, name FROM categories;"
cargo run -p databasecraft-mcp -- export-merge --query "SELECT sku, title, price, hero_image FROM products;" --target designcraft
cargo run -p databasecraft-mcp -- mcp
```
