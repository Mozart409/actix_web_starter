pub mod api;
pub mod db;

use sqlx::SqlitePool;
use utoipa::OpenApi;

pub struct AppState {
    pub db_pool: SqlitePool,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        api::health_check_handler,
        api::db_demo_handler,
    ),
    components(
        schemas(api::HealthResponse, api::DbDemoResponse, api::ErrorResponse)
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "database", description = "Database demo endpoints")
    ),
    info(
        title = "Actix Web Starter API",
        version = "0.1.0",
        description = "A starter template API built with Actix Web framework in Rust",
        contact(
            name = "API Support",
            url = "https://github.com/Mozart409/actix_web_starter"
        )
    )
)]
pub struct ApiDoc;
