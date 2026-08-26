use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init(app_name: &str) {
    let default = format!("info,{app_name}=debug,hilen_server=debug,tower_http=debug");
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default)))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
