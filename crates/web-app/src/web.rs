//! HTTP layer: routing, form parsing, session cookies, and rendering.
//! Routes stay thin and delegate policy to the accounts service.

mod addresses;
mod authoring;
mod charts;
#[cfg(test)]
mod charts_tests;
mod learning;
#[cfg(test)]
mod learning_tests;
mod login_limit;
mod moves;
#[cfg(test)]
mod moves_tests;
mod stats;
#[cfg(test)]
mod stats_tests;
mod types;
#[cfg(test)]
mod types_tests;

use crate::{
    accounts::Accounts,
    application::authoring::{Authoring, FormRecord, SpeciesRecord, WorldRecord},
    auth::{Actor, AuthError, Credential, IdentityProvider},
};
use askama::Template;
use axum::{
    Form, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State, rejection::FormRejection},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use login_limit::LoginLimit;
use serde::Deserialize;
use std::{sync::Arc, time::Instant};

const SESSION_COOKIE: &str = "karta_session";
const CSRF_COOKIE: &str = "karta_csrf";
const CSRF_LEN: usize = 64;
const AUTH_FORM_LIMIT: usize = 4096;

#[derive(Clone)]
pub struct AppState {
    authoring: Authoring,
    types: crate::application::types::Types,
    charts: crate::application::charts::Charts,
    stats: crate::application::stats::Stats,
    stat_values: crate::application::stat_values::StatValues,
    moves: crate::application::moves::Moves,
    learning: crate::application::learning::Learning,
    accounts: Accounts,
    provider: Option<Arc<dyn IdentityProvider>>,
    public_origin: String,
    cookie_secure: bool,
    login_limit: Arc<LoginLimit>,
}

impl AppState {
    pub fn new(
        accounts: Accounts,
        provider: Option<Arc<dyn IdentityProvider>>,
        public_origin: String,
        cookie_secure: bool,
    ) -> Self {
        Self {
            authoring: Authoring::new(accounts.clone()),
            types: crate::application::types::Types::new(accounts.clone()),
            charts: crate::application::charts::Charts::new(accounts.clone()),
            stats: crate::application::stats::Stats::new(accounts.clone()),
            stat_values: crate::application::stat_values::StatValues::new(accounts.clone()),
            moves: crate::application::moves::Moves::new(accounts.clone()),
            learning: crate::application::learning::Learning::new(accounts.clone()),
            accounts,
            provider,
            public_origin,
            cookie_secure,
            login_limit: Arc::new(LoginLimit::new()),
        }
    }
}

pub fn router(state: AppState) -> Router {
    let inner = Router::new()
        .merge(authoring::routes())
        .merge(types::routes())
        .merge(charts::routes())
        .merge(stats::routes())
        .merge(moves::routes())
        .merge(learning::routes())
        .route("/", get(home))
        .route("/creatures", get(creatures))
        .route("/creatures/{id}", get(detail))
        .route(
            "/login",
            get(login_form)
                .post(login_submit)
                .layer(DefaultBodyLimit::max(AUTH_FORM_LIMIT))
                .layer(middleware::from_fn_with_state(state.clone(), limit_login)),
        )
        .route(
            "/logout",
            post(logout_submit).layer(DefaultBodyLimit::max(AUTH_FORM_LIMIT)),
        )
        .route("/account", get(account))
        .route("/static/style.css", get(styles))
        .route("/healthz", get(|| async { "ok\n" }))
        .fallback(not_found)
        .with_state(state.clone());
    // The adapter must run before inner route matching (including static `new`).
    Router::new()
        .fallback_service(inner)
        .layer(middleware::from_fn_with_state(state, addresses::adapt))
}

async fn limit_login(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if request.method() == Method::POST
        && let Err(retry_after) = state.login_limit.check(Instant::now())
    {
        let mut response = message(
            StatusCode::TOO_MANY_REQUESTS,
            "Please wait before signing in",
            "Too many sign-in attempts. Wait up to one minute, then try again.",
        );
        response.headers_mut().insert(
            header::RETRY_AFTER,
            HeaderValue::from_str(&retry_after.to_string()).expect("numeric Retry-After header"),
        );
        return response;
    }
    next.run(request).await
}

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
    count: usize,
}

#[derive(Template)]
#[template(path = "creatures.html")]
struct Creatures {
    creatures: Vec<SpeciesRecord>,
    query: String,
    can_edit_library: bool,
}

