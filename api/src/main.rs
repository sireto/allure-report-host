use api::handlers::manifest::get_manifest;
use api::helpers::access_control::{AccessControl, access_control};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Request},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use dotenvy::dotenv;
use serde_json::json;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;
use utoipa::{
    OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::handlers::report_handler::upload_report
    ),
    components(
        schemas(
            api::models::report::ReportType,
            api::models::report::FileUploadRequest
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
    if env::var("RUST_ENV").unwrap_or_default() != "production" {
        dotenv().ok();
    }

    if env::var("API_SECRET").is_err() || env::var("API_SECRET").unwrap().is_empty() {
        panic!("CRITICAL ERROR: API_SECRET environment variable is not set or is empty.");
    }

    tracing_subscriber::fmt::init();

    let data_dir = env::var("DATA_DIR").unwrap_or_else(|_| "../data".to_string());

    let allowed_proxies: Vec<String> = std::env::var("PROXY_ALLOWED_IPS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let allowed_ips: Vec<String> = std::env::var("ALLOWED_IPS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let ac = Arc::new(AccessControl::new(allowed_ips, allowed_proxies));

    let api_routes = Router::new()
        .nest("/api", api::route::create_api_router())
        .route_layer(middleware::from_fn(auth))
        .layer(middleware::from_fn(check_content_length))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE_BYTES));

    let public_routes = Router::new().route("/manifest.json", get(get_manifest));

    let swagger_routes =
        SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());

    let static_reports = Router::new()
        .nest_service("/", ServeDir::new(&data_dir))
        .layer(middleware::from_fn_with_state(ac.clone(), access_control));

    let app = Router::new()
        .merge(swagger_routes)
        .merge(public_routes)
        .merge(api_routes)
        .fallback_service(static_reports);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on {}", addr);
    println!("Max upload size: {}MB", MAX_UPLOAD_SIZE_MB);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

/// Middleware to check Content-Length before processing the request
async fn check_content_length(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if let Some(content_length) = headers.get("content-length")
        && let Ok(length_str) = content_length.to_str()
        && let Ok(length) = length_str.parse::<u64>()
        && length > MAX_UPLOAD_SIZE_BYTES as u64
    {
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

    Ok(next.run(request).await)
}

async fn auth(headers: HeaderMap, request: Request, next: Next) -> Result<Response, StatusCode> {
    let api_key = env::var("API_SECRET").expect("API_SECRET must be set");

    match headers.get("x-api-key") {
        Some(key) if key.to_str().unwrap_or_default() == api_key => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
