use databasecraft_core::{dispatch, models::ExportTarget, Command, CONTROL_PORT};
use databasecraft_store::DatabaseStore;

#[tokio::test]
async fn test_databasecraft_status_and_control_port() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    let res = dispatch(Command::Status, &store).await;
    assert_eq!(res["app"], "databasecraft");
    assert_eq!(res["control_port"], CONTROL_PORT);
    assert_eq!(CONTROL_PORT, 14113);
    Ok(())
}

#[tokio::test]
async fn test_seed_ecommerce_and_query() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    let seed_res = dispatch(
        Command::SeedDemo {
            name: "ecommerce".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(seed_res["success"], true);

    let query_res = dispatch(
        Command::Query {
            sql: "SELECT sku, title, price FROM products WHERE inventory > 10 ORDER BY price ASC;"
                .to_string(),
        },
        &store,
    )
    .await;

    assert_eq!(query_res["success"], true);
    assert_eq!(query_res["row_count"], 2);
    let columns = query_res["columns"]
        .as_array()
        .ok_or("missing columns array")?;
    assert_eq!(columns.first().and_then(|v| v.as_str()), Some("sku"));
    assert_eq!(columns.get(1).and_then(|v| v.as_str()), Some("title"));
    assert_eq!(columns.get(2).and_then(|v| v.as_str()), Some("price"));
    Ok(())
}

#[tokio::test]
async fn test_export_merge_designcraft() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    dispatch(
        Command::SeedDemo {
            name: "ecommerce".to_string(),
        },
        &store,
    )
    .await;

    let res = dispatch(
        Command::ExportMerge {
            query: "SELECT sku, title, price, hero_image FROM products ORDER BY id ASC LIMIT 2;"
                .to_string(),
            target: ExportTarget::DesignCraft,
            name: Some("catalog_spread".to_string()),
        },
        &store,
    )
    .await;

    assert_eq!(res["success"], true);
    assert_eq!(res["export"]["app"], "designcraft");
    assert_eq!(res["export"]["format"], "data_merge");
    assert_eq!(res["export"]["record_count"], 2);
    Ok(())
}

#[tokio::test]
async fn test_export_merge_deckcraft() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    dispatch(
        Command::SeedDemo {
            name: "kpi".to_string(),
        },
        &store,
    )
    .await;

    let res = dispatch(
        Command::ExportMerge {
            query: "SELECT quarter, arr_usd, mrr_usd FROM quarterly_metrics;".to_string(),
            target: ExportTarget::DeckCraft,
            name: Some("board_deck".to_string()),
        },
        &store,
    )
    .await;

    assert_eq!(res["success"], true);
    assert_eq!(res["export"]["app"], "deckcraft");
    assert_eq!(res["export"]["format"], "slide_table");
    Ok(())
}

#[tokio::test]
async fn test_hostile_input_fence() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    // SQL injection attempt in table name
    let res = dispatch(
        Command::RecordDelete {
            table: "products; DROP TABLE products;--".to_string(),
            id: 1,
        },
        &store,
    )
    .await;

    assert_eq!(res["success"], false);
    let err_msg = res["error"].as_str().ok_or("expected error message")?;
    assert!(err_msg.contains("illegal characters"));
    Ok(())
}

#[tokio::test]
async fn test_vector_insert_and_cosine_search() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    let init_res = dispatch(
        Command::VectorInit {
            table: "assets_vec".to_string(),
            dimensions: 4,
        },
        &store,
    )
    .await;
    assert_eq!(init_res["success"], true);

    // Insert two vectors: vector 1 is close to [1.0, 0.0, 0.0, 0.0], vector 2 is orthogonal
    dispatch(
        Command::VectorInsert {
            table: "assets_vec".to_string(),
            id: 1,
            vector: vec![0.95, 0.05, 0.0, 0.0],
            metadata: serde_json::json!({ "title": "Warm Sunset Landscape", "app": "photocraft" }),
        },
        &store,
    )
    .await;

    dispatch(
        Command::VectorInsert {
            table: "assets_vec".to_string(),
            id: 2,
            vector: vec![0.0, 0.0, 0.98, 0.02],
            metadata: serde_json::json!({ "title": "Cyberpunk Neon City", "app": "vectorcraft" }),
        },
        &store,
    )
    .await;

    // Query for something close to vector 1
    let search_res = dispatch(
        Command::VectorSearch {
            table: "assets_vec".to_string(),
            vector: vec![1.0, 0.0, 0.0, 0.0],
            metric: Some(databasecraft_store::VectorMetric::Cosine),
            top_k: Some(1),
        },
        &store,
    )
    .await;

    assert_eq!(search_res["success"], true);
    assert_eq!(search_res["count"], 1);
    let hits = search_res["hits"].as_array().ok_or("missing hits")?;
    let top_hit = hits.first().ok_or("missing top hit")?;
    assert_eq!(top_hit["id"], 1);
    assert!(top_hit["score"].as_f64().unwrap_or(0.0) > 0.99);
    assert_eq!(top_hit["metadata"]["app"], "photocraft");
    Ok(())
}

