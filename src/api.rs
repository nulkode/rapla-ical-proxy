use axum::Router;
use axum::extract::{Path, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::calendar::Calendar;
use crate::db::{Db, DeltaType, OverlayDelta};
use crate::overlay::{self, OverlayEvent};
use crate::proxy::handle as proxy_handle;
use crate::resolver::UpstreamUrlComponents;

#[derive(Clone)]
pub struct ApiState {
    pub db: Db,
    pub client: reqwest::Client,
    pub tag: Option<String>,
    pub server_url: String,
}

#[derive(Serialize)]
pub struct CalendarResponse {
    pub id: i64,
    pub name: String,
    pub rapla_url: String,
    pub forked_ics_url: String,
}

#[derive(Deserialize)]
pub struct CreateCalendarRequest {
    pub name: String,
    pub rapla_url: String,
}

#[derive(Deserialize)]
pub struct CreateDeltaRequest {
    pub r#type: String,
    pub match_key: Option<String>,
    pub event: Option<OverlayEvent>,
}

#[derive(Serialize)]
pub struct DeltaResponse {
    pub id: uuid::Uuid,
    pub calendar_id: i64,
    pub r#type: String,
    pub match_key: Option<String>,
    pub event: Option<OverlayEvent>,
}

impl From<&OverlayDelta> for DeltaResponse {
    fn from(delta: &OverlayDelta) -> Self {
        Self {
            id: delta.id,
            calendar_id: delta.calendar_id,
            r#type: match delta.r#type {
                DeltaType::Delete => "delete".into(),
                DeltaType::Modify => "modify".into(),
                DeltaType::Add => "add".into(),
            },
            match_key: delta.match_key.clone(),
            event: delta
                .event_json
                .as_ref()
                .and_then(|j| serde_json::from_str(j).ok()),
        }
    }
}

pub fn apply_api_routes(router: Router, state: ApiState, username: &str, password: &str) -> Router {
    let auth = middleware::from_fn_with_state(
        AuthState {
            credentials: format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(&format!("{}:{}", username, password))),
        },
        auth_middleware,
    );

    let public_routes = Router::new()
        .route("/calendars/{id}/forked.ics", get(get_forked_ics))
        .with_state(state.clone());

    let protected_routes = Router::new()
        .route("/calendars", get(list_calendars))
        .route("/calendars", post(create_calendar))
        .route("/calendars/{id}", delete(delete_calendar))
        .route("/calendars/{id}/deltas", get(list_deltas))
        .route("/calendars/{id}/deltas", post(create_delta))
        .route("/calendars/{id}/deltas/{delta_id}", delete(delete_delta))
        .route("/calendars/{id}/deltas/{delta_id}", put(update_delta))
        .with_state(state)
        .layer(auth);

    router.nest("/api", public_routes.merge(protected_routes))
}

async fn list_calendars(
    State(state): State<ApiState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let calendars = state
        .db
        .list_calendars()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let responses: Vec<CalendarResponse> = calendars
        .iter()
        .map(|c| CalendarResponse {
            id: c.id,
            name: c.name.clone(),
            rapla_url: c.rapla_url.clone(),
            forked_ics_url: format!("{}/api/calendars/{}/forked.ics", state.server_url, c.id),
        })
        .collect();
    Ok(axum::Json(responses))
}

async fn create_calendar(
    State(state): State<ApiState>,
    axum::Json(req): axum::Json<CreateCalendarRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let id = state
        .db
        .add_calendar(&req.name, &req.rapla_url)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok((StatusCode::CREATED, axum::Json(serde_json::json!({ "id": id }))))
}

async fn delete_calendar(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state
        .db
        .delete_calendar(id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_deltas(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let deltas = state
        .db
        .list_deltas(id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let responses: Vec<DeltaResponse> = deltas.iter().map(DeltaResponse::from).collect();
    Ok(axum::Json(responses))
}

async fn create_delta(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
    axum::Json(req): axum::Json<CreateDeltaRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let delta_type = match req.r#type.as_str() {
        "delete" => DeltaType::Delete,
        "modify" => DeltaType::Modify,
        "add" => DeltaType::Add,
        _ => return Err((StatusCode::BAD_REQUEST, "invalid type".into())),
    };

    let event_json = req
        .event
        .as_ref()
        .map(|e| serde_json::to_string(e).unwrap());

    let delta = state
        .db
        .add_delta(id, delta_type, req.match_key, event_json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, axum::Json(DeltaResponse::from(&delta))))
}

async fn delete_delta(
    State(state): State<ApiState>,
    Path((_id, delta_id)): Path<(i64, uuid::Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state
        .db
        .delete_delta(delta_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_delta(
    State(state): State<ApiState>,
    Path((id, delta_id)): Path<(i64, uuid::Uuid)>,
    axum::Json(req): axum::Json<CreateDeltaRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let delta_type = match req.r#type.as_str() {
        "delete" => DeltaType::Delete,
        "modify" => DeltaType::Modify,
        "add" => DeltaType::Add,
        _ => return Err((StatusCode::BAD_REQUEST, "invalid type".into())),
    };

    let event_json = req
        .event
        .as_ref()
        .map(|e| serde_json::to_string(e).unwrap());

    let delta = state
        .db
        .update_delta(delta_id, id, delta_type, req.match_key, event_json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::OK, axum::Json(DeltaResponse::from(&delta))))
}

async fn get_forked_ics(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Response, (StatusCode, String)> {
    let calendar = state
        .db
        .get_calendar(id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "calendar not found".into()))?;

    let components = UpstreamUrlComponents::from_simple_uri(
        &calendar
            .rapla_url
            .parse()
            .map_err(|_| (StatusCode::BAD_REQUEST, "invalid rapla_url".into()))?,
    )
    .ok_or((StatusCode::BAD_REQUEST, "could not parse rapla_url".into()))?;

    let upstream = components.generate_url();
    let rapla_calendar: Calendar = proxy_handle(&state.client, upstream)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let deltas = state
        .db
        .list_deltas(id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let merged = overlay::merge_calendar(rapla_calendar, &deltas, state.tag.as_deref());

    Ok((
        [("content-type", "text/calendar")],
        merged.to_ics().to_string(),
    )
        .into_response())
}

#[derive(Clone)]
pub struct AuthState {
    pub credentials: String,
}

async fn auth_middleware(
    State(state): State<AuthState>,
    request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(val) if val == state.credentials => next.run(request).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"Rapla Manager\"")],
            "Unauthorized",
        )
            .into_response(),
    }
}
