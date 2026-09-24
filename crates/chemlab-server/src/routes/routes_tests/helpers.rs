use super::super::*;
use crate::config::Config;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

pub(crate) fn test_config() -> Config {
    Config {
        bind_addr: "127.0.0.1:0".into(),
        database_url: "sqlite::memory:?cache=shared".into(),
        static_dir: None,
        vite_dev_proxy: None,
        cookie_secure: false,
        signup_enabled: true,
        session_ttl_hours: 24,
    }
}

pub(crate) async fn test_app_state() -> (Router, AppState) {
    test_app_state_with(test_config()).await
}

pub(crate) async fn test_app_state_with(config: Config) -> (Router, AppState) {
    let state = AppState::new(&config).await.expect("state");
    (router(state.clone()), state)
}

pub(crate) async fn test_app() -> Router {
    test_app_state().await.0
}

pub(crate) async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

pub(crate) async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

pub(crate) fn set_cookie_headers(response: &axum::response::Response) -> Vec<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().to_string())
        .collect()
}

pub(crate) fn cookie_has_secure(set_cookie: &str) -> bool {
    set_cookie
        .split(';')
        .any(|part| part.trim().eq_ignore_ascii_case("secure"))
}

pub(crate) fn assert_html_security_headers(response: &axum::response::Response) {
    use axum::http::header::{
        CONTENT_SECURITY_POLICY, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
    };
    use frontend::CONTENT_SECURITY_POLICY_VALUE;

    assert_eq!(
        response
            .headers()
            .get(CONTENT_SECURITY_POLICY)
            .and_then(|v| v.to_str().ok()),
        Some(CONTENT_SECURITY_POLICY_VALUE)
    );
    assert_eq!(
        response
            .headers()
            .get(X_CONTENT_TYPE_OPTIONS)
            .and_then(|v| v.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        response
            .headers()
            .get(X_FRAME_OPTIONS)
            .and_then(|v| v.to_str().ok()),
        Some("SAMEORIGIN")
    );
    assert_eq!(
        response
            .headers()
            .get(REFERRER_POLICY)
            .and_then(|v| v.to_str().ok()),
        Some("strict-origin-when-cross-origin")
    );
    assert!(response
        .headers()
        .get("strict-transport-security")
        .is_none());
}

pub(crate) fn first_set_cookie(response: &axum::response::Response) -> String {
    response
        .headers()
        .get("set-cookie")
        .expect("set-cookie")
        .to_str()
        .unwrap()
        .to_string()
}

pub(crate) fn without_clock(scene: &serde_json::Value) -> serde_json::Value {
    let mut scene = scene.clone();
    if let Some(obj) = scene.as_object_mut() {
        obj.remove("last_applied_unix_ms");
    }
    scene
}

/// Strip item temperatures so ambient cool between POST and GET does not break equality.
pub(crate) fn without_item_temperatures(scene: &serde_json::Value) -> serde_json::Value {
    let mut scene = scene.clone();
    if let Some(items) = scene
        .as_object_mut()
        .and_then(|obj| obj.get_mut("items"))
        .and_then(|v| v.as_array_mut())
    {
        for item in items {
            if let Some(props) = item
                .as_object_mut()
                .and_then(|obj| obj.get_mut("properties"))
                .and_then(|v| v.as_object_mut())
            {
                props.remove("temperature_c");
            }
        }
    }
    scene
}

pub(crate) fn cookie_pair(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_string()
}

pub(crate) fn csrf_pair_from_response(response: &axum::response::Response) -> (String, String) {
    let set_cookie = set_cookie_headers(response)
        .into_iter()
        .find(|value| value.contains("chemlab_csrf="))
        .expect("chemlab_csrf set-cookie");
    let cookie = cookie_pair(&set_cookie);
    let token = cookie
        .split_once('=')
        .expect("csrf cookie value")
        .1
        .to_string();
    (token, cookie)
}

pub(crate) fn set_cookie_clears_cookie(set_cookie: &str, name: &str) -> bool {
    set_cookie.starts_with(&format!("{name}="))
        && set_cookie.to_ascii_lowercase().contains("max-age=0")
}

pub(crate) fn session_cookie_pair(response: &axum::response::Response) -> String {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("chemlab_session="))
        .map(cookie_pair)
        .expect("chemlab_session set-cookie")
}

