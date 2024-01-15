use axum::{extract::State, http::StatusCode, routing::get, Router};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::env;

#[derive(Clone)]
struct AppState {
    tenant_id: String,
    client_id: String,
    client_secret: String,
    scope: String,
    http_client: reqwest::Client,
}

#[derive(Deserialize, Debug)]
struct Token {
    access_token: String,
    // TODO: get and handle the "expires_in" field
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Credential {
    end_date_time: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Application {
    app_id: String,
    display_name: String,
    password_credentials: Vec<Credential>,
    key_credentials: Vec<Credential>,
}

#[derive(Deserialize, Debug)]
struct Applications {
    value: Vec<Application>,
}

async fn get_token(state: &AppState) -> Result<Token, reqwest::Error> {
    let params = [
        ("client_id", state.client_id.to_string()),
        ("scope", state.scope.to_string()),
        ("client_secret", state.client_secret.to_string()),
        ("grant_type", "client_credentials".to_string()),
    ];
    let res: Token = state
        .http_client
        .post(format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            state.tenant_id
        ))
        .form(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(res)
}

fn parse_credentials(application: &Application, credentials: &Vec<Credential>, res: &mut String) {

    let mut jours_restants: i64;
    let date_now = Utc::now();

    for (id, credential) in credentials.iter().enumerate() {
        if date_now > credential.end_date_time {
            println!(
                "Le secret n°{id} de l'application {} a expiré le {}",
                application.display_name,
                credential.end_date_time.format("%d/%m/%Y")
            );
            jours_restants = 0;
        } else {
            jours_restants = (credential.end_date_time - date_now).num_days();
        }
        res.push_str(&format!(
            "# HELP application_{}_{id} Secret N°{id} pour l'application {}\n",
            application.app_id, application.display_name
        ));
        res.push_str(&format!(
            "# TYPE application_{}_{id} gauge\n",
            application.app_id
        ));
        res.push_str(&format!(
            "application_{}_{id}{{application=\"{}\",type=\"secret\",app=\"Azure Certificat Expiration\"}} {jours_restants}\n\n",
            application.app_id,
            application.display_name));
    }
}

async fn get_subscription_list(state: AppState) -> Result<String, reqwest::Error> {
    let token = get_token(&state).await?;

    let applications: Applications = state
        .http_client
        .get("https://graph.microsoft.com/v1.0/applications")
        .bearer_auth(&token.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut res = String::new(); // This is what we are going to return

    for mut application in applications.value {
        // On nettoie les noms pour remove les caractères spéciaux + espaces (pour la conversion en métriques)
        application.app_id = application.app_id.replace('-', "_");
        application.display_name = application
            .display_name
            .replace('-', "_")
            .replace([' ', '(', ')'], "")
            .replace(['é', 'ê', 'è', 'ë'], "e");

        // Handle secrets
        parse_credentials(&application, &application.password_credentials, &mut res);

        // Handle certificates
        parse_credentials(&application, &application.key_credentials, &mut res);
    }

    Ok(res)
}

async fn get_subscription_list_handler(State(state): State<AppState>) -> (StatusCode, String) {
    match get_subscription_list(state).await {
        Ok(res) => (StatusCode::OK, res),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
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
    let proxy_str = env::var("http_proxy").unwrap_or_else(|_| "http://proxy-http:8080".to_string());

    // Create an HTTP client with or without a proxy depending on the value of env::var("http_proxy")
    let http_client = match proxy_str.as_str() {
        "" => reqwest::Client::new(), // no proxy
        _ => reqwest::ClientBuilder::new()
            .proxy(reqwest::Proxy::https(&proxy_str)?)
            .build()?,
    };

    println!("Utilisation du proxy '{proxy_str}'");

    println!(
        "tenant_id={tenant_id} client_id={client_id} client_secret={client_secret} scope={scope}"
    );

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/metrics", get(get_subscription_list_handler))
        .with_state(AppState {
            tenant_id,
            client_id,
            client_secret,
            scope,
            http_client,
        });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
