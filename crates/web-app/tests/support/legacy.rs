//! Provision genuine pre-card schemas without invoking today's owning writers on them.
use sqlx::{Connection, SqlitePool};
use std::path::Path;

fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Copy only the legacy target's tables/columns from a separate, validated native
/// fixture database. This preserves snapshot bytes, not historical writer behavior.
/// Migration bookkeeping and newer (notably card) tables never cross the boundary.
pub async fn copy_core(source: &Path, target: &SqlitePool) {
    let mut conn = target.acquire().await.unwrap();
    sqlx::query("ATTACH DATABASE ? AS fixture_source")
        .bind(source.to_str().unwrap())
        .execute(&mut *conn)
        .await
        .unwrap();
    let mut tx = conn.begin().await.unwrap();
    sqlx::query("PRAGMA defer_foreign_keys=ON")
        .execute(&mut *tx)
        .await
        .unwrap();
    let triggers: Vec<(String, String)> = sqlx::query_as(
        "SELECT name,sql FROM main.sqlite_schema WHERE type='trigger' ORDER BY name",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    for (name, _) in &triggers {
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "DROP TRIGGER {}",
            quoted(name)
        )))
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM main.sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations' ORDER BY name",
    ).fetch_all(&mut *tx).await.unwrap();
    assert!(!tables.iter().any(|table| table == "card_components"));
    for table in tables {
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM main.{}",
            quoted(&table)
        )))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(count, 0, "legacy target must be empty: {table}");
        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                .bind(&table)
                .fetch_all(&mut *tx)
                .await
                .unwrap();
        let columns = columns
            .iter()
            .map(|c| quoted(c))
            .collect::<Vec<_>>()
            .join(",");
        let table = quoted(&table);
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "INSERT INTO main.{table} ({columns}) SELECT {columns} FROM fixture_source.{table}"
        )))
        .execute(&mut *tx)
        .await
        .unwrap();
        // Compare every retained column in both directions, including exact JSON
        // strings, user/session data and audit rows (no reserialization).
        for (a, b) in [("main", "fixture_source"), ("fixture_source", "main")] {
            let differences = sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                "SELECT {columns} FROM {a}.{table} EXCEPT SELECT {columns} FROM {b}.{table}"
            )))
            .fetch_all(&mut *tx)
            .await
            .unwrap();
            assert!(differences.is_empty(), "fixture copy differs: {table}");
        }
    }
    for (_, sql) in &triggers {
        // SQL comes only from this test DB's original migration-defined triggers.
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    assert!(
        sqlx::query("PRAGMA main.foreign_key_check")
            .fetch_all(&mut *tx)
            .await
            .unwrap()
            .is_empty()
    );
    tx.commit().await.unwrap();
    sqlx::query("DETACH DATABASE fixture_source")
        .execute(&mut *conn)
        .await
        .unwrap();
}
