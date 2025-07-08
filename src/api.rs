use actix_files::NamedFile;
use actix_web::{HttpResponse, Responder, ResponseError, Result as ActixResult, get, web};
use color_eyre::{
    Result,
    eyre::{Context, Report},
};
use sqlx::SqlitePool;
use std::fmt;

use crate::{AppState, db};

// Custom error wrapper that implements ResponseError
#[derive(Debug)]
pub struct AppError(pub Report);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        // You can customize this based on your needs
        HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Internal server error",
            "message": self.0.to_string()
        }))
    }
}

// Convert color_eyre::Report to our AppError
impl From<Report> for AppError {
    fn from(err: Report) -> Self {
        AppError(err)
    }
}

// Health check handler
async fn health_check_handler(state: web::Data<AppState>) -> ActixResult<impl Responder, AppError> {
    let status = check_system_health(&state.db_pool)
        .await
        .context("Health check failed")?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": status,
        "timestamp": chrono::Utc::now()
    })))
}

// Example function that returns color_eyre::Result
async fn check_system_health(db_pool: &SqlitePool) -> Result<String> {
    // Check if database is reachable
    sqlx::query("SELECT 1")
        .execute(db_pool)
        .await
        .context("Database connection failed")?;

    Ok("All systems operational".to_string())
}

// Database demo endpoint
async fn db_demo_handler(data: web::Data<AppState>) -> ActixResult<impl Responder, AppError> {
    // Insert a demo record
    sqlx::query("INSERT INTO demo (name) VALUES (?)")
        .bind("Demo Entry")
        .execute(&data.db_pool)
        .await
        .context("Failed to insert demo record")?;

    // Fetch all records
    let records =
        sqlx::query_as::<_, (i64, String)>("SELECT id, name FROM demo ORDER BY id DESC LIMIT 10")
            .fetch_all(&data.db_pool)
            .await
            .context("Failed to fetch demo records")?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Database demo successful",
        "records": records,
        "timestamp": chrono::Utc::now()
    })))
}

// Favicon endpoint
#[get("/favicon.ico")]
async fn favicon_handler() -> ActixResult<impl Responder, AppError> {
    let file = NamedFile::open_async("static/logo.png")
        .await
        .context("Failed to open favicon file")?;
    Ok(file)
}
// Configure all routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/health", web::get().to(health_check_handler))
            .route("/db-demo", web::post().to(db_demo_handler)),
    );
}