#[tokio::test]
async fn test_hybrid_rag_search() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;

    // 1. Vector table
    dispatch(
        Command::VectorInit {
            table: "film_scenes_vec".to_string(),
            dimensions: 3,
        },
        &store,
    )
    .await;

    dispatch(
        Command::VectorInsert {
            table: "film_scenes_vec".to_string(),
            id: 101,
            vector: vec![0.9, 0.1, 0.0],
            metadata: serde_json::json!({ "scene": "Vienna Rooftop Dawn" }),
        },
        &store,
    )
    .await;

    // 2. FTS table
    dispatch(
        Command::FtsInit {
            table: "film_scenes_fts".to_string(),
            columns: vec!["title".to_string(), "notes".to_string()],
        },
        &store,
    )
    .await;

    store.execute(
        "INSERT INTO film_scenes_fts (rowid, title, notes) VALUES (101, 'Vienna Rooftop Dawn', 'Hero master take 35mm');"
    )?;

    // 3. Hybrid search
    let hybrid_res = dispatch(
        Command::HybridSearch {
            vector_table: "film_scenes_vec".to_string(),
            fts_table: "film_scenes_fts".to_string(),
            query_text: "Rooftop Dawn".to_string(),
            query_vector: vec![1.0, 0.0, 0.0],
            top_k: Some(5),
        },
        &store,
    )
    .await;

    assert_eq!(hybrid_res["success"], true);
    let hits = hybrid_res["hits"].as_array().ok_or("missing hybrid hits")?;
    let top = hits.first().ok_or("missing top hit")?;
    assert_eq!(top["id"], 101);
    assert!(top["rrf_score"].as_f64().unwrap_or(0.0) > 0.0);
    Ok(())
}

#[tokio::test]
async fn test_command_registry_and_describe() -> Result<(), Box<dyn std::error::Error>> {
    let registry = databasecraft_core::command_registry();
    assert_eq!(registry.len(), 37);

    for cmd in &registry {
        assert!(!cmd.id.is_empty());
        assert!(!cmd.label.is_empty());
        assert!(!cmd.description.is_empty());
    }

    let status_desc = databasecraft_core::describe_command("status").ok_or("missing status")?;
    assert_eq!(status_desc.id, "status");
    assert_eq!(status_desc.label, "App Status");

    let query_desc = databasecraft_core::describe_command("db.query").ok_or("missing db.query")?;
    assert_eq!(query_desc.id, "db.query");

    let export_desc =
        databasecraft_core::describe_command("db.export_merge").ok_or("missing db.export_merge")?;
    assert_eq!(export_desc.id, "db.export_merge");

    let hybrid_desc = databasecraft_core::describe_command("rag.hybrid_search")
        .ok_or("missing rag.hybrid_search")?;
    assert_eq!(hybrid_desc.id, "rag.hybrid_search");

    let embed_desc =
        databasecraft_core::describe_command("rag.embed_text").ok_or("missing rag.embed_text")?;
    assert_eq!(embed_desc.id, "rag.embed_text");

    let ask_desc = databasecraft_core::describe_command("rag.semantic_search")
        .ok_or("missing rag.semantic_search")?;
    assert_eq!(ask_desc.id, "rag.semantic_search");

    let unknown = databasecraft_core::describe_command("nonexistent_command");
    assert!(unknown.is_none());

    Ok(())
}

