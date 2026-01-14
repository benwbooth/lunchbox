use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;

pub mod schema;

pub type DbPool = SqlitePool;

/// Initialize the database connection pool
pub async fn init_pool(db_path: &Path) -> Result<DbPool, sqlx::Error> {
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}

/// Connect to an existing database (read-only, for LaunchBox import)
pub async fn connect_readonly(db_path: &Path) -> Result<DbPool, sqlx::Error> {
    let db_url = format!("sqlite:{}?mode=ro", db_path.display());

    SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
}
