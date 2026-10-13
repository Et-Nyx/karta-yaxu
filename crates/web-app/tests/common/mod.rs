#![allow(dead_code)]

use std::sync::Arc;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, Response, StatusCode, header},
};
use game_data::{BUNDLED_FIXTURES, Catalog};
use sqlx::SqlitePool;
use tempfile::TempDir;
use tower::ServiceExt;
use web_app::{
    AppState,
    accounts::Accounts,
    auth::{IdentityProvider, placeholder::PlaceholderProvider},
    persistence, router,
};

pub struct TestApp {
    pub router: Router,
    pub pool: SqlitePool,
    pub accounts: Accounts,
    _dir: TempDir,
}

pub async fn app_with_catalog(
    catalog: Catalog,
    provider: Option<Arc<dyn IdentityProvider>>,
) -> TestApp {
    app_with_options(catalog, provider, false).await
}

pub async fn app_with_options(
    catalog: Catalog,
    provider: Option<Arc<dyn IdentityProvider>>,
    cookie_secure: bool,
) -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/test.db", dir.path().display());
    let pool = persistence::connect_for_migration(&url).await.unwrap();
    persistence::migrate(&pool).await.unwrap();
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    let state = AppState::new(
        catalog,
        accounts.clone(),
        provider,
        "http://127.0.0.1:3000".to_owned(),
        cookie_secure,
    );
    TestApp {
        router: router(state),
        pool,
        accounts,
        _dir: dir,
    }
}

pub async fn app_with(provider: Option<Arc<dyn IdentityProvider>>) -> TestApp {
    let catalog = Catalog::from_json(BUNDLED_FIXTURES).unwrap();
    app_with_catalog(catalog, provider).await
}

pub async fn app() -> TestApp {
    app_with(Some(Arc::new(PlaceholderProvider::new()))).await
}

pub async fn send(router: &Router, request: Request<Body>) -> Response<Body> {
    router.clone().oneshot(request).await.unwrap()
}

pub fn get(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap()
}

pub fn post_form(
    path: &str,
    origin: Option<&str>,
    cookie: Option<&str>,
    form: &str,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if let Some(origin) = origin {
        builder = builder.header(header::ORIGIN, origin);
    }
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    builder.body(Body::from(form.to_owned())).unwrap()
}

pub fn with_cookie(mut request: Request<Body>, cookie: String) -> Request<Body> {
    request
        .headers_mut()
        .insert(header::COOKIE, cookie.parse().unwrap());
    request
}

pub async fn read_body(response: Response<Body>) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

pub async fn text(router: &Router, path: &str) -> (StatusCode, String) {
    let response = send(router, get(path)).await;
    let status = response.status();
    (status, read_body(response).await)
}

/// Return the value of a `Set-Cookie` for `name`, if present.
pub fn set_cookie(response: &Response<Body>, name: &str) -> Option<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|cookie| {
            let (key, rest) = cookie.split_once('=')?;
            if key == name {
                Some(rest.split(';').next()?.to_owned())
            } else {
                None
            }
        })
}

pub fn location(response: &Response<Body>) -> Option<String> {
    response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}