#[derive(Template)]
#[template(path = "detail.html")]
struct Detail {
    creature: SpeciesRecord,
    forms: Vec<FormOverview>,
    worlds: Vec<WorldRecord>,
    can_edit_library: bool,
}

struct FormOverview {
    form: FormRecord,
    typing: types::TypingSummary,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPage<'a> {
    title: &'a str,
    message: &'a str,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage<'a> {
    csrf: &'a str,
    username: &'a str,
    error: &'a str,
    dev_auth: bool,
}

#[derive(Template)]
#[template(path = "account.html")]
struct AccountPage<'a> {
    csrf: &'a str,
    display_name: &'a str,
    role: &'a str,
    user_id: &'a str,
}

#[derive(Template)]
#[template(path = "logout_error.html")]
struct LogoutFailurePage<'a> {
    csrf: &'a str,
}

fn render_status(template: impl Template, status: StatusCode) -> Response {
    match template.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(error) => {
            eprintln!("Template rendering failed: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "This page could not be displayed. Please try again.",
            )
                .into_response()
        }
    }
}

fn render(template: impl Template) -> Response {
    render_status(template, StatusCode::OK)
}

fn message(status: StatusCode, title: &str, body: &str) -> Response {
    render_status(
        ErrorPage {
            title,
            message: body,
        },
        status,
    )
}

async fn home(State(state): State<AppState>) -> Response {
    match state.authoring.species().await {
        Ok(species) => render(Home {
            count: species.len(),
        }),
        Err(error) => authoring::read_error(error),
    }
}

#[derive(Default, Deserialize)]
struct Search {
    #[serde(default)]
    q: String,
}

async fn creatures(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<Search>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let query = match query {
        Ok(Query(query)) if query.q.chars().count() <= 100 => query,
        _ => {
            return message(
                StatusCode::BAD_REQUEST,
                "Check your search",
                "Search by name using at most 100 characters. Return to the catalog to try again.",
            );
        }
    };
    let species = match state.authoring.species().await {
        Ok(species) => species,
        Err(error) => return authoring::read_error(error),
    };
    let capabilities = match state
        .authoring
        .capabilities(cookie_value(&headers, SESSION_COOKIE).as_deref(), None)
        .await
    {
        Ok(value) => value,
        Err(error) => return authoring::read_error(error),
    };
    let needle = query.q.trim().to_lowercase();
    render(Creatures {
        creatures: species
            .into_iter()
            .filter(|species| species.fields.display_name.to_lowercase().contains(&needle))
            .collect(),
        query: query.q,
        can_edit_library: capabilities.can_edit_library,
    })
}

async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let result = async {
        let creature = state.authoring.species_detail(&id).await?;
        let mut forms = Vec::new();
        for form in state.authoring.forms(&id).await? {
            let typing =
                types::TypingSummary::new(state.types.form_typing(&form.id, None).await?, None);
            forms.push(FormOverview { form, typing });
        }
        let worlds = state.authoring.availability(&id).await?;
        let capabilities = state
            .authoring
            .capabilities(cookie_value(&headers, SESSION_COOKIE).as_deref(), None)
            .await?;
        Ok(render(Detail {
            creature,
            forms,
            worlds,
            can_edit_library: capabilities.can_edit_library,
        }))
    }
    .await;
    authoring::finish(result)
}

async fn not_found() -> Response {
    message(
        StatusCode::NOT_FOUND,
        "Page not found",
        "We could not find that page or creature. Browse the catalog to find an available entry.",
    )
}

async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../static/style.css"),
    )
}

async fn login_form(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match current_actor(&state, &headers).await {
        Ok(Some(_)) => return Redirect::to("/account").into_response(),
        Ok(None) => {}
        Err(_) => {
            return temporarily_unavailable("Your session could not be checked. Please try again.");
        }
    }
    let (csrf, set) = match csrf_for(&headers) {
        Ok(value) => value,
        Err(_) => {
            return temporarily_unavailable(
                "The sign-in form could not be prepared. Please reload and try again.",
            );
        }
    };
    login_response(&state, &csrf, "", "", StatusCode::OK, set)
}

#[derive(Deserialize)]
struct LoginForm {
    csrf: String,
    username: String,
}

