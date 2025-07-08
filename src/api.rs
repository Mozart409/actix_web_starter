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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test, web};
    use sqlx::SqlitePool;
    use std::path::Path;
    use tempfile::tempdir;

    async fn create_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // Create demo table
        sqlx::query("CREATE TABLE demo (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        pool
    }

    async fn create_test_app_state() -> AppState {
        let db_pool = create_test_db().await;
        AppState { db_pool }
    }

    #[actix_web::test]
    async fn test_health_check_success() {
        let app_state = create_test_app_state().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .route("/health", web::get().to(health_check_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "All systems operational");
        assert!(body["timestamp"].is_string());
    }

    #[actix_web::test]
    async fn test_db_demo_success() {
        let app_state = create_test_app_state().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .route("/db-demo", web::post().to(db_demo_handler)),
        )
        .await;

        let req = test::TestRequest::post().uri("/db-demo").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Database demo successful");
        assert!(body["records"].is_array());
        assert!(body["timestamp"].is_string());

        // Check that a record was actually inserted
        let records = body["records"].as_array().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0][1], "Demo Entry");
    }

    #[actix_web::test]
    async fn test_db_demo_multiple_inserts() {
        let app_state = create_test_app_state().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .route("/db-demo", web::post().to(db_demo_handler)),
        )
        .await;

        // Make multiple requests
        for _ in 0..3 {
            let req = test::TestRequest::post().uri("/db-demo").to_request();
            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());
        }

        // Check final state
        let req = test::TestRequest::post().uri("/db-demo").to_request();
        let resp = test::call_service(&app, req).await;
        let body: serde_json::Value = test::read_body_json(resp).await;
        let records = body["records"].as_array().unwrap();
        assert_eq!(records.len(), 4); // 3 previous + 1 current
    }

    #[actix_web::test]
    async fn test_favicon_handler() {
        // Create a temporary favicon file for testing
        let temp_dir = tempdir().unwrap();
        let static_dir = temp_dir.path().join("static");
        std::fs::create_dir_all(&static_dir).unwrap();
        let favicon_path = static_dir.join("logo.png");
        std::fs::write(&favicon_path, b"fake_png_data").unwrap();

        // Change to temp directory for test
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let app = test::init_service(App::new().service(favicon_handler)).await;

        let req = test::TestRequest::get().uri("/favicon.ico").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    async fn test_app_error_display() {
        let report = color_eyre::eyre::eyre!("Test error message");
        let app_error = AppError(report);

        assert_eq!(format!("{}", app_error), "Test error message");
    }

    #[test]
    async fn test_app_error_from_report() {
        let report = color_eyre::eyre::eyre!("Test error");
        let app_error: AppError = report.into();

        assert_eq!(format!("{}", app_error), "Test error");
    }
}
