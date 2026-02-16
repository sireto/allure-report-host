use axum::{
    Router, extract::{DefaultBodyLimit, Request}, http::{HeaderMap, StatusCode}, middleware::{self, Next}, response::Response, routing::get
};
use dotenvy::dotenv;
use std::env;
use std::net::SocketAddr;
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

#[tokio::main]
async fn main() {
    dotenv().ok();

    if env::var("API_KEY").is_err() || env::var("API_KEY").unwrap().is_empty() {
        panic!("CRITICAL ERROR: API_KEY environment variable is not set or is empty.");
    }

    tracing_subscriber::fmt::init();

    let api_routes = Router::new()
        .nest("/api", api::route::create_api_router()) 
        .route_layer(middleware::from_fn(auth))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024));

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/", get(root))
        .merge(api_routes);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8088));
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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