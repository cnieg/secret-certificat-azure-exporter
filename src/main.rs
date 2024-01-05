use std::env;

fn get_subscription_list() {

}

#[tokio::main]
async fn main() {

    let tenant_id = env::var("TENANT_ID").expect("env variable TENANT_ID is not set. exiting.");
    let client_id = env::var("CLIENT_ID").expect("env variable CLIENT_ID is not set. exiting.");
    let client_secret = env::var("CLIENT_SECRET").expect("env variable CLIENT_SECRET is not set. exiting.");
    let scope = env::var("SCOPE").expect("env variable SCOPE is not set. exiting.");
    let proxy = env::var("http_proxy").unwrap_or("http://proxy-http:8080".to_string());

    println!("Utilisation du proxy {proxy}");

    println!("tenant_id={tenant_id} client_id={client_id} client_secret={client_secret} scope={scope}");
}