async fn login_submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    form: Result<Form<LoginForm>, axum::extract::rejection::FormRejection>,
) -> Response {
    if !origin_ok(&headers, &state.public_origin) {
        return message(
            StatusCode::FORBIDDEN,
            "Request blocked",
            "This form must be submitted from Karta Yaxu itself.",
        );
    }
    let form = match form {
        Ok(Form(form)) => form,
        Err(error) => return form_error(error),
    };
    if !csrf_matches(&headers, &form.csrf) {
        return message(
            StatusCode::FORBIDDEN,
            "Request blocked",
            "This form has expired or was not submitted from Karta Yaxu. Reload the sign-in page and try again.",
        );
    }
    // Matching a valid cookie guarantees a valid existing token, without RNG work.
    let csrf = &form.csrf;
    let set = false;
    let Some(provider) = state.provider.clone() else {
        return login_response(
            &state,
            csrf,
            &form.username,
            "Authentication is disabled. Set KARTA_DEV_AUTH=1 for local development.",
            StatusCode::SERVICE_UNAVAILABLE,
            set,
        );
    };
    let subject = match provider
        .authenticate(Credential::DevUsername(form.username.clone()))
        .await
    {
        Ok(subject) => subject,
        Err(error) => {
            let (status, text) = auth_error(&error);
            return login_response(&state, csrf, &form.username, text, status, set);
        }
    };
    let actor = match state.accounts.sign_in(subject).await {
        Ok(actor) => actor,
        Err(error) => {
            let (status, text) = auth_error(&error);
            return login_response(&state, csrf, &form.username, text, status, set);
        }
    };
    let (token, _expires_at) = match state.accounts.start_session(&actor.user_id).await {
        Ok(value) => value,
        Err(error) => {
            eprintln!("session start failed: {error}");
            return login_response(
                &state,
                csrf,
                &form.username,
                auth_error(&error).1,
                auth_error(&error).0,
                set,
            );
        }
    };
    let mut response = Redirect::to("/account").into_response();
    append_cookie(
        &mut response,
        session_cookie(&token, state.accounts.session_ttl(), state.cookie_secure),
    );
    response
}

#[derive(Deserialize)]
struct LogoutForm {
    csrf: String,
}

async fn logout_submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    form: Result<Form<LogoutForm>, axum::extract::rejection::FormRejection>,
) -> Response {
    if !origin_ok(&headers, &state.public_origin) {
        return message(
            StatusCode::FORBIDDEN,
            "Request blocked",
            "This form must be submitted from Karta Yaxu itself.",
        );
    }
    let form = match form {
        Ok(Form(form)) => form,
        Err(error) => return form_error(error),
    };
    if !csrf_matches(&headers, &form.csrf) {
        return message(
            StatusCode::FORBIDDEN,
            "Request blocked",
            "This form has expired or was not submitted from Karta Yaxu. Reload and try again.",
        );
    }
    let token = match unique_cookie_value(&headers, SESSION_COOKIE) {
        Ok(token) => token,
        Err(()) => {
            return message(
                StatusCode::BAD_REQUEST,
                "Session cookies could not be read",
                "Sign-out was not completed because this browser sent conflicting or unreadable cookies. Please retry from the browser you used to sign in.",
            );
        }
    };
    if let Some(token) = token
        && let Err(error) = state.accounts.sign_out(&token).await
    {
        eprintln!("session revoke failed: {error}");
        return render_status(
            LogoutFailurePage { csrf: &form.csrf },
            StatusCode::SERVICE_UNAVAILABLE,
        );
    }
    let mut response = Redirect::to("/").into_response();
    append_cookie(
        &mut response,
        clear_cookie(SESSION_COOKIE, state.cookie_secure),
    );
    append_cookie(
        &mut response,
        clear_cookie(CSRF_COOKIE, state.cookie_secure),
    );
    response
}

async fn account(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let actor = match current_actor(&state, &headers).await {
        Ok(Some(actor)) => actor,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(_) => {
            return temporarily_unavailable("Your account could not be loaded. Please try again.");
        }
    };
    let (csrf, set) = match csrf_for(&headers) {
        Ok(value) => value,
        Err(_) => {
            return temporarily_unavailable(
                "The sign-out form could not be prepared. Please reload and try again.",
            );
        }
    };
    let mut response = render_status(
        AccountPage {
            csrf: &csrf,
            display_name: &actor.display_name,
            role: actor.role.as_str(),
            user_id: &actor.user_id,
        },
        StatusCode::OK,
    );
    if set {
        append_cookie(&mut response, csrf_cookie(&csrf, state.cookie_secure));
    }
    response
}

async fn current_actor(state: &AppState, headers: &HeaderMap) -> Result<Option<Actor>, AuthError> {
    let Some(token) = cookie_value(headers, SESSION_COOKIE) else {
        return Ok(None);
    };
    state.accounts.authenticate(&token).await
}

