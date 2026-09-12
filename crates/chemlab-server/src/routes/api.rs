use crate::auth::{
    clear_session_cookie, csrf_cookie, csrf_token_from_jar, generate_session_token, hash_password,
    hash_token, require_csrf, session_cookie, token_from_jar, validate_credentials,
    verify_password,
};
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::CookieJar;
use chemlab_contracts::{
    AuthUserResponse, CsrfResponse, HealthResponse, LoginRequest, MeResponse, RegisterRequest,
};
use chemlab_db::{
    create_session, delete_session_by_token_hash, find_user_by_email,
    find_valid_session_by_token_hash, insert_user, UserRecord,
};
use chrono::Duration;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/csrf", get(csrf))
        .route("/api/auth/me", get(me))
        .merge(csrf_protected())
}

/// Mutating JSON routes. Later lab POSTs should join this router (or `.layer(from_fn(require_csrf))`).
fn csrf_protected() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
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
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, CookieJar, Json<AuthUserResponse>), ApiError> {
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
    Json(body): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthUserResponse>), ApiError> {
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

fn to_auth_user(user: &UserRecord) -> AuthUserResponse {
    AuthUserResponse {
        id: user.id.clone(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        created_at: user.created_at,
    }
}
