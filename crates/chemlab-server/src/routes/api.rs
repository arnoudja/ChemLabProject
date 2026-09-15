use crate::auth::{
    clear_session_cookie, csrf_cookie, csrf_token_from_jar, generate_session_token, hash_password,
    hash_token, require_csrf, session_cookie, token_from_jar, validate_credentials,
    verify_password,
};
use crate::error::ApiError;
use crate::rate_limit::{check_auth_attempt, ClientIp};
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::CookieJar;
use chemlab_contracts::{
    AuthUserResponse, CsrfResponse, DissolveRequest, DissolveResponse, HealthResponse, LabAction,
    LabActionResponse, LabEvent, LabScene, LoginRequest, MeResponse, RegisterRequest,
};
use chemlab_db::{
    create_session, delete_session_by_token_hash, delete_sessions_for_user, find_user_by_email,
    find_valid_session_by_token_hash, get_or_create_lab_for_user, insert_user, save_lab_state,
    LabRecord, UserRecord,
};
use chrono::Duration;
use chrono::Utc;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/csrf", get(csrf))
        .route("/api/auth/me", get(me))
        .route("/api/lab/scene", get(lab_scene))
        .merge(csrf_protected())
}

/// Mutating JSON routes (auth + lab). Reuses `require_csrf`; do not fork a second CSRF helper.
fn csrf_protected() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/lab/dissolve", post(lab_dissolve))
        .route("/api/lab/action", post(lab_action))
        .layer(middleware::from_fn(require_csrf))
}

async fn csrf(State(state): State<AppState>, jar: CookieJar) -> (CookieJar, Json<CsrfResponse>) {
    if let Some(token) = csrf_token_from_jar(&jar) {
        return (jar, Json(CsrfResponse { csrf_token: token }));
    }
    let token = generate_session_token();
    let ttl = Duration::hours(state.inner.session_ttl_hours);
    let jar = jar.add(csrf_cookie(&token, state.inner.cookie_secure, ttl));
    (jar, Json(CsrfResponse { csrf_token: token }))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok(env!("CARGO_PKG_VERSION")))
}

