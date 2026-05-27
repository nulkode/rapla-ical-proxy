mod api;
mod cache;
mod calendar;
mod db;
mod logging;
mod overlay;
mod parser;
mod proxy;
mod resolver;
mod web;

use std::env::{self, VarError};
use std::fmt::Display;
use std::net::SocketAddr;
use std::str::FromStr;

use axum::Router;
use tokio::net::TcpListener;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    #[cfg(debug_assertions)]
    if let Some(uri) = getenv("RAPLA_DEBUG") {
        use crate::proxy::{build_client, handle};
        use crate::resolver::UpstreamUrlComponents;

        let calendar = handle(
            &build_client(),
            UpstreamUrlComponents::from_request_uri(&uri)
                .expect("couldn't resolve upstream")
                .generate_url(),
        )
        .await
        .expect("couldn't handle request");

        eprintln!("{calendar:#?}");

        return Ok(());
    }

    let address =
        getenv("RAPLA_ADDRESS").unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8080)));

    let cache_ttl = Duration::from_secs(getenv("RAPLA_CACHE_TTL").unwrap_or(3600));
    let cache_capacity = getenv("RAPLA_CACHE_MAX_SIZE").unwrap_or(0);

    let db_path = getenv::<String>("RAPLA_DB_PATH").unwrap_or_else(|| "rapla.db".into());
    let db = db::Db::open(&db_path).expect("failed to open database");

    let tag = getenv::<String>("RAPLA_TAG");

    let server_url = getenv::<String>("RAPLA_SERVER_URL").unwrap_or_else(|| {
        format!("http://{}", address)
    });

    let (web_username, web_password) = match (
        getenv::<String>("RAPLA_WEB_USERNAME"),
        getenv::<String>("RAPLA_WEB_PASSWORD"),
    ) {
        (Some(u), Some(p)) => (u, p),
        _ => {
            eprintln!("RAPLA_WEB_USERNAME and RAPLA_WEB_PASSWORD must be set to enable the web interface");
            return Ok(());
        }
    };

    let client = proxy::build_client();
    let api_state = api::ApiState {
        db,
        client,
        tag,
        server_url,
    };

    let router = Router::new();
    let router = crate::proxy::apply_routes(router);
    let router = crate::cache::apply_middleware(router, (cache_ttl, cache_capacity));
    let router = crate::resolver::apply_middleware(router);
    let router = crate::logging::apply_middleware(router);

    let router = crate::api::apply_api_routes(router, api_state, &web_username, &web_password);

    let router = crate::web::apply_web_routes(router);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

fn getenv<T: FromStr>(key: &str) -> Option<T>
where
    T::Err: Display,
{
    use std::process;

    let val = match env::var(key) {
        Ok(val) => val,
        Err(VarError::NotPresent) => return None,
        Err(err) => {
            eprintln!("Invalid ${key}: {err}");
            process::exit(1);
        }
    };

    Some(T::from_str(&val).unwrap_or_else(|err| {
        eprintln!("Invalid ${key}: {err}");
        process::exit(1);
    }))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
