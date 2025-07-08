use color_eyre::eyre::{self, Result, WrapErr};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

pub async fn init_sqlite() -> Result<SqlitePool> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| eyre::eyre!("DATABASE_URL environment variable is not set"))?;
    if database_url.is_empty() {
        return Err(eyre::eyre!("DATABASE_URL cannot be empty"));
    }

    // Extract the file path from the sqlite:// URL if present
    let db_path = if database_url.starts_with("sqlite://") {
        database_url.strip_prefix("sqlite://").unwrap()
    } else {
        &database_url
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(db_path)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal),
        )
        .await
        .wrap_err("Failed to create SQLite connection pool")?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .wrap_err("Failed to run database migrations")?;

    Ok(pool)
}
