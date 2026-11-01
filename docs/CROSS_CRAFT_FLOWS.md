# DatabaseCraft — Cross-Craft Multi-App Integration

**Status**: Active Standard (2026-11-01)  
**Parent Standard**: [`mcp-central-docs/patterns/CRAFT_MULTI_APP_FLOWS.md`](../../mcp-central-docs/patterns/CRAFT_MULTI_APP_FLOWS.md)  
**Registered Control Channel**: `databasecraft --control 14113`

---

## 1. Overview

DatabaseCraft operates as the **central relational data, schema, and metadata engine** for the 13 ArtCraft applications. Rather than forcing creative tools to parse loose JSON or brittle spreadsheets, DatabaseCraft provides typed, query-backed data payloads formatted specifically for each target app's native layout engine.

---

## 2. Cross-Craft Export Targets

DatabaseCraft natively generates five application-specific export formats via `Command::ExportMerge`:

### 2.1 DesignCraft (`Target::DesignCraft`)
* **Purpose**: Multi-page catalog, brochure, and magazine spreads.
* **Payload Structure**:
  ```json
  {
    "app": "designcraft",
    "format": "data_merge",
    "dataset": "spring_catalog",
    "fields": ["sku", "title", "price", "hero_image"],
    "record_count": 2,
    "records": [
      {
        "sku": "AUD-900",
        "title": "Obsidian Field Monitor Headphones",
        "price": 349.00,
        "hero_image": "assets/products/aud-900.png"
      }
    ]
  }
  ```
* **DesignCraft Ingestion**: Place variable text frames bound to `@title`, `@price`, and dynamic image frames bound to `@hero_image`.

### 2.2 DeckCraft (`Target::DeckCraft`)
* **Purpose**: Live pitch decks, board presentations, and automated financial reviews.
* **Payload Structure**:
  ```json
  {
    "app": "deckcraft",
    "format": "slide_table",
    "title": "Quarterly Performance",
    "slide_count": 1,
    "slides": [
      {
        "header": "Quarterly Performance",
        "table": {
          "columns": ["quarter", "arr_usd", "gross_margin_pct"],
          "rows": [["2026-Q1", 1580000.0, 81.2]]
        }
      }
    ]
  }
  ```

### 2.3 WordCraft (`Target::WordCraft`)
* **Purpose**: Contract assembly, personalized invitations, certificates, and reports.
* **Payload Structure**:
  ```json
  {
    "app": "wordcraft",
    "format": "mail_merge",
    "template_dataset": "client_invoices",
    "entries": [
      { "client_name": "Studio Vienna", "amount": 2450.00 }
    ]
  }
  ```

### 2.4 GridCraft (`Target::GridCraft`)
* **Purpose**: High-throughput tabular interchange between relational tables and ad-hoc calculation sheets.
* **Payload Structure**: Standard RFC-4180 CSV string with escaped delimiters.

### 2.5 PdfCraft (`Target::PdfCraft`)
* **Purpose**: Direct print-ready structured ledger, audit, and inventory reports.
* **Payload Structure**: Report table payload with metadata timestamp, headers, and formatted rows.

---

## 3. The Three New Multi-App Production Flows

Registered in [`CRAFT_MULTI_APP_FLOWS.md`](../../mcp-central-docs/patterns/CRAFT_MULTI_APP_FLOWS.md):

### Flow 13: Product Catalog Factory
```
db:product-query (14113) -> photo:plate-grade (14109) -> design:print-spread (14102) -> pdf:portfolio (14108)
```
1. `databasecraft-cli` queries in-stock items with SKU, price, and raw image paths.
2. `photocraft-cli` batches non-destructive color grade on hero product plates.
3. `designcraft-cli` merges graded plates and pricing into facing catalog spreads.
4. `pdfcraft-cli` bundles print-ready high-DPI deliverables with audit verification.

### Flow 14: Film Production Asset Logger
```
db:call-sheet-query (14113) -> sound:ident-jingle (14110) -> film:timeline-cut (14105) -> pdf:portfolio (14108)
```
1. `databasecraft-cli` retrieves scene/take metadata and audio timecodes for today's shoot.
2. `soundcraft-cli` bounces synced audio slate stems and audio bed.
3. `filmcraft-cli` cuts timeline sequence conforming to logged print takes.
4. `pdfcraft-cli` generates official production wrap report with signed shot notes.

### Flow 15: Executive Board Metric Deck
```
db:revenue-kpis (14113) -> grid:homepage-grid (14106) -> deck:pitch-deck (14101) -> pdf:portfolio (14108)
```
1. `databasecraft-cli` runs analytical SQL aggregation across revenue and customer cohorts.
2. `gridcraft-cli` calculates financial projections and model formulas.
3. `deckcraft-cli` auto-populates slide graphs and KPI callouts.
4. `pdfcraft-cli` exports board deck for investor distribution.
