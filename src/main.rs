use axum::{extract::State, http::StatusCode, routing::get, Router};
use chrono::{DateTime, Utc};
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use std::{env, future::IntoFuture, time::Duration};
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::{mpsc, oneshot},
    time,
};

const MICROSOFT_DATA_REFRESH_HOURS_DEFAULT: u8 = 6;

#[derive(Debug)]
enum ActorMessage {
    GetResponse { respond_to: oneshot::Sender<String> },
}

#[derive(Clone)]
struct AppState {
    sender: mpsc::Sender<ActorMessage>,
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

async fn get_token(
    http_client: &Client,
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    scope: &str,
) -> Result<Token, reqwest::Error> {
    let params = [
        ("client_id", client_id),
        ("scope", scope),
        ("client_secret", client_secret),
        ("grant_type", "client_credentials"),
    ];
    let res: Token = http_client
        .post(format!(
            "https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token",
        ))
        .form(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(res)
}

fn parse_credentials(application: &Application, credentials: &[Credential]) -> String {
    let mut res = String::new(); // This is what we are going to return
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

    res
}

async fn get_subscription_list(
    http_client: &Client,
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    scope: &str,
) -> Result<String, reqwest::Error> {
    let token = get_token(http_client, tenant_id, client_id, client_secret, scope).await?;

    let applications: Applications = http_client
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
        res.push_str(&parse_credentials(
            &application,
            &application.password_credentials,
        ));

        // Handle certificates
        res.push_str(&parse_credentials(
            &application,
            &application.key_credentials,
        ));
    }

    Ok(res)
}

#[allow(clippy::redundant_pub_crate)] // Because clippy is not happy with the tokio::select macro
async fn secrets_actor(mut receiver: mpsc::Receiver<ActorMessage>) {
    let mut response = String::new(); // The is the state this actor is handling

    dotenv().ok();

    let tenant_id = env::var("TENANT_ID").expect("env variable TENANT_ID");
    let client_id = env::var("CLIENT_ID").expect("env variable CLIENT_ID");
    let client_secret = env::var("CLIENT_SECRET").expect("env variable CLIENT_SECRET");
    let scope = env::var("SCOPE").expect("env variable SCOPE");
    let proxy_string = env::var("http_proxy");
    let proxy_str = proxy_string.as_deref().unwrap_or("http://proxy-http:8080");
    let microsoft_data_refresh_hours = env::var("MICROSOFT_DATA_REFRESH_HOURS").map_or(
        MICROSOFT_DATA_REFRESH_HOURS_DEFAULT,
        |value| {
            value
                .parse()
                .map_or(MICROSOFT_DATA_REFRESH_HOURS_DEFAULT, |value| {
                    if value > 0 && value <= 24 {
                        value
                    } else {
                        MICROSOFT_DATA_REFRESH_HOURS_DEFAULT
                    }
                })
        },
    );

    // Create an HTTP client with or without a proxy depending on the value of env::var("http_proxy")
    let http_client = match proxy_str {
        "" => reqwest::Client::new(), // no proxy
        _ => reqwest::ClientBuilder::new()
            .proxy(reqwest::Proxy::https(proxy_str).expect("Failed to add proxy"))
            .build()
            .expect("Failed to build reqwest client"),
    };

    println!("Utilisation du proxy '{proxy_str}'");

    // State (response) initialization is done below (the first call to timer.tick() returns immediately)

    let mut timer = time::interval(Duration::from_secs(
        u64::from(microsoft_data_refresh_hours) * 3600,
    ));

    // We now wait for some messages (or for the timer to tick)
    loop {
        tokio::select! {
            msg = receiver.recv() => match msg {
                Some(msg) => match msg {
                    ActorMessage::GetResponse { respond_to } => respond_to.send(response.clone()).unwrap_or_else(|_| println!("Failed to send reponse : oneshot channel was closed"))
                },
                None => break
            },
            _ = timer.tick() => {
                // State (response) update
                match get_subscription_list(&http_client, &tenant_id, &client_id, &client_secret, &scope).await
                {
                    Ok(res) => response = res,
                    Err(e) => {
                        println!("get_subscription_list() failed with : {e}");
                        response.clear();
                    },
                }
            }
        }
    }
}

async fn root_handler() -> &'static str {
    "I'm Alive :D"
}

async fn get_subscription_list_handler(State(state): State<AppState>) -> (StatusCode, String) {
    // We are going to send a message to our actor and wait for an answer
    // But first, we create a oneshot channel to get the actor's response
    let (send, recv) = oneshot::channel();
    let msg = ActorMessage::GetResponse { respond_to: send };

    // Ignore send errors. If this send fails, so does the
    // recv.await below. There's no reason to check for the
    // same failure twice.
    let _ = state.sender.send(msg).await;

    match recv.await {
        Ok(res) => (StatusCode::OK, res),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // An infinite stream of 'SIGTERM' signals.
    let mut sigterm_stream = signal(SignalKind::terminate())?;

    // Create a channel and then an actor
    let (sender, receiver) = mpsc::channel(8);
    let actor_handle = tokio::spawn(secrets_actor(receiver));

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/metrics", get(get_subscription_list_handler))
        .with_state(AppState { sender });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    println!("listening on {}", listener.local_addr()?);

    // Waiting for one of the following :
    // - a SIGTERM signal
    // - the actor to finish/panic
    // - the axum server to finish
    tokio::select! {
        _ = sigterm_stream.recv() => {
            println!("Received a SIGTERM signal! exiting.");
            Ok(())
        },
        _ = actor_handle => {
            println!("The actor died! exiting.");
            Ok(())
        },
        _ = axum::serve(listener, app).into_future() => {
            println!("The server died! exiting.");
            Ok(())
        }
    }
}
