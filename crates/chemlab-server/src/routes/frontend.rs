//! Serve the Vite-built SPA, or reverse-proxy to the Vite dev server.
//!
//! Dev (see README):
//! - `CHEMLAB_VITE_PROXY=http://127.0.0.1:5179` → Axum forwards non-API paths to Vite
//! - or open Vite directly with `/api` proxied to Axum
//!
//! Prod-like: `CHEMLAB_STATIC_DIR=apps/web/dist` serves `index.html` + assets.

use crate::state::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

pub fn router(state: AppState) -> Router<AppState> {
    // Always serve the favicon so opening /api/health (or the embedded welcome)
    // does not produce a 5xx when the browser auto-requests /favicon.ico.
    let mut router = Router::new()
        .route("/", get(welcome))
        .route("/favicon.svg", get(favicon_svg))
        .route("/favicon.ico", get(favicon_svg));

    if state.inner.vite_dev_proxy.is_some() {
        router = router.fallback(proxy_to_vite);
    } else if let Some(static_dir) = state.inner.static_dir.clone() {
        let index = static_dir.join("index.html");
        let serve = ServeDir::new(static_dir).not_found_service(ServeFile::new(index));
        router = router.fallback_service(serve);
    } else {
        router = router.fallback(missing_frontend);
    }

    router
}

async fn favicon_svg() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "image/svg+xml")
        .header("cache-control", "public, max-age=86400")
        .body(Body::from(include_str!("../../../../apps/web/public/favicon.svg")))
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "favicon").into_response())
}

async fn welcome(State(state): State<AppState>) -> Response {
    if let Some(proxy) = &state.inner.vite_dev_proxy {
        return proxy_request(proxy, "/").await;
    }
    if let Some(static_dir) = &state.inner.static_dir {
        let index = static_dir.join("index.html");
        match tokio::fs::read_to_string(&index).await {
            Ok(html) => Html(html).into_response(),
            Err(_) => missing_frontend_message().into_response(),
        }
    } else {
        Html(embedded_welcome_html()).into_response()
    }
}

async fn proxy_to_vite(State(state): State<AppState>, req: Request<Body>) -> Response {
    let Some(base) = &state.inner.vite_dev_proxy else {
        return missing_frontend().await;
    };
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    proxy_request(base, path).await
}

async fn proxy_request(base: &str, path: &str) -> Response {
    let target = format!("{}{}", base.trim_end_matches('/'), path);
    let client = reqwest::Client::new();
    match client.get(&target).send().await {
        Ok(upstream) => {
            let status =
                StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let content_type = upstream
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            match upstream.bytes().await {
                Ok(bytes) => {
                    let mut builder = Response::builder().status(status);
                    if let Some(ct) = content_type {
                        builder = builder.header("content-type", ct);
                    }
                    builder.body(Body::from(bytes)).unwrap_or_else(|_| {
                        (StatusCode::BAD_GATEWAY, "proxy response build failed").into_response()
                    })
                }
                Err(err) => (
                    StatusCode::BAD_GATEWAY,
                    format!("Vite proxy body error: {err}"),
                )
                    .into_response(),
            }
        }
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            format!("Vite proxy failed ({err}). Start the Vite app or unset CHEMLAB_VITE_PROXY."),
        )
            .into_response(),
    }
}

async fn missing_frontend() -> Response {
    missing_frontend_message().into_response()
}

fn missing_frontend_message() -> (StatusCode, Html<&'static str>) {
    // 404 (not 503): unknown paths like /favicon.ico used to 503 and make
    // tower_http::trace log ERROR when browsing /api/health with no SPA configured.
    (
        StatusCode::NOT_FOUND,
        Html(
            "<!doctype html><html><body style='font-family:sans-serif;padding:2rem'>\
             <h1>ChemLab frontend not configured</h1>\
             <p>Set <code>CHEMLAB_VITE_PROXY</code> (dev) or <code>CHEMLAB_STATIC_DIR</code> (built assets),\
             or open the Vite app directly. See README.</p>\
             <p>API health: <a href='/api/health'>/api/health</a></p>\
             </body></html>",
        ),
    )
}

fn embedded_welcome_html() -> String {
    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
  <title>ChemLab</title>
  <style>
    :root { color-scheme: dark; --ink:#f1f5f7; --ink-soft:#aebbc6; --accent:#5eb8b0; }
    body { margin:0; min-height:100vh; font-family: Georgia, "Times New Roman", serif;
      background: radial-gradient(1000px 560px at 8% 0%, rgba(55,110,120,0.22) 0%, transparent 55%),
                  linear-gradient(168deg, #070a0d 0%, #0e1318 45%, #0a1014 100%);
      color: var(--ink); display:grid; place-items:center; }
    main { max-width: 36rem; padding: 2rem; text-align: left; }
    .brand { font-size: clamp(2.8rem, 8vw, 4.5rem); letter-spacing: -0.03em; margin:0; font-weight:700; }
    p { font-size: 1.15rem; line-height: 1.5; color: var(--ink-soft); }
    a { color: var(--accent); }
  </style>
</head>
<body>
  <main>
    <p class="brand">ChemLab</p>
    <p>ChemLab is warming up. Start the Vite app for the full welcome page, or hit the API.</p>
    <p><a href="/api/health">API health</a></p>
  </main>
</body>
</html>"#
    .to_string()
}
