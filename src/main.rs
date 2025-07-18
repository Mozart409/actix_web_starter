use actix_files::Files;
use actix_web::{
    App, HttpServer,
    middleware::{self, Logger},
    web,
};
use color_eyre::{Result, eyre::Context};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use actix_web_starter::{ApiDoc, AppState, api, db};

const STATIC_DIR: &str = concat![env!("CARGO_MANIFEST_DIR"), "/static"];

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    color_eyre::install()?;

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Initialize the database connection pool
    let db_pool = db::init_sqlite()
        .await
        .wrap_err("Failed to initialize SQLite database connection pool")?;

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .service(
                Files::new("/static", STATIC_DIR)
                    .show_files_listing()
                    .prefer_utf8(true),
            )
            .service(api::favicon_handler)
            .service(Scalar::with_url("/scalar", ApiDoc::openapi()))
            .service(
                web::resource("/api-docs/openapi.json")
                    .route(web::get().to(|| async { web::Json(ApiDoc::openapi()) })),
            )
            .configure(api::configure_routes)
            .wrap(middleware::Compress::default())
            .wrap(middleware::NormalizePath::trim())
            .wrap(Logger::default())
            .app_data(web::Data::new(AppState {
                db_pool: db_pool.clone(),
            }))
    })
    .bind(("0.0.0.0", 8080))
    .wrap_err("Failed to bind server to address 0.0.0.0:8080")?
    .run()
    .await
    .wrap_err("Server failed to run")
}
