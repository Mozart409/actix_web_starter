use actix_files::Files;
use actix_web::{App, middleware, test, web};
use sqlx::SqlitePool;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

// Import the necessary modules from the main application
use actix_web_starter::{ApiDoc, AppState, api};

const STATIC_DIR: &str = concat![env!("CARGO_MANIFEST_DIR"), "/static"];

/// Helper function to create a test database pool
async fn create_test_db_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    // Create the demo table like in the main application
    sqlx::query("CREATE TABLE demo (id INTEGER PRIMARY KEY, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();

    pool
}

/// Helper function to create a test app with all the same configuration as the main app
async fn create_test_app() -> impl actix_web::dev::Service<
    actix_web::dev::ServiceRequest,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    let db_pool = create_test_db_pool().await;

    test::init_service(
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
            .app_data(web::Data::new(AppState {
                db_pool: db_pool.clone(),
            })),
    )
    .await
}

#[actix_web::test]
async fn test_scalar_endpoint_availability() {
    let app = create_test_app().await;

    let req = test::TestRequest::get().uri("/scalar").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_success(),
        "Scalar endpoint should be available at /scalar"
    );

    // Check that the response contains HTML content (Scalar UI)
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Scalar UI should contain these elements
    assert!(
        body_str.contains("html") || body_str.contains("DOCTYPE"),
        "Scalar endpoint should return HTML content"
    );
}

#[actix_web::test]
async fn test_openapi_json_endpoint_availability() {
    let app = create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_success(),
        "OpenAPI JSON endpoint should be available at /api-docs/openapi.json"
    );

    // Check that the response is valid JSON
    let body: serde_json::Value = test::read_body_json(resp).await;

    // Verify it's a valid OpenAPI spec
    assert!(
        body.get("openapi").is_some(),
        "Response should contain 'openapi' field"
    );
    assert!(
        body.get("info").is_some(),
        "Response should contain 'info' field"
    );
    assert!(
        body.get("paths").is_some(),
        "Response should contain 'paths' field"
    );

    // Check specific info from our API
    let info = body.get("info").unwrap();
    assert_eq!(info.get("title").unwrap(), "Actix Web Starter API");
    assert_eq!(info.get("version").unwrap(), "0.1.0");
}

#[actix_web::test]
async fn test_openapi_spec_contains_health_endpoint() {
    let app = create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;

    let paths = body.get("paths").unwrap();
    assert!(
        paths.get("/api/v1/health").is_some(),
        "OpenAPI spec should contain health endpoint"
    );

    let health_path = paths.get("/api/v1/health").unwrap();
    assert!(
        health_path.get("get").is_some(),
        "Health endpoint should support GET method"
    );
}

#[actix_web::test]
async fn test_openapi_spec_contains_db_demo_endpoint() {
    let app = create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;

    let paths = body.get("paths").unwrap();
    assert!(
        paths.get("/api/v1/db-demo").is_some(),
        "OpenAPI spec should contain db-demo endpoint"
    );

    let db_demo_path = paths.get("/api/v1/db-demo").unwrap();
    assert!(
        db_demo_path.get("post").is_some(),
        "DB demo endpoint should support POST method"
    );
}

#[actix_web::test]
async fn test_openapi_spec_contains_schemas() {
    let app = create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;

    let components = body.get("components").unwrap();
    let schemas = components.get("schemas").unwrap();

    // Check that our defined schemas are present
    assert!(
        schemas.get("HealthResponse").is_some(),
        "OpenAPI spec should contain HealthResponse schema"
    );
    assert!(
        schemas.get("DbDemoResponse").is_some(),
        "OpenAPI spec should contain DbDemoResponse schema"
    );
    assert!(
        schemas.get("ErrorResponse").is_some(),
        "OpenAPI spec should contain ErrorResponse schema"
    );
}

#[actix_web::test]
async fn test_openapi_spec_contains_tags() {
    let app = create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;

    let tags = body.get("tags").unwrap().as_array().unwrap();

    // Check that our defined tags are present
    let tag_names: Vec<&str> = tags
        .iter()
        .map(|tag| tag.get("name").unwrap().as_str().unwrap())
        .collect();

    assert!(
        tag_names.contains(&"health"),
        "OpenAPI spec should contain 'health' tag"
    );
    assert!(
        tag_names.contains(&"database"),
        "OpenAPI spec should contain 'database' tag"
    );
}

#[actix_web::test]
async fn test_scalar_endpoint_with_custom_paths() {
    let app = create_test_app().await;

    // Test different paths that might be served by Scalar
    let paths = vec!["/scalar", "/scalar/"];

    for path in paths {
        let req = test::TestRequest::get().uri(path).to_request();
        let resp = test::call_service(&app, req).await;

        assert!(
            resp.status().is_success() || resp.status().is_redirection(),
            "Scalar endpoint should be accessible at path: {}",
            path
        );
    }
}

#[actix_web::test]
async fn test_integration_health_endpoint_through_openapi() {
    let app = create_test_app().await;

    // First verify the endpoint is documented in OpenAPI
    let req = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;

    let paths = body.get("paths").unwrap();
    assert!(paths.get("/api/v1/health").is_some());

    // Then test the actual endpoint
    let req = test::TestRequest::get().uri("/api/v1/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "All systems operational");
}

#[actix_web::test]
async fn test_integration_db_demo_endpoint_through_openapi() {
    let app = create_test_app().await;

    // First verify the endpoint is documented in OpenAPI
    let req = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;

    let paths = body.get("paths").unwrap();
    assert!(paths.get("/api/v1/db-demo").is_some());

    // Then test the actual endpoint
    let req = test::TestRequest::post()
        .uri("/api/v1/db-demo")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "Database demo successful");
}

#[actix_web::test]
async fn test_openapi_json_content_type() {
    let app = create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    // Check that the content type is JSON
    let content_type = resp.headers().get("content-type");
    assert!(content_type.is_some());

    let content_type_str = content_type.unwrap().to_str().unwrap();
    assert!(
        content_type_str.contains("application/json"),
        "OpenAPI endpoint should return JSON content type"
    );
}
