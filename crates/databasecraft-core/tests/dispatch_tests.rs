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
    assert_eq!(registry.len(), 15);

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

    let export_desc = databasecraft_core::describe_command("db.export_merge").ok_or("missing db.export_merge")?;
    assert_eq!(export_desc.id, "db.export_merge");

    let hybrid_desc = databasecraft_core::describe_command("rag.hybrid_search").ok_or("missing rag.hybrid_search")?;
    assert_eq!(hybrid_desc.id, "rag.hybrid_search");

    let unknown = databasecraft_core::describe_command("nonexistent_command");
    assert!(unknown.is_none());

    Ok(())
}