#[tokio::test]
async fn test_rag_embed_and_semantic_search() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;

    // 1. Test in-process embedding generation
    let embed_res = dispatch(
        Command::RagEmbed {
            text: "wireless noise cancelling headphones".to_string(),
            dimensions: Some(64),
        },
        &store,
    )
    .await;
    assert_eq!(embed_res["success"], true);
    assert_eq!(embed_res["dimensions"], 64);
    let vec_arr = embed_res["vector"]
        .as_array()
        .ok_or("missing vector array")?;
    assert_eq!(vec_arr.len(), 64);

    // 2. Setup vector table and insert semantic record
    dispatch(
        Command::VectorInit {
            table: "catalog_vec".to_string(),
            dimensions: 64,
        },
        &store,
    )
    .await;

    let headphones_vec =
        databasecraft_store::generate_text_embedding("wireless noise cancelling headphones", 64);
    dispatch(
        Command::VectorInsert {
            table: "catalog_vec".to_string(),
            id: 42,
            vector: headphones_vec,
            metadata: serde_json::json!({ "title": "Acoustic Pro 900", "price": 299.0 }),
        },
        &store,
    )
    .await;

    // 3. Setup FTS5 table
    dispatch(
        Command::FtsInit {
            table: "catalog_fts".to_string(),
            columns: vec!["title".to_string()],
        },
        &store,
    )
    .await;
    store.execute(
        "INSERT INTO catalog_fts (rowid, title) VALUES (42, 'Acoustic Pro 900 Headphones');",
    )?;

    // 4. Test high-level semantic search using raw text query (no upstream float array required!)
    let search_res = dispatch(
        Command::RagAsk {
            vector_table: "catalog_vec".to_string(),
            fts_table: Some("catalog_fts".to_string()),
            query: "wireless headphones".to_string(),
            dimensions: Some(64),
            top_k: Some(5),
        },
        &store,
    )
    .await;

    assert_eq!(search_res["success"], true);
    assert_eq!(search_res["mode"], "hybrid");
    let hits = search_res["hits"].as_array().ok_or("missing hits")?;
    let top = hits.first().ok_or("missing top hit")?;
    assert_eq!(top["id"], 42);
    assert_eq!(top["metadata"]["title"], "Acoustic Pro 900");

    Ok(())
}

#[tokio::test]
async fn test_ai_nl_to_sql_and_profile() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    dispatch(
        Command::SeedDemo {
            name: "ecommerce".to_string(),
        },
        &store,
    )
    .await;

    // Test NLQ
    let nlq_res = dispatch(
        Command::AiNlToSql {
            prompt: "find products with price > 100 limit 5".to_string(),
        },
        &store,
    )
    .await;

    assert_eq!(nlq_res["success"], true);
    assert!(nlq_res["sql"].as_str().unwrap_or("").contains("SELECT"));
    assert!(nlq_res["sql"].as_str().unwrap_or("").contains("products"));
    assert_eq!(nlq_res["read_only"], true);

    // Test DataProfile
    let profile_res = dispatch(
        Command::DataProfile {
            table: "products".to_string(),
        },
        &store,
    )
    .await;

    assert_eq!(profile_res["success"], true);
    assert_eq!(profile_res["profile"]["table"], "products");
    assert!(profile_res["profile"]["total_rows"].as_u64().unwrap_or(0) > 0);

    Ok(())
}

#[tokio::test]
async fn test_ai_schema_advisor_and_synthetic() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    dispatch(
        Command::SeedDemo {
            name: "ecommerce".to_string(),
        },
        &store,
    )
    .await;

    // Test Schema Advisor
    let adv_res = dispatch(Command::AiSchemaAdvisor, &store).await;
    assert_eq!(adv_res["success"], true);
    assert!(adv_res["total_tables"].as_u64().unwrap_or(0) >= 2);

    // Test Synthetic Generator
    let syn_res = dispatch(
        Command::AiGenerateSynthetic {
            table: "products".to_string(),
            count: 3,
            insert: Some(true),
        },
        &store,
    )
    .await;

    assert_eq!(syn_res["success"], true);
    assert_eq!(syn_res["inserted"], true);
    let records = syn_res["records"].as_array().ok_or("missing records")?;
    assert_eq!(records.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_ai_document_ingest_and_dedupe() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;

    // Test Document Ingest
    let doc_content = "# Scene 1\nCamera tracks actor across neon bridge.\n# Scene 2\nActor enters antique store.";
    let ingest_res = dispatch(
        Command::AiIngestDocument {
            title: "Cyberpunk Script Draft".to_string(),
            content: doc_content.to_string(),
            tags: Some(vec!["film".to_string(), "cyberpunk".to_string()]),
        },
        &store,
    )
    .await;

    assert_eq!(ingest_res["success"], true);
    assert_eq!(ingest_res["chunks_indexed"], 2);

    // Test Semantic Dedupe
    store.execute("CREATE TABLE tags (id INTEGER PRIMARY KEY, label TEXT);")?;
    store.execute("INSERT INTO tags VALUES (1, 'Wireless Bluetooth Headphones');")?;
    store.execute("INSERT INTO tags VALUES (2, 'Bluetooth Wireless Headphone');")?;
    store.execute("INSERT INTO tags VALUES (3, 'Ceramic Coffee Mug');")?;

    let dedupe_res = dispatch(
        Command::AiSemanticDedupe {
            table: "tags".to_string(),
            column: "label".to_string(),
            threshold: Some(0.60),
        },
        &store,
    )
    .await;

    assert_eq!(dedupe_res["success"], true);
    assert!(dedupe_res["duplicates_found"].as_u64().unwrap_or(0) >= 1);

    Ok(())
}

