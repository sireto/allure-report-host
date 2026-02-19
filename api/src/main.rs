use axum::{
    Json, Router, extract::{DefaultBodyLimit, Request}, http::{HeaderMap, StatusCode}, middleware::{self, Next}, response::Response, routing::get
};
use dotenvy::dotenv;
use serde_json::json;
use std::env;
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        root,
        api::handlers::report_handler::upload_report
    ),
    components(
        schemas(
            api::models::report::CreateReportRequest,
            api::models::report::ReportResponse,
            api::models::report::ReportType
        )
    ),
    modifiers(&SecurityAddon),
    info(title = "Allure Report Host API", version = "0.1.0")
)]
struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-api-key"))),
            )
        }
    }
}

const MAX_UPLOAD_SIZE_BYTES: usize = 500 * 1024 * 1024; // 500MB
const MAX_UPLOAD_SIZE_MB: usize = MAX_UPLOAD_SIZE_BYTES / (1024 * 1024);

#[tokio::main]
async fn main() {
    dotenv().ok();

    if env::var("API_KEY").is_err() || env::var("API_KEY").unwrap().is_empty() {
        panic!("CRITICAL ERROR: API_KEY environment variable is not set or is empty.");
    }

    tracing_subscriber::fmt::init();

    let data_dir = env::var("DATA_DIR").unwrap_or_else(|_| "../data".to_string());

    let api_routes = Router::new()
        .nest("/api", api::route::create_api_router())
        .route_layer(middleware::from_fn(auth))
        .layer(middleware::from_fn(check_content_length))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_BYTES));

    let public_routes = Router::new()
        .route("/", get(root));

    let swagger_routes = SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi());

    // Serve static reports without authentication
    let static_reports = Router::new()
        .nest_service("/", ServeDir::new(&data_dir));

    let app = Router::new()
        .merge(swagger_routes)
        .merge(public_routes)
        .merge(api_routes)
        .fallback_service(static_reports);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8088));
    println!("Listening on {}", addr);
    println!("Max upload size: {}MB", MAX_UPLOAD_SIZE_MB);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Middleware to check Content-Length before processing the request
async fn check_content_length(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if let Some(content_length) = headers.get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<u64>() {
                if length > MAX_UPLOAD_SIZE_BYTES as u64 {
                    let size_mb = length / (1024 * 1024);
                    let error_response = json!({
                        "error": format!(
                            "File size exceeds maximum limit of {}MB (received: {}MB)",
                            MAX_UPLOAD_SIZE_MB, size_mb
                        ),
                        "max_size_bytes": MAX_UPLOAD_SIZE_BYTES,
                        "max_size_mb": MAX_UPLOAD_SIZE_MB,
                        "received_bytes": length,
                        "received_mb": size_mb
                    });
                    return Err((StatusCode::PAYLOAD_TOO_LARGE, Json(error_response)));
                }
            }
        }
    }

    Ok(next.run(request).await)
}

async fn auth(headers: HeaderMap, request: Request, next: Next) -> Result<Response, StatusCode> {
    let api_key = env::var("API_KEY").expect("API_KEY must be set");

    match headers.get("x-api-key") {
        Some(key) if key.to_str().unwrap_or_default() == api_key => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Welcome message", body = String),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("api_key" = [])
    )
)]
async fn root() -> &'static str {
    "Hello, Devs! Welcome to the Allure Report Host API."
}