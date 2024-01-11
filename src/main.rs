use axum::{extract::State, http::StatusCode, routing::get, Router};
use std::env;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    tenant_id: Arc<String>,
    client_id: Arc<String>,
    client_secret: Arc<String>,
    http_client: reqwest::Client,
}

async fn get_subscription_list(state: AppState) -> Result<String, String> {
    Ok("".to_string())
}

async fn get_subscription_list_handler(State(state): State<AppState>) -> (StatusCode, String) {
    match get_subscription_list(state).await {
        Ok(res) => (StatusCode::OK, res),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

async fn root_handler() -> String {
    "I'm Alive :D".to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tenant_id = env::var("TENANT_ID").expect("env variable TENANT_ID");
    let client_id = env::var("CLIENT_ID").expect("env variable CLIENT_ID");
    let client_secret = env::var("CLIENT_SECRET").expect("env variable CLIENT_SECRET");
    let scope = env::var("SCOPE").expect("env variable SCOPE");
    let proxy = env::var("http_proxy").unwrap_or("http://proxy-http:8080".to_string());

    println!("Utilisation du proxy {proxy}");

    println!(
        "tenant_id={tenant_id} client_id={client_id} client_secret={client_secret} scope={scope}"
    );

    // Create a reqwest client
    let client = reqwest::Client::new();

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/metrics", get(get_subscription_list_handler))
        .with_state(AppState {
            tenant_id: Arc::new(tenant_id),
            client_id: Arc::new(client_id),
            client_secret: Arc::new(client_secret),
            http_client: reqwest::Client::new(),
        });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;

    Ok(())
}