#[tokio::test]
async fn test_diagnose_and_repair_inconsistent_database() -> Result<(), Box<dyn std::error::Error>>
{
    let store = DatabaseStore::open_in_memory()?;

    // 1. Initially healthy
    let diag1 = dispatch(Command::Diagnose, &store).await;
    assert_eq!(diag1["success"], true);
    assert_eq!(diag1["report"]["status"], "HEALTHY");

    // 2. Introduce deliberate relational inconsistency: disable FK temporarily to insert orphan
    store.execute("CREATE TABLE authors (id INTEGER PRIMARY KEY, name TEXT);")?;
    store.execute("CREATE TABLE books (id INTEGER PRIMARY KEY, title TEXT, author_id INTEGER REFERENCES authors(id));")?;

    // Insert orphan book referencing non-existent author_id 999
    store.execute("PRAGMA foreign_keys = OFF;")?;
    store.execute("INSERT INTO books VALUES (1, 'Orphaned Novel', 999);")?;
    store.execute("PRAGMA foreign_keys = ON;")?;

    // 3. Diagnose should detect inconsistency as DEGRADED
    let diag2 = dispatch(Command::Diagnose, &store).await;
    assert_eq!(diag2["success"], true);
    assert_eq!(diag2["report"]["status"], "DEGRADED");
    let violations = diag2["report"]["fk_violations"]
        .as_array()
        .ok_or("missing fk_violations")?;
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0]["table"], "books");
    assert_eq!(violations[0]["target_table"], "authors");

    // 4. Auto-repair should purge orphan and return to HEALTHY
    let repair_res = dispatch(
        Command::Repair {
            action: "auto".to_string(),
            salvage_path: None,
        },
        &store,
    )
    .await;

    assert_eq!(repair_res["success"], true);
    assert_eq!(repair_res["report"]["orphans_purged"], 1);
    assert_eq!(repair_res["report"]["final_health_status"], "HEALTHY");

    // 5. Verify database is once again HEALTHY
    let diag3 = dispatch(Command::Diagnose, &store).await;
    assert_eq!(diag3["report"]["status"], "HEALTHY");

    Ok(())
}

#[tokio::test]
async fn test_blueprint_constructor_facilities() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Check list of blueprints
    let store = DatabaseStore::open_in_memory()?;
    let list_res = dispatch(Command::ListBlueprints, &store).await;
    assert_eq!(list_res["success"], true);
    let bps = list_res["blueprints"]
        .as_array()
        .ok_or("missing blueprints")?;
    assert_eq!(bps.len(), 11);

    // 2. Construct Small Business
    let biz_store = DatabaseStore::open_in_memory()?;
    let biz_res = dispatch(
        Command::Construct {
            blueprint: "small_business".to_string(),
            seed_sample_data: Some(true),
            enable_search_indexes: Some(true),
        },
        &biz_store,
    )
    .await;
    assert_eq!(biz_res["success"], true);
    assert_eq!(biz_res["construction"]["blueprint_id"], "small_business");
    assert_eq!(
        biz_res["construction"]["tables_created"]
            .as_array()
            .map(|a| a.len()),
        Some(5)
    );
    assert!(
        biz_res["construction"]["sample_records_seeded"]
            .as_u64()
            .unwrap_or(0)
            > 0
    );

    // 3. Construct Medical Surgery
    let surgery_store = DatabaseStore::open_in_memory()?;
    let surgery_res = dispatch(
        Command::Construct {
            blueprint: "surgery".to_string(),
            seed_sample_data: Some(true),
            enable_search_indexes: Some(true),
        },
        &surgery_store,
    )
    .await;
    assert_eq!(surgery_res["success"], true);
    assert_eq!(surgery_res["construction"]["blueprint_id"], "surgery");
    assert_eq!(
        surgery_res["construction"]["tables_created"]
            .as_array()
            .map(|a| a.len()),
        Some(6)
    );

    // 4. Construct Real Estate & Automotive Blueprints
    let prop_store = DatabaseStore::open_in_memory()?;
    let prop_res = dispatch(
        Command::Construct {
            blueprint: "real_estate".to_string(),
            seed_sample_data: Some(true),
            enable_search_indexes: Some(true),
        },
        &prop_store,
    )
    .await;
    assert_eq!(prop_res["success"], true);
    assert_eq!(prop_res["construction"]["blueprint_id"], "real_estate");

    let auto_store = DatabaseStore::open_in_memory()?;
    let auto_res = dispatch(
        Command::Construct {
            blueprint: "automotive".to_string(),
            seed_sample_data: Some(true),
            enable_search_indexes: Some(true),
        },
        &auto_store,
    )
    .await;
    assert_eq!(auto_res["success"], true);
    assert_eq!(auto_res["construction"]["blueprint_id"], "automotive");

    // Verify patients and appointments query works in surgery DB
    let query_res = dispatch(
        Command::Query {
            sql: "SELECT p.full_name, a.chief_complaint, d.full_name AS doctor FROM appointments a JOIN patients p ON a.patient_id = p.id JOIN practitioners d ON a.practitioner_id = d.id;".to_string(),
        },
        &surgery_store,
    )
    .await;
    assert_eq!(query_res["success"], true);
    assert!(query_res["row_count"].as_u64().unwrap_or(0) >= 1);

    // Verify diagnosed healthy
    let diag = dispatch(Command::Diagnose, &surgery_store).await;
    assert_eq!(diag["report"]["status"], "HEALTHY");

    Ok(())
}

