use axum::{extract::State, http::StatusCode, routing::get, Router};
use std::env;

async fn get_subscription_list(client: Client) -> Result<String, String> {
    Ok("".to_string())
}

async fn get_subscription_list_handler(State(client): State<Client>) -> (StatusCode, String) {
    match get_subscription_list(client).await {
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
        .with_state(client);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;

    Ok(())
}
