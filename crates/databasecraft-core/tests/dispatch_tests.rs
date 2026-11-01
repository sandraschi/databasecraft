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
