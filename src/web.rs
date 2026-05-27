use axum::Router;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct WebAssets;

pub fn apply_web_routes(router: Router) -> Router {
    router.route("/web", get(serve_index))
        .route("/web/", get(serve_index))
        .route("/web/{*path}", get(serve_asset))
        .route("/public/{calendar_id}", get(serve_public_page))
}

async fn serve_index() -> Response {
    serve_file("index.html")
}

async fn serve_public_page() -> Response {
    serve_file("public.html")
}

async fn serve_asset(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches("/web/");
    serve_file(path)
}

fn serve_file(path: &str) -> Response {
    match WebAssets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}