#[tokio::test]
async fn test_blueprint_billing_stripe_mirror() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    let res = dispatch(
        Command::Construct {
            blueprint: "billing".to_string(),
            seed_sample_data: Some(true),
            enable_search_indexes: Some(true),
        },
        &store,
    )
    .await;
    assert_eq!(res["success"], true);
    assert_eq!(res["construction"]["blueprint_id"], "billing");
    assert_eq!(
        res["construction"]["tables_created"]
            .as_array()
            .map(|a| a.len()),
        Some(5)
    );
    assert_eq!(res["construction"]["sample_records_seeded"], 13);

    // Seed covers the subscription lifecycle states the reconcile logic needs.
    let subs = dispatch(
        Command::Query {
            sql: "SELECT stripe_subscription_id, status FROM subscriptions ORDER BY id".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(subs["success"], true);
    assert_eq!(subs["row_count"], 3);
    let statuses: Vec<String> = subs["rows"]
        .as_array()
        .ok_or("missing rows")?
        .iter()
        .filter_map(|r| r.as_array()?.get(1)?.as_str().map(|s| s.to_string()))
        .collect();
    assert!(statuses.contains(&"active".to_string()));
    assert!(statuses.contains(&"past_due".to_string()));
    assert!(statuses.contains(&"canceled".to_string()));

    // FTS indexes registered for customer/product search.
    let fts = res["construction"]["fts_tables_created"]
        .as_array()
        .ok_or("missing fts tables")?;
    assert_eq!(fts.len(), 2);
    Ok(())
}

#[tokio::test]
async fn test_canvas_schema_relations() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    dispatch(
        Command::SeedDemo {
            name: "ecommerce".to_string(),
        },
        &store,
    )
    .await;

    // Test reverse relation on categories (categories <- products)
    let cat_rel = dispatch(
        Command::SchemaRelations {
            table: "categories".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(cat_rel["success"], true);
    assert_eq!(cat_rel["table"], "categories");
    let reverse = cat_rel["reverse_relations"]
        .as_array()
        .ok_or("missing reverse_relations")?;
    assert_eq!(reverse.len(), 1);
    assert_eq!(reverse[0]["relation_name"], "products");
    assert_eq!(reverse[0]["target_col"], "category_id");
    assert_eq!(reverse[0]["is_reverse"], true);

    // Test forward relation on products (products -> categories)
    let prod_rel = dispatch(
        Command::SchemaRelations {
            table: "products".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(prod_rel["success"], true);
    let forward = prod_rel["forward_relations"]
        .as_array()
        .ok_or("missing forward_relations")?;
    assert_eq!(forward.len(), 1);
    assert_eq!(forward[0]["relation_name"], "categories");
    assert_eq!(forward[0]["source_col"], "category_id");
    assert_eq!(forward[0]["target_col"], "id");
    assert_eq!(forward[0]["is_reverse"], false);

    Ok(())
}

#[tokio::test]
async fn test_canvas_compile_and_query_hierarchy() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    dispatch(
        Command::SeedDemo {
            name: "ecommerce".to_string(),
        },
        &store,
    )
    .await;

    use databasecraft_core::canvas::{
        AggFunc, CanvasAggregation, CanvasColumn, CanvasNestedRelation, CanvasSpec,
    };

    // Build a perspective:
    // Categories with:
    // - columns: id, name
    // - inline rollup aggregate: product_count (COUNT(products)), total_value (SUM(products.price))
    // - nested relation: products with sku, title, price
    let spec = CanvasSpec {
        root_table: "categories".to_string(),
        columns: vec![CanvasColumn::new("id"), CanvasColumn::new("name")],
        inlined_lookups: vec![],
        nested_relations: vec![CanvasNestedRelation {
            relation_name: "products".to_string(),
            target_table: "products".to_string(),
            parent_key_col: "id".to_string(),
            foreign_key_col: "category_id".to_string(),
            is_reverse: true,
            columns: vec![
                CanvasColumn::new("sku"),
                CanvasColumn::new("title"),
                CanvasColumn::new("price"),
            ],
            inlined_lookups: vec![],
            nested_relations: vec![],
            aggregations: vec![],
            filters: vec![],
            limit: None,
        }],
        aggregations: vec![
            CanvasAggregation {
                target_relation: Some("products".to_string()),
                target_col: "id".to_string(),
                func: AggFunc::Count,
                alias: "product_count".to_string(),
            },
            CanvasAggregation {
                target_relation: Some("products".to_string()),
                target_col: "price".to_string(),
                func: AggFunc::Sum,
                alias: "total_category_value".to_string(),
            },
        ],
        filters: vec![],
        sort: vec![],
        limit: Some(10),
        offset: None,
    };

    // 1. Test SQL Compilation
    let comp_res = dispatch(Command::CanvasCompileSql { spec: spec.clone() }, &store).await;
    assert_eq!(comp_res["success"], true);
    let compiled_sql = comp_res["sql"].as_str().ok_or("missing sql")?;
    assert!(compiled_sql.contains("json_group_array"));
    assert!(compiled_sql.contains("json_object"));
    assert!(compiled_sql.contains("total_category_value"));

    // 2. Test Execution and Non-1NF JSON Nesting
    let query_res = dispatch(Command::CanvasQuery { spec }, &store).await;
    assert_eq!(query_res["success"], true);
    let data = query_res["result"]["data"]
        .as_array()
        .ok_or("missing data")?;
    assert_eq!(data.len(), 3); // 3 categories

    // Category 1: Audio Hardware has 2 products (AUD-900: $349, MIC-042: $799 -> total $1148)
    let cat1 = &data[0];
    assert_eq!(cat1["name"], "Audio Hardware");
    assert_eq!(cat1["product_count"], 2);
    assert_eq!(cat1["total_category_value"], 1148.0);

    let cat1_products = cat1["products"]
        .as_array()
        .ok_or("missing nested products")?;
    assert_eq!(cat1_products.len(), 2);
    assert_eq!(cat1_products[0]["sku"], "AUD-900");
    assert_eq!(cat1_products[1]["sku"], "MIC-042");

    // Category 2: Optics & Lenses has 2 products (OPT-35A: $1850, OPT-85A: $2100 -> total $3950)
    let cat2 = &data[1];
    assert_eq!(cat2["name"], "Optics & Lenses");
    assert_eq!(cat2["product_count"], 2);
    assert_eq!(cat2["total_category_value"], 3950.0);

    // Category 3: Studio Furniture has 1 product (DSK-PRO: $2450)
    let cat3 = &data[2];
    assert_eq!(cat3["name"], "Studio Furniture");
    assert_eq!(cat3["product_count"], 1);
    assert_eq!(cat3["total_category_value"], 2450.0);

    Ok(())
}

#[tokio::test]
async fn test_seed_northwind_and_relations() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    let seed_res = dispatch(
        Command::SeedDemo {
            name: "northwind".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(seed_res["success"], true);

    // Test reverse relation on customers -> orders
    let cust_rel = dispatch(
        Command::SchemaRelations {
            table: "customers".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(cust_rel["success"], true);
    let cust_rev = cust_rel["reverse_relations"]
        .as_array()
        .ok_or("missing reverse_relations")?;
    assert!(cust_rev.iter().any(|r| r["relation_name"] == "orders"));

    // Test reverse relation on orders -> order_items
    let order_rel = dispatch(
        Command::SchemaRelations {
            table: "orders".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(order_rel["success"], true);
    let order_rev = order_rel["reverse_relations"]
        .as_array()
        .ok_or("missing order reverse_relations")?;
    assert!(order_rev
        .iter()
        .any(|r| r["relation_name"] == "order_items"));

    Ok(())
}

#[tokio::test]
async fn test_canvas_hostile_input_fence() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    use databasecraft_core::canvas::{CanvasColumn, CanvasSpec};

    // SQL injection attempt via root_table
    let bad_spec = CanvasSpec {
        root_table: "categories; DROP TABLE products;--".to_string(),
        columns: vec![CanvasColumn::new("id")],
        inlined_lookups: vec![],
        nested_relations: vec![],
        aggregations: vec![],
        filters: vec![],
        sort: vec![],
        limit: None,
        offset: None,
    };

    let res = dispatch(Command::CanvasQuery { spec: bad_spec }, &store).await;
    assert_eq!(res["success"], false);
    assert!(res["error"]
        .as_str()
        .unwrap_or("")
        .contains("illegal characters"));
    Ok(())
}

#[tokio::test]
async fn test_notification_bus_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    use databasecraft_core::notifications::{
        global_notification_bus, CraftNotification, NotificationCategory, NotificationSeverity,
    };

    let test_alert = CraftNotification {
        id: "alert-disk-001".to_string(),
        timestamp: "2026-11-10T11:00:00Z".to_string(),
        app: "databasecraft".to_string(),
        severity: NotificationSeverity::Warning,
        category: NotificationCategory::DiskStarvation,
        title: "Disk Space Threshold Warning".to_string(),
        message: "Database storage partition has 480MB remaining".to_string(),
        source_entity: Some("disk:c:/".to_string()),
        metrics: serde_json::json!({ "free_mb": 480, "threshold_mb": 500 }),
        acknowledged: false,
        suggested_action: Some("Run VACUUM to reclaim free space".to_string()),
    };

    global_notification_bus().emit(test_alert);

    // List notifications via command bus
    let list_res = dispatch(
        Command::NotificationsList {
            severity: Some(NotificationSeverity::Warning),
            unacknowledged_only: Some(true),
        },
        &store,
    )
    .await;
    assert_eq!(list_res["success"], true);
    assert!(list_res["count"].as_u64().unwrap_or(0) >= 1);

    // Acknowledge alert
    let ack_res = dispatch(
        Command::NotificationAck {
            id: "alert-disk-001".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(ack_res["success"], true);
    assert_eq!(ack_res["acknowledged"], true);

    // Verify it is no longer unacknowledged
    let post_ack = dispatch(
        Command::NotificationsList {
            severity: None,
            unacknowledged_only: Some(true),
        },
        &store,
    )
    .await;
    let unack_alerts: Vec<String> = post_ack["notifications"]
        .as_array()
        .ok_or("missing notifications")?
        .iter()
        .filter_map(|n| n["id"].as_str().map(|s| s.to_string()))
        .collect();
    assert!(!unack_alerts.contains(&"alert-disk-001".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_watchdog_quiescence_anomaly() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    dispatch(
        Command::SeedDemo {
            name: "ecommerce".to_string(),
        },
        &store,
    )
    .await;

    // Simulate rapid mutations on quiescent table 'categories' (configured threshold is 10/min)
    for _ in 0..15 {
        dispatch(
            Command::WatchdogRecordMutation {
                table: "categories".to_string(),
            },
            &store,
        )
        .await;
    }

    // Run watchdog check
    let check_res = dispatch(Command::WatchdogCheck, &store).await;
    assert_eq!(check_res["success"], true);
    assert!(check_res["new_alerts_emitted"].as_u64().unwrap_or(0) >= 1);

    let alerts = check_res["alerts"].as_array().ok_or("missing alerts")?;
    let quiescence_alert = alerts.iter().find(|a| {
        a["category"] == "quiescence_anomaly" && a["source_entity"] == "table:categories"
    });
    assert!(quiescence_alert.is_some());
    let alert = quiescence_alert.ok_or("missing quiescence alert")?;
    assert!(alert["title"]
        .as_str()
        .unwrap_or("")
        .contains("Quiescence Anomaly"));
    assert!(
        alert["metrics"]["mutations_last_minute"]
            .as_u64()
            .unwrap_or(0)
            >= 15
    );

    Ok(())
}

#[tokio::test]
async fn test_cell_update_dispatch_and_watchdog() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    dispatch(
        Command::SeedDemo {
            name: "ecommerce".to_string(),
        },
        &store,
    )
    .await;

    // Mutate a cell via Command::UpdateCell
    let res = dispatch(
        Command::UpdateCell {
            table: "categories".to_string(),
            pk_col: "id".to_string(),
            pk_val: serde_json::json!(1),
            col: "name".to_string(),
            new_val: serde_json::json!("Super Electronics"),
        },
        &store,
    )
    .await;

    assert_eq!(res["success"], true);
    assert_eq!(res["rows_affected"], 1);

    // Verify raw query reflects the atomic write
    let qr = store.query("SELECT name FROM categories WHERE id = 1;")?;
    assert_eq!(
        qr.rows.first().and_then(|r| r.first()),
        Some(&serde_json::Value::String("Super Electronics".to_string()))
    );

    // Verify hostile input rejection on table and col
    let bad_res = dispatch(
        Command::UpdateCell {
            table: "categories; DROP TABLE users;--".to_string(),
            pk_col: "id".to_string(),
            pk_val: serde_json::json!(1),
            col: "name".to_string(),
            new_val: serde_json::json!("Hacked"),
        },
        &store,
    )
    .await;
    assert_eq!(bad_res["success"], false);

    Ok(())
}

#[tokio::test]
async fn test_canvas_inlined_lookup_compact_join() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    dispatch(
        Command::SeedDemo {
            name: "northwind".to_string(),
        },
        &store,
    )
    .await;

    use databasecraft_core::canvas::{
        suggest_display_column, CanvasColumn, CanvasInlinedLookup, CanvasSpec,
    };

    // Auto-detect display column for customers table
    let display_col = suggest_display_column(&store, "customers")?;
    assert_eq!(display_col, "company_name");

    // Perspective on orders with inlined customer company_name (Compact Join without sprouting sub-table)
    let spec = CanvasSpec {
        root_table: "orders".to_string(),
        columns: vec![
            CanvasColumn::new("id"),
            CanvasColumn::new("customer_id"),
            CanvasColumn::new("order_date"),
        ],
        inlined_lookups: vec![CanvasInlinedLookup {
            local_fk_col: "customer_id".to_string(),
            foreign_table: "customers".to_string(),
            foreign_pk_col: "id".to_string(),
            display_col: "company_name".to_string(),
            alias: "customer_name".to_string(),
            width_px: None,
        }],
        nested_relations: vec![],
        aggregations: vec![],
        filters: vec![],
        sort: vec![],
        limit: Some(10),
        offset: None,
    };

    let res = dispatch(Command::CanvasQuery { spec }, &store).await;
    assert_eq!(res["success"], true);
    let data = res["result"]["data"].as_array().ok_or("missing data")?;
    assert!(!data.is_empty());

    // Verify first order row has the inlined customer_name
    let first_row = &data[0];
    assert!(first_row.get("customer_name").is_some());
    let cust_name = first_row["customer_name"].as_str().unwrap_or("");
    assert!(!cust_name.is_empty());
    assert_ne!(cust_name, "NULL");

    Ok(())
}

#[tokio::test]
async fn test_canvas_column_reordering_and_resizing() -> Result<(), Box<dyn std::error::Error>> {
    use databasecraft_core::canvas::{reorder_slice, CanvasColumn, CanvasSpec};

    // 1. Test reorder_slice
    let mut items = vec!["A", "B", "C", "D"];
    // Move A (0) to C (2), drop_after = false -> should place A before C: [B, A, C, D]
    reorder_slice(&mut items, 0, 2, false);
    assert_eq!(items, vec!["B", "A", "C", "D"]);

    // Move D (3) to B (0), drop_after = false -> [D, B, A, C]
    reorder_slice(&mut items, 3, 0, false);
    assert_eq!(items, vec!["D", "B", "A", "C"]);

    // Move D (0) to A (2), drop_after = true -> [B, A, D, C]
    reorder_slice(&mut items, 0, 2, true);
    assert_eq!(items, vec!["B", "A", "D", "C"]);

    // 2. Test CanvasSpec reorder_columns and set_column_width
    let mut spec = CanvasSpec::new("customers");
    spec.columns = vec![
        CanvasColumn::new("id"),
        CanvasColumn::new("company_name"),
        CanvasColumn::new("contact_name"),
        CanvasColumn::new("city"),
    ];

    // Reorder: move contact_name before id
    assert!(spec.reorder_columns("contact_name", "id", false));
    let col_names: Vec<&str> = spec.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        col_names,
        vec!["contact_name", "id", "company_name", "city"]
    );

    // Set custom column widths
    assert!(spec.set_column_width("contact_name", 180));
    assert!(spec.set_column_width("company_name", 250));
    assert_eq!(spec.columns[0].width_px, Some(180));
    assert_eq!(spec.columns[2].width_px, Some(250));

    Ok(())
}