async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    ClientIp(ip): ClientIp,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, CookieJar, Json<AuthUserResponse>), ApiError> {
    check_auth_attempt(&state.inner.auth_rate_limiter, "register", &ip, &body.email)?;
    validate_credentials(&body.email, &body.password, Some(&body.display_name))
        .map_err(|m| ApiError::bad_request("validation", m))?;

    let password_hash =
        hash_password(&body.password).map_err(|e| ApiError::internal(format!("hash: {e}")))?;

    let user = insert_user(
        state.pool(),
        body.email.trim(),
        body.display_name.trim(),
        &password_hash,
    )
    .await?;

    let (jar, response) = issue_session(&state, jar, &user).await?;
    Ok((StatusCode::CREATED, jar, Json(response)))
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    ClientIp(ip): ClientIp,
    Json(body): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthUserResponse>), ApiError> {
    check_auth_attempt(&state.inner.auth_rate_limiter, "login", &ip, &body.email)?;
    validate_credentials(&body.email, &body.password, None)
        .map_err(|m| ApiError::bad_request("validation", m))?;

    let user = find_user_by_email(state.pool(), body.email.trim()).await?;
    let ok = verify_password(&body.password, &user.password_hash)
        .map_err(|e| ApiError::internal(format!("verify: {e}")))?;
    if !ok {
        return Err(ApiError::unauthorized(
            "invalid_credentials",
            "Invalid email or password",
        ));
    }

    delete_sessions_for_user(state.pool(), &user.id).await?;
    let (jar, response) = issue_session(&state, jar, &user).await?;
    Ok((jar, Json(response)))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(StatusCode, CookieJar), ApiError> {
    if let Some(token) = token_from_jar(&jar) {
        let token_hash = hash_token(&token);
        let _ = delete_session_by_token_hash(state.pool(), &token_hash).await;
    }
    let jar = jar.add(clear_session_cookie(state.inner.cookie_secure));
    Ok((StatusCode::NO_CONTENT, jar))
}

async fn lab_dissolve(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<DissolveRequest>,
) -> Result<Json<DissolveResponse>, ApiError> {
    if current_user(&state, &jar).await?.is_none() {
        return Err(ApiError::unauthorized("unauthenticated", "Login required"));
    }

    let outcome = chemlab_core::dissolve(&body.substance_id, &body.solvent_id, body.temperature_c)?;
    Ok(Json(DissolveResponse {
        dissolved: outcome.dissolved,
        explanation: outcome.explanation.to_string(),
    }))
}

async fn lab_scene(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<LabScene>, ApiError> {
    let user = require_current_user(&state, &jar).await?;
    let mut scene = load_or_initialize_scene(&state, &user.id).await?;
    apply_elapsed_clock(&mut scene, unix_now_ms());
    Ok(Json(persist_scene(&state, scene).await?))
}

async fn lab_action(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(action): Json<LabAction>,
) -> Result<Json<LabActionResponse>, ApiError> {
    let user = require_current_user(&state, &jar).await?;
    let mut scene = load_or_initialize_scene(&state, &user.id).await?;
    apply_elapsed_clock(&mut scene, unix_now_ms());
    chemlab_core::apply_action(&mut scene, action_to_core(action))?;
    scene.version = scene
        .version
        .checked_add(1)
        .ok_or_else(|| ApiError::internal("Lab version overflow"))?;

    Ok(Json(LabActionResponse {
        scene: persist_scene(&state, scene).await?,
    }))
}

async fn me(State(state): State<AppState>, jar: CookieJar) -> Result<Json<MeResponse>, ApiError> {
    match current_user(&state, &jar).await? {
        Some(user) => Ok(Json(MeResponse {
            authenticated: true,
            user: Some(to_auth_user(&user)),
        })),
        None => Ok(Json(MeResponse {
            authenticated: false,
            user: None,
        })),
    }
}

async fn issue_session(
    state: &AppState,
    jar: CookieJar,
    user: &UserRecord,
) -> Result<(CookieJar, AuthUserResponse), ApiError> {
    let token = generate_session_token();
    let token_hash = hash_token(&token);
    let ttl = Duration::hours(state.inner.session_ttl_hours);
    create_session(state.pool(), &user.id, &token_hash, ttl).await?;
    let jar = jar.add(session_cookie(&token, state.inner.cookie_secure, ttl));
    Ok((jar, to_auth_user(user)))
}

async fn current_user(state: &AppState, jar: &CookieJar) -> Result<Option<UserRecord>, ApiError> {
    let Some(token) = token_from_jar(jar) else {
        return Ok(None);
    };
    let token_hash = hash_token(&token);
    match find_valid_session_by_token_hash(state.pool(), &token_hash).await {
        Ok((_session, user)) => Ok(Some(user)),
        Err(chemlab_db::DbError::SessionInvalid) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

async fn require_current_user(state: &AppState, jar: &CookieJar) -> Result<UserRecord, ApiError> {
    current_user(state, jar)
        .await?
        .ok_or_else(|| ApiError::unauthorized("unauthenticated", "Login required"))
}

async fn load_or_initialize_scene(
    state: &AppState,
    user_id: &str,
) -> Result<chemlab_core::Scene, ApiError> {
    let lab = get_or_create_lab_for_user(state.pool(), user_id).await?;
    if let Some(blob) = lab.state_blob.as_deref().filter(|blob| !blob.is_empty()) {
        let mut scene = contract_to_scene(
            serde_json::from_slice(blob)
                .map_err(|error| ApiError::internal(format!("deserialize lab scene: {error}")))?,
        );
        scene.version = lab_version(&lab)?;
        scene.lab_id = lab.id;
        chemlab_core::ensure_default_bench_items(&mut scene);
        return Ok(scene);
    }

    let scene = chemlab_core::initial_bench_scene(&lab.id);
    persist_scene(state, scene.clone()).await?;
    Ok(scene)
}

/// Clamp for server-authoritative ticks on each GET/POST (spec: e.g. 0–2 s).
const MAX_ELAPSED_MS: i64 = 2000;

fn unix_now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn apply_elapsed_clock(scene: &mut chemlab_core::Scene, now_ms: i64) {
    let last = scene.last_applied_unix_ms.unwrap_or(now_ms);
    let dt_ms = now_ms.saturating_sub(last).clamp(0, MAX_ELAPSED_MS);
    chemlab_core::apply_elapsed(scene, dt_ms as f64 / 1000.0);
    scene.last_applied_unix_ms = Some(now_ms);
}

async fn persist_scene(state: &AppState, scene: chemlab_core::Scene) -> Result<LabScene, ApiError> {
    let contract_scene = scene_to_contract(scene);
    let state_blob = serde_json::to_vec(&contract_scene)
        .map_err(|error| ApiError::internal(format!("serialize lab scene: {error}")))?;
    save_lab_state(
        state.pool(),
        &contract_scene.lab_id,
        &state_blob,
        i64::from(contract_scene.version),
    )
    .await?;
    Ok(contract_scene)
}

fn lab_version(lab: &LabRecord) -> Result<u32, ApiError> {
    u32::try_from(lab.version).map_err(|_| ApiError::internal("Invalid lab version"))
}

fn action_to_core(action: LabAction) -> chemlab_core::Action {
    match action {
        LabAction::UseTool {
            tool_item_id,
            target_item_id,
        } => chemlab_core::Action::UseTool {
            tool_item_id,
            target_item_id,
        },
        LabAction::Pour {
            source_item_id,
            target_item_id,
        } => chemlab_core::Action::Pour {
            source_item_id,
            target_item_id,
        },
        LabAction::PutAway { tool_item_id } => chemlab_core::Action::PutAway { tool_item_id },
        LabAction::Reset => chemlab_core::Action::Reset,
        LabAction::ToggleBurner { burner_item_id } => {
            chemlab_core::Action::ToggleBurner { burner_item_id }
        }
    }
}

fn scene_to_contract(scene: chemlab_core::Scene) -> LabScene {
    LabScene {
        lab_id: scene.lab_id,
        version: scene.version,
        temperature_c: scene.temperature_c,
        items: scene
            .items
            .into_iter()
            .map(|item| chemlab_contracts::Item {
                id: item.id,
                kind: item.kind,
                label: item.label,
                location: item.location,
                properties: chemlab_contracts::ItemProperties {
                    volume_ml: item.properties.volume_ml,
                    fill_ml: item.properties.fill_ml,
                    transparent: item.properties.transparent,
                    colourless: item.properties.colourless,
                    temperature_c: item.properties.temperature_c,
                    composition: item
                        .properties
                        .composition
                        .into_iter()
                        .map(composition_to_contract)
                        .collect(),
                    holding: item
                        .properties
                        .holding
                        .into_iter()
                        .map(composition_to_contract)
                        .collect(),
                    on: item.properties.on,
                    source_item_id: item.properties.source_item_id,
                },
            })
            .collect(),
        last_events: scene
            .last_events
            .into_iter()
            .map(|event| LabEvent {
                kind: event.kind,
                message: event.message,
            })
            .collect(),
        last_applied_unix_ms: scene.last_applied_unix_ms,
    }
}

fn composition_to_contract(
    entry: chemlab_core::CompositionEntry,
) -> chemlab_contracts::CompositionEntry {
    chemlab_contracts::CompositionEntry {
        substance_id: entry.substance_id,
        phase: entry.phase,
        amount_ml: entry.amount_ml,
        amount_scoop: entry.amount_scoop,
        amount_g: entry.amount_g,
        amount_mol: entry.amount_mol,
    }
}

fn contract_to_scene(scene: LabScene) -> chemlab_core::Scene {
    chemlab_core::Scene {
        lab_id: scene.lab_id,
        version: scene.version,
        temperature_c: scene.temperature_c,
        items: scene
            .items
            .into_iter()
            .map(|item| chemlab_core::SceneItem {
                id: item.id,
                kind: item.kind,
                label: item.label,
                location: item.location,
                properties: chemlab_core::ItemProperties {
                    volume_ml: item.properties.volume_ml,
                    fill_ml: item.properties.fill_ml,
                    transparent: item.properties.transparent,
                    colourless: item.properties.colourless,
                    temperature_c: item.properties.temperature_c,
                    composition: item
                        .properties
                        .composition
                        .into_iter()
                        .map(composition_to_core)
                        .collect(),
                    holding: item
                        .properties
                        .holding
                        .into_iter()
                        .map(composition_to_core)
                        .collect(),
                    on: item.properties.on,
                    source_item_id: item.properties.source_item_id,
                },
            })
            .collect(),
        last_events: scene
            .last_events
            .into_iter()
            .map(|event| chemlab_core::SceneEvent {
                kind: event.kind,
                message: event.message,
            })
            .collect(),
        last_applied_unix_ms: scene.last_applied_unix_ms,
    }
}

fn composition_to_core(
    entry: chemlab_contracts::CompositionEntry,
) -> chemlab_core::CompositionEntry {
    chemlab_core::CompositionEntry {
        substance_id: entry.substance_id,
        phase: entry.phase,
        amount_ml: entry.amount_ml,
        amount_scoop: entry.amount_scoop,
        amount_g: entry.amount_g,
        amount_mol: entry.amount_mol,
    }
}

fn to_auth_user(user: &UserRecord) -> AuthUserResponse {
    AuthUserResponse {
        id: user.id.clone(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        created_at: user.created_at,
    }
}
