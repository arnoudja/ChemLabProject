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
use axum::http::header::{
    CONTENT_SECURITY_POLICY, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderValue, Request, StatusCode};
use axum::middleware::map_response;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

/// Locked CSP (no `unsafe-inline` / `unsafe-eval`). Keep in sync with `packaging/caddy/Caddyfile`.
pub(crate) const CONTENT_SECURITY_POLICY_VALUE: &str =
    "default-src 'self'; script-src 'self'; style-src 'self' https://fonts.googleapis.com; \
     font-src 'self' https://fonts.gstatic.com; img-src 'self' data:; connect-src 'self'; \
     frame-ancestors 'self'; base-uri 'self'; form-action 'self'";

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
        // Only mount ServeDir when the SPA entry exists; otherwise unknown paths
        // get a clear 404 HTML page instead of a broken file service.
        if index.is_file() {
            // SPA client routes: unknown paths serve index.html with 200 (not 404).
            let serve = ServeDir::new(static_dir).fallback(ServeFile::new(index));
            router = router.fallback_service(serve);
        } else {
            router = router.fallback(missing_frontend);
        }
    } else {
        router = router.fallback(missing_frontend);
    }

    router.layer(map_response(apply_html_security_headers_middleware))
}

async fn apply_html_security_headers_middleware<B>(mut response: Response<B>) -> Response<B> {
    if !response_is_html(&response) {
        return response;
    }
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CONTENT_SECURITY_POLICY_VALUE),
    );
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("SAMEORIGIN"));
    headers.insert(
        REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}

fn response_is_html<B>(response: &Response<B>) -> bool {
    response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| {
            let ct = ct
                .split(';')
                .next()
                .unwrap_or(ct)
                .trim()
                .to_ascii_lowercase();
            ct == "text/html" || ct == "application/xhtml+xml"
        })
        .unwrap_or(false)
}

async fn favicon_svg() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "image/svg+xml")
        .header("cache-control", "public, max-age=86400")
        .body(Body::from(include_str!(
            "../../../../apps/web/public/favicon.svg"
        )))
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
            "<!doctype html><html><body>\
             <h1>ChemLab frontend not configured</h1>\
             <p>Set <code>CHEMLAB_VITE_PROXY</code> (dev) or <code>CHEMLAB_STATIC_DIR</code> to a built SPA\
             that contains <code>index.html</code>. If <code>CHEMLAB_STATIC_DIR</code> is set but the path\
             is missing or incomplete, fix the directory and restart. See README.</p>\
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
</head>
<body>
  <main>
    <p>ChemLab</p>
    <p>ChemLab is warming up. Start the Vite app for the full welcome page, or hit the API.</p>
    <p><a href="/api/health">API health</a></p>
  </main>
</body>
</html>"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    fn assert_html_security_headers(response: &Response) {
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
        assert!(
            response
                .headers()
                .get("strict-transport-security")
                .is_none(),
            "HSTS must not be set on Axum HTML"
        );
    }

    #[tokio::test]
    async fn apply_headers_only_for_html_content_type() {
        let html = apply_html_security_headers_middleware(
            Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_html_security_headers(&html);

        let json = apply_html_security_headers_middleware(
            Response::builder()
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert!(json.headers().get(CONTENT_SECURITY_POLICY).is_none());
    }
}