fn temporarily_unavailable(text: &str) -> Response {
    message(
        StatusCode::SERVICE_UNAVAILABLE,
        "Authentication temporarily unavailable",
        text,
    )
}

fn form_error(error: FormRejection) -> Response {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        message(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Form too large",
            "Authentication forms must be no larger than 4 KiB. Return to the form and try again.",
        )
    } else {
        message(
            StatusCode::BAD_REQUEST,
            "Check the form",
            "This form could not be read. Return to the form and try again.",
        )
    }
}

fn auth_error(error: &AuthError) -> (StatusCode, &'static str) {
    match error {
        AuthError::InvalidCredential(text) => (StatusCode::BAD_REQUEST, text),
        AuthError::NotInvited => (
            StatusCode::FORBIDDEN,
            "This username has not been invited. Ask an administrator to invite it.",
        ),
        AuthError::Disabled => (StatusCode::FORBIDDEN, "This account is disabled."),
        AuthError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Sign-in is temporarily unavailable.",
        ),
    }
}

fn login_response(
    state: &AppState,
    csrf: &str,
    username: &str,
    error: &str,
    status: StatusCode,
    set_csrf: bool,
) -> Response {
    let mut response = render_status(
        LoginPage {
            csrf,
            username,
            error,
            dev_auth: state.provider.is_some(),
        },
        status,
    );
    if set_csrf {
        append_cookie(&mut response, csrf_cookie(csrf, state.cookie_secure));
    }
    response
}

fn origin_ok(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        == Some(expected)
}

fn csrf_for(headers: &HeaderMap) -> Result<(String, bool), AuthError> {
    csrf_for_with(headers, || random_hex(32))
}

fn csrf_for_with(
    headers: &HeaderMap,
    generate: impl FnOnce() -> Option<String>,
) -> Result<(String, bool), AuthError> {
    match cookie_value(headers, CSRF_COOKIE) {
        Some(value) if valid_csrf(&value) => Ok((value, false)),
        _ => generate()
            .filter(|value| valid_csrf(value))
            .map(|value| (value, true))
            .ok_or(AuthError::Unavailable),
    }
}

fn csrf_matches(headers: &HeaderMap, submitted: &str) -> bool {
    match cookie_value(headers, CSRF_COOKIE) {
        Some(value) => {
            valid_csrf(&value) && valid_csrf(submitted) && constant_time_eq(&value, submitted)
        }
        None => false,
    }
}

fn valid_csrf(value: &str) -> bool {
    value.len() == CSRF_LEN && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn random_hex(bytes: usize) -> Option<String> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).ok()?;
    Some(hex::encode(buffer))
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    unique_cookie_value(headers, name).ok().flatten()
}

fn unique_cookie_value(headers: &HeaderMap, name: &str) -> Result<Option<String>, ()> {
    let mut found = None;
    for header in headers.get_all(header::COOKIE) {
        for part in header.to_str().map_err(|_| ())?.split(';') {
            let Some((key, value)) = part.trim().split_once('=') else {
                continue;
            };
            if key == name {
                if found.is_some() {
                    return Err(());
                }
                found = Some(value.to_owned());
            }
        }
    }
    Ok(found)
}

fn append_cookie(response: &mut Response, cookie: String) {
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}

fn session_cookie(token: &str, ttl_seconds: i64, secure: bool) -> String {
    format!(
        "{SESSION_COOKIE}={token}; Path=/; Max-Age={ttl_seconds}; HttpOnly; SameSite=Strict{}",
        secure_suffix(secure)
    )
}

fn csrf_cookie(value: &str, secure: bool) -> String {
    format!(
        "{CSRF_COOKIE}={value}; Path=/; Max-Age=3600; HttpOnly; SameSite=Strict{}",
        secure_suffix(secure)
    )
}

fn clear_cookie(name: &str, secure: bool) -> String {
    format!(
        "{name}=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict{}",
        secure_suffix(secure)
    )
}

fn secure_suffix(secure: bool) -> &'static str {
    if secure { "; Secure" } else { "" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csrf_entropy_failure_never_produces_an_empty_token() {
        let headers = HeaderMap::new();
        assert_eq!(
            csrf_for_with(&headers, || None),
            Err(AuthError::Unavailable)
        );
        assert_eq!(
            csrf_for_with(&headers, || Some(String::new())),
            Err(AuthError::Unavailable)
        );
    }
}