pub(crate) async fn get_me(app: &Router, cookie: &str) -> serde_json::Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}

pub(crate) async fn issue_csrf(app: &Router) -> (String, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/csrf")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = cookie_pair(&first_set_cookie(&response));
    let json = body_json(response).await;
    let token = json["csrf_token"].as_str().expect("csrf_token").to_string();
    (token, cookie)
}

pub(crate) async fn post_login(
    app: &Router,
    csrf_token: &str,
    csrf_cookie: &str,
    email: &str,
    password: &str,
    forwarded_for: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .header("cookie", csrf_cookie)
        .header("x-csrf-token", csrf_token);
    if let Some(ip) = forwarded_for {
        builder = builder.header("x-forwarded-for", ip);
    }
    let body = format!(r#"{{"email":"{email}","password":"{password}"}}"#);
    app.clone()
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap()
}

pub(crate) async fn post_register(
    app: &Router,
    csrf_token: &str,
    csrf_cookie: &str,
    email: &str,
) -> axum::response::Response {
    let body = format!(r#"{{"email":"{email}","password":"secret123","display_name":"Ada"}}"#);
    post_register_body(app, csrf_token, csrf_cookie, body).await
}

pub(crate) async fn post_register_body(
    app: &Router,
    csrf_token: &str,
    csrf_cookie: &str,
    body: String,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("content-type", "application/json")
                .header("cookie", csrf_cookie)
                .header("x-csrf-token", csrf_token)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}

pub(crate) async fn register_user(app: &Router, email: &str) -> (String, String, String) {
    let (csrf_token, csrf_cookie) = issue_csrf(app).await;
    let register = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("content-type", "application/json")
                .header("cookie", &csrf_cookie)
                .header("x-csrf-token", &csrf_token)
                .body(Body::from(format!(
                    r#"{{"email":"{email}","password":"secret123","display_name":"Ada"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(register.status(), StatusCode::CREATED);
    let session_cookie = session_cookie_pair(&register);
    let (csrf_token, csrf_cookie) = csrf_pair_from_response(&register);
    (csrf_token, csrf_cookie, session_cookie)
}

pub(crate) fn dissolve_json(substance_id: &str, solvent_id: &str, temperature_c: i32) -> String {
    serde_json::json!({
        "substance_id": substance_id,
        "solvent_id": solvent_id,
        "temperature_c": temperature_c,
    })
    .to_string()
}

pub(crate) async fn post_dissolve(
    app: &Router,
    cookie: &str,
    csrf_token: Option<&str>,
    body: String,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/lab/dissolve")
        .header("content-type", "application/json")
        .header("cookie", cookie);
    if let Some(token) = csrf_token {
        builder = builder.header("x-csrf-token", token);
    }
    app.clone()
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap()
}

pub(crate) async fn get_scene(app: &Router, cookie: Option<&str>) -> axum::response::Response {
    let mut builder = Request::builder().uri("/api/lab/scene");
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    app.clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

pub(crate) async fn post_action(
    app: &Router,
    cookie: &str,
    csrf_token: Option<&str>,
    action: serde_json::Value,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/lab/action")
        .header("content-type", "application/json")
        .header("cookie", cookie);
    if let Some(token) = csrf_token {
        builder = builder.header("x-csrf-token", token);
    }
    app.clone()
        .oneshot(builder.body(Body::from(action.to_string())).unwrap())
        .await
        .unwrap()
}

pub(crate) fn scene_item<'a>(scene: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("missing item {id}"))
}

pub(crate) fn set_main_beaker_water(scene: &mut serde_json::Value, amount_ml: f64) {
    let water = scene["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["id"] == "beaker-water")
        .expect("beaker-water");
    water["properties"]["fill_ml"] = serde_json::json!(amount_ml);
    water["properties"]["composition"] = serde_json::json!([
        {
            "substance_id": "water",
            "phase": "liquid",
            "amount_ml": amount_ml
        }
    ]);
}

pub(crate) async fn save_scene_blob(state: &AppState, scene: &serde_json::Value) {
    let lab_id = scene["lab_id"].as_str().unwrap().to_string();
    let version = scene["version"].as_u64().unwrap() as i64;
    chemlab_db::save_lab_state(
        state.pool(),
        &lab_id,
        &serde_json::to_vec(scene).unwrap(),
        version,
    )
    .await
    .unwrap();
}

pub(crate) async fn persist_filled_main_beaker(state: &AppState, app: &Router, cookies: &str) {
    let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
    set_main_beaker_water(&mut scene, 200.0);
    save_scene_blob(state, &scene).await;
}

pub(crate) async fn pipette_one_ml_into_dish(app: &Router, cookies: &str, csrf_token: &str) {
    assert_eq!(
        post_action(
            app,
            cookies,
            Some(csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "pipette-1",
                "target_item_id": "beaker-h2o"
            }),
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        post_action(
            app,
            cookies,
            Some(csrf_token),
            serde_json::json!({
                "type": "pour",
                "source_item_id": "pipette-1",
                "target_item_id": "dish-1"
            }),
        )
        .await
        .status(),
        StatusCode::OK
    );
}

pub(crate) async fn persist_scene_without_tongs(state: &AppState, app: &Router, cookies: &str) {
    let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
    // Old labs had a filled "Water" beaker; backfill must not wipe that fill.
    scene["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["id"] == "beaker-water")
        .expect("beaker-water")["label"] = serde_json::json!("Water");
    set_main_beaker_water(&mut scene, 200.0);
    scene["items"]
        .as_array_mut()
        .unwrap()
        .retain(|item| item["id"] != "tongs-1");
    assert!(scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["id"] != "tongs-1"));
    save_scene_blob(state, &scene).await;
}

pub(crate) async fn persist_dry_dish_solids(state: &AppState, app: &Router, cookies: &str) {
    let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
    let dish = scene["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["id"] == "dish-1")
        .expect("dish-1");
    dish["properties"]["composition"] = serde_json::json!([
        {
            "substance_id": "nacl",
            "phase": "solid",
            "amount_g": 0.6
        },
        {
            "substance_id": "cacl2",
            "phase": "solid",
            "amount_g": 0.4
        }
    ]);
    dish["properties"]["fill_ml"] = serde_json::json!(0.0);
    let lab_id = scene["lab_id"].as_str().unwrap().to_string();
    let version = scene["version"].as_u64().unwrap() as i64;
    chemlab_db::save_lab_state(
        state.pool(),
        &lab_id,
        &serde_json::to_vec(&scene).unwrap(),
        version,
    )
    .await
    .unwrap();
}

pub(crate) async fn persist_scene_without_beaker_h2o(
    state: &AppState,
    app: &Router,
    cookies: &str,
) {
    let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
    scene["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["id"] == "beaker-water")
        .expect("beaker-water")["label"] = serde_json::json!("Water");
    set_main_beaker_water(&mut scene, 200.0);
    scene["items"]
        .as_array_mut()
        .unwrap()
        .retain(|item| item["id"] != "beaker-h2o");
    assert!(scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["id"] != "beaker-h2o"));
    save_scene_blob(state, &scene).await;
}

pub(crate) async fn persist_scene_without_filtration(
    state: &AppState,
    app: &Router,
    cookies: &str,
) {
    let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
    scene["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["id"] == "beaker-water")
        .expect("beaker-water")["label"] = serde_json::json!("Water");
    set_main_beaker_water(&mut scene, 200.0);
    scene["items"]
        .as_array_mut()
        .unwrap()
        .retain(|item| item["id"] != "beaker-filtrate" && item["id"] != "filter-paper-1");
    assert!(scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| { item["id"] != "beaker-filtrate" && item["id"] != "filter-paper-1" }));
    save_scene_blob(state, &scene).await;
}
