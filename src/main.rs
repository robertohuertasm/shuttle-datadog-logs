use axum::{routing::get, Router};
use dd_tracing_layer::{DatadogOptions, Region};
use shuttle_secrets::SecretStore;
use tracing::instrument;
use tracing_subscriber::prelude::*;

const VERSION: &'static str = "version:0.1.0";

#[instrument]
async fn hello_world() -> &'static str {
    tracing::info!("Saying hello");
    tracing::debug!("Saying hello for debug level only");
    "Hello, world!"
}

#[shuttle_runtime::main]
async fn axum(#[shuttle_secrets::Secrets] secret_store: SecretStore) -> shuttle_axum::ShuttleAxum {
    // getting the Datadog Key from the secrets
    let dd_api_key = secret_store
        .get("DD_API_KEY")
        .expect("DD_API_KEY not found");

    // getting the Datadog tags from the secrets
    let tags = secret_store
        .get("DD_TAGS")
        .map(|tags| format!("{},{}", tags, VERSION))
        .unwrap_or(VERSION.to_string());

    // getting the log level from the secrets
    let log_level = secret_store.get("LOG_LEVEL").unwrap_or("INFO".to_string());

    // datadog tracing layer
    let dd_layer = dd_tracing_layer::create(
        DatadogOptions::new(
            // first parameter is the name of the service
            "shuttle-datadog-logs",
            // this is the Datadog API Key
            dd_api_key,
        )
        // this is the default, so it can be omitted
        .with_region(Region::US1)
        // adding some optional tags
        .with_tags(tags),
    );

    // filter layer
    let filter_layer =
        tracing_subscriber::EnvFilter::try_new(log_level).expect("failed to set log level");

    // format layer
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .json()
        .flatten_event(true)
        .with_target(true)
        .with_span_list(true);

    // starting the tracing subscriber
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(dd_layer)
        .init();

    // starting the server
    let router = Router::new().route("/", get(hello_world));
    tracing::info!("Starting axum service");
    Ok(router.into())
}
