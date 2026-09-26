use crate::paths;
use crate::sim::{units_from_json, Building, Unit, Units};
use crate::store::Store;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct App {
    pub store: Arc<Store>,
}

struct VoiceError(anyhow::Error);

impl IntoResponse for VoiceError {
    fn into_response(self) -> Response {
        let msg = format!("{}", self.0);
        let body = json!({ "error": msg });
        (StatusCode::BAD_REQUEST, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for VoiceError {
    fn from(e: anyhow::Error) -> Self {
        VoiceError(e)
    }
}

impl From<serde_json::Error> for VoiceError {
    fn from(e: serde_json::Error) -> Self {
        VoiceError(anyhow::Error::from(e))
    }
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

async fn auth_account(app: &App, headers: &HeaderMap) -> Result<i64, VoiceError> {
    let token = bearer(headers).ok_or_else(|| {
        VoiceError(anyhow::anyhow!("mourama: sit down — no session on this window."))
    })?;
    Ok(app.store.account_for(&token)?)
}

async fn api_status(State(app): State<App>) -> Result<Json<Value>, VoiceError> {
    let s = app.store.status()?;
    Ok(Json(serde_json::to_value(s)?))
}

async fn api_catalog() -> Json<Value> {
    Json(crate::sim::catalog())
}

#[derive(Deserialize)]
struct ClaimReq {
    invite: String,
    name: String,
    password: String,
}

async fn api_claim(
    State(app): State<App>,
    Json(body): Json<ClaimReq>,
) -> Result<Json<Value>, VoiceError> {
    let token = app.store.claim(&body.invite, &body.name, &body.password)?;
    Ok(Json(json!({ "token": token })))
}

#[derive(Deserialize)]
struct LoginReq {
    name: String,
    password: String,
}

async fn api_login(
    State(app): State<App>,
    Json(body): Json<LoginReq>,
) -> Result<Json<Value>, VoiceError> {
    let token = app.store.login(&body.name, &body.password)?;
    Ok(Json(json!({ "token": token })))
}

async fn api_me(State(app): State<App>, headers: HeaderMap) -> Result<Json<Value>, VoiceError> {
    let id = auth_account(&app, &headers).await?;
    Ok(Json(app.store.me(id)?))
}

async fn api_map(State(app): State<App>, headers: HeaderMap) -> Result<Json<Value>, VoiceError> {
    let viewer = match bearer(&headers) {
        Some(t) => app.store.account_for(&t).ok(),
        None => None,
    };
    Ok(Json(app.store.map(viewer)?))
}

#[derive(Deserialize)]
struct HillQ {
    q: i32,
    r: i32,
}

async fn api_hill(
    State(app): State<App>,
    headers: HeaderMap,
    Query(q): Query<HillQ>,
) -> Result<Json<Value>, VoiceError> {
    let id = auth_account(&app, &headers).await?;
    Ok(Json(app.store.hill_at(id, q.q, q.r)?))
}

async fn api_reports(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<Value>, VoiceError> {
    let id = auth_account(&app, &headers).await?;
    Ok(Json(app.store.reports(id)?))
}

async fn api_commands(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<Value>, VoiceError> {
    let id = auth_account(&app, &headers).await?;
    Ok(Json(app.store.commands(id)?))
}

#[derive(Deserialize)]
struct UpgradeReq {
    castro_id: i64,
    building: String,
}

async fn api_upgrade(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<UpgradeReq>,
) -> Result<Json<Value>, VoiceError> {
    let id = auth_account(&app, &headers).await?;
    let b = Building::parse(&body.building)
        .ok_or_else(|| anyhow::anyhow!("mourama: no building by that name."))?;
    app.store.upgrade(id, body.castro_id, b)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct TrainReq {
    castro_id: i64,
    unit: String,
    count: i64,
}

async fn api_train(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<TrainReq>,
) -> Result<Json<Value>, VoiceError> {
    let id = auth_account(&app, &headers).await?;
    let u = Unit::parse(&body.unit)
        .ok_or_else(|| anyhow::anyhow!("mourama: no folk by that name."))?;
    app.store.train(id, body.castro_id, u, body.count)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct SendReq {
    castro_id: i64,
    q: i32,
    r: i32,
    units: Value,
    mission: String,
}

async fn api_send(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<SendReq>,
) -> Result<Json<Value>, VoiceError> {
    let id = auth_account(&app, &headers).await?;
    let units: Units = units_from_json(&body.units);
    let arrive = app
        .store
        .send(id, body.castro_id, body.q, body.r, units, &body.mission)?;
    Ok(Json(json!({ "ok": true, "arrive": arrive })))
}

#[derive(Deserialize)]
struct EncantoReq {
    castro_id: i64,
}

async fn api_encanto(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<EncantoReq>,
) -> Result<Json<Value>, VoiceError> {
    let id = auth_account(&app, &headers).await?;
    let until = app.store.encanto(id, body.castro_id)?;
    Ok(Json(json!({ "ok": true, "encanto_until": until })))
}

pub async fn serve(addr: SocketAddr, store: Arc<Store>) -> anyhow::Result<()> {
    let web = paths::web_dir();
    if !web.join("index.html").is_file() {
        anyhow::bail!(
            "mourama: the window files are missing.\n  next:  set MOURAMA_WEB to the web/ folder"
        );
    }
    let app = App { store: store.clone() };
    let router = Router::new()
        .route("/api/status", get(api_status))
        .route("/api/catalog", get(api_catalog))
        .route("/api/claim", post(api_claim))
        .route("/api/login", post(api_login))
        .route("/api/me", get(api_me))
        .route("/api/map", get(api_map))
        .route("/api/hill", get(api_hill))
        .route("/api/reports", get(api_reports))
        .route("/api/commands", get(api_commands))
        .route("/api/upgrade", post(api_upgrade))
        .route("/api/train", post(api_train))
        .route("/api/send", post(api_send))
        .route("/api/encanto", post(api_encanto))
        .fallback_service(ServeDir::new(&web).append_index_html_on_directories(true))
        .layer(CorsLayer::permissive())
        .with_state(app);

    let store_tick = store.clone();
    tokio::spawn(async move {
        let mut iv = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            iv.tick().await;
            if let Err(e) = store_tick.tick(chrono::Local::now().timestamp()) {
                eprintln!("mourama: tick slipped: {e}");
            }
        }
    });

    let listener = TcpListener::bind(addr).await?;
    eprintln!("mourama: the wood is open at http://{addr}");
    eprintln!("  next:  mourama invite");
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            eprintln!("mourama: smooring. citânias pause until serve runs again.");
        })
        .await?;
    Ok(())
}
