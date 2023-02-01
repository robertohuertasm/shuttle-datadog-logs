use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use shuttle_secrets::SecretStore;
use shuttle_service::error::CustomError;
use sqlx::{Executor, PgPool};
use std::path::PathBuf;
use sync_wrapper::SyncWrapper;
use tower_http::services::ServeDir;
use tracing::instrument;

const VERSION: &'static str = "version:0.1.0";

#[instrument]
async fn hello_world() -> &'static str {
    tracing::info!("Saying hello");
    "Hello, world!"
}

#[instrument(skip(db))]
async fn message(State(db): State<PgPool>) -> Result<String, (StatusCode, String)> {
    tracing::info!("Getting a message from the database");
    let row: (String,) = sqlx::query_as("SELECT message FROM messages LIMIT 1")
        .fetch_one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let msg = row.0;
    tracing::info!(?msg, "Got message from database");
    Ok(msg)
}

#[instrument]
async fn handle_error(error: std::io::Error) -> impl IntoResponse {
    tracing::error!(?error, "Error serving static file");
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}

#[instrument(skip(pool, _secret_store, static_folder))]
async fn axum(
    pool: PgPool,
    _secret_store: SecretStore,
    static_folder: PathBuf,
) -> shuttle_service::ShuttleAxum {
    pool.execute(include_str!("../db.sql"))
        .await
        .map_err(CustomError::new)?;

    let serve_dir = get_service(ServeDir::new(static_folder)).handle_error(handle_error);

    let router = Router::new()
        .route("/message", get(message))
        .route("/hello", get(hello_world))
        .route("/", serve_dir)
        .with_state(pool);

    let sync_wrapper = SyncWrapper::new(router);
    tracing::info!("Starting axum service");
    Ok(sync_wrapper)
}

fn get_secret(secret_store: &SecretStore, key: &str) -> Option<String> {
    let final_key = if cfg!(debug_assertions) {
        format!("{}_DEV", key)
    } else {
        key.to_string()
    };

    secret_store.get(&final_key)
}

async fn main(
    factory: &mut dyn shuttle_service::Factory,
    runtime: &shuttle_service::Runtime,
    logger: shuttle_service::Logger,
) -> Result<Box<dyn shuttle_service::Service>, shuttle_service::Error> {
    use dd_tracing_layer::{DatadogOptions, Region};
    use shuttle_service::tracing_subscriber::prelude::*;
    use shuttle_service::ResourceBuilder;

    // shuttle secret store
    let secret_store = shuttle_secrets::Secrets::new()
        .build(factory, runtime)
        .await?;

    let tags = get_secret(&secret_store, "DD_TAGS")
        .map(|tags| format!("{},{}", tags, VERSION))
        .unwrap_or(VERSION.to_string());

    let log_level = get_secret(&secret_store, "LOG_LEVEL").unwrap_or("INFO".to_string());

    // getting the datadog api key from the secret store
    let dd_api_key = secret_store
        .get("DD_API_KEY")
        .expect("DD_API_KEY not found");

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
    let filter_layer = shuttle_service::tracing_subscriber::EnvFilter::try_new(log_level)
        .expect("failed to set log level");

    // starting the tracing subscriber
    runtime
        .spawn_blocking(move || {
            shuttle_service::tracing_subscriber::registry()
                .with(filter_layer)
                .with(dd_layer)
                .with(logger)
                .init();
        })
        .await
        .map_err(|e| {
            if e.is_panic() {
                let mes = e
                    .into_panic()
                    .downcast_ref::<&str>()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "panicked setting logger".to_string());
                shuttle_service::Error::BuildPanic(mes)
            } else {
                shuttle_service::Error::Custom(
                    shuttle_service::error::CustomError::new(e).context("failed to set logger"),
                )
            }
        })?;

    // shared database
    let pool = shuttle_shared_db::Postgres::new()
        .build(factory, runtime)
        .await?;

    // shuttle static folder support
    let static_folder = shuttle_static_folder::StaticFolder::new()
        .build(factory, runtime)
        .await?;

    // starting the axum service and injecting the dependencies
    runtime
        .spawn(async {
            axum(pool, secret_store, static_folder)
                .await
                .map(|ok| Box::new(ok) as Box<dyn shuttle_service::Service>)
        })
        .await
        .map_err(|e| {
            if e.is_panic() {
                let mes = e
                    .into_panic()
                    .downcast_ref::<&str>()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "panicked calling main".to_string());
                shuttle_service::Error::BuildPanic(mes)
            } else {
                shuttle_service::Error::Custom(
                    shuttle_service::error::CustomError::new(e).context("failed to call main"),
                )
            }
        })?
}

#[no_mangle]
pub extern "C" fn _create_service() -> *mut shuttle_service::Bootstrapper {
    use shuttle_service::Context;
    let bootstrapper = shuttle_service::Bootstrapper::new(
        |factory, runtime, logger| Box::pin(main(factory, runtime, logger)),
        |srv, addr, runtime| {
            runtime.spawn(async move {
                srv.bind(addr)
                    .await
                    .context("failed to bind service")
                    .map_err(Into::into)
            })
        },
        shuttle_service::Runtime::new().unwrap(),
    );
    let boxed = Box::new(bootstrapper);
    Box::into_raw(boxed)
}
