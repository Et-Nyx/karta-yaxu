use web_app::persistence;

#[tokio::test]
async fn readiness_is_read_only_and_rejects_untracked_auth_tables() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/test.db", dir.path().display());
    let pool = persistence::connect_for_migration(&url).await.unwrap();
    assert!(!persistence::schema_ready(&pool).await.unwrap());
    let tables: i64 = sqlx::query_scalar("SELECT count(*) FROM sqlite_master WHERE type = 'table'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(tables, 0);
    sqlx::query("CREATE TABLE users (id TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        persistence::schema_ready(&pool)
            .await
            .unwrap_err()
            .contains("without migration history")
    );
    pool.close().await;
}

#[tokio::test]
async fn readiness_checks_migration_history_and_actual_columns() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/test.db", dir.path().display());
    let pool = persistence::connect_for_migration(&url).await.unwrap();
    persistence::migrate(&pool).await.unwrap();
    persistence::migrate(&pool).await.unwrap();
    assert!(persistence::schema_ready(&pool).await.unwrap());
    let checksum: Vec<u8> =
        sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("UPDATE _sqlx_migrations SET checksum = X'00'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        persistence::schema_ready(&pool)
            .await
            .unwrap_err()
            .contains("checksum")
    );
    sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1, success = 0")
        .bind(checksum)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        persistence::schema_ready(&pool)
            .await
            .unwrap_err()
            .contains("incomplete")
    );
    sqlx::query("UPDATE _sqlx_migrations SET success = 1, version = 99")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        persistence::schema_ready(&pool)
            .await
            .unwrap_err()
            .contains("unknown")
    );
    sqlx::query("UPDATE _sqlx_migrations SET version = 1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE sessions RENAME COLUMN expires_at TO wrong_name")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        persistence::schema_ready(&pool)
            .await
            .unwrap_err()
            .contains("schema")
    );
    pool.close().await;
}

#[tokio::test]
async fn missing_migration_is_reported_without_applying_it() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/test.db", dir.path().display());
    let pool = persistence::connect_for_migration(&url).await.unwrap();
    persistence::migrate(&pool).await.unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations")
        .execute(&pool)
        .await
        .unwrap();
    assert!(!persistence::schema_ready(&pool).await.unwrap());
    assert!(
        persistence::require_schema(&pool)
            .await
            .unwrap_err()
            .contains("-- migrate")
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    pool.close().await;
}
