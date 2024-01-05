use std::env;

fn get_subscription_list() {

}

#[tokio::main]
async fn main() {

    let tenant_id = env::var("TENANT_ID").expect("env variable TENANT_ID");
    let client_id = env::var("CLIENT_ID").expect("env variable CLIENT_ID");
    let client_secret = env::var("CLIENT_SECRET").expect("env variable CLIENT_SECRET");
    let scope = env::var("SCOPE").expect("env variable SCOPE");
    let proxy = env::var("http_proxy").unwrap_or("http://proxy-http:8080".to_string());

    println!("Utilisation du proxy {proxy}");

    println!("tenant_id={tenant_id} client_id={client_id} client_secret={client_secret} scope={scope}");
}
