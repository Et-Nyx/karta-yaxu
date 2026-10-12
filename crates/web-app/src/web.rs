use crate::application;
use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use game_data::{Catalog, Creature};
use serde::Deserialize;
use std::sync::Arc;

pub fn router(catalog: Catalog) -> Router {
    Router::new()
        .route("/", get(home))
        .route("/creatures", get(creatures))
        .route("/creatures/{id}", get(detail))
        .route("/static/style.css", get(styles))
        .route("/healthz", get(|| async { "ok\n" }))
        .fallback(not_found)
        .with_state(Arc::new(catalog))
}

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
    count: usize,
}

#[derive(Template)]
#[template(path = "creatures.html")]
struct Creatures<'a> {
    creatures: Vec<&'a Creature>,
    query: &'a str,
}

#[derive(Template)]
#[template(path = "detail.html")]
struct Detail<'a> {
    creature: &'a Creature,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPage<'a> {
    title: &'a str,
    message: &'a str,
}

fn render(template: impl Template) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
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

async fn home(State(catalog): State<Arc<Catalog>>) -> Response {
    render(Home {
        count: catalog.creatures().len(),
    })
}

#[derive(Default, Deserialize)]
struct Search {
    #[serde(default)]
    q: String,
}

async fn creatures(
    State(catalog): State<Arc<Catalog>>,
    query: Result<Query<Search>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let query = match query {
        Ok(Query(query)) if query.q.chars().count() <= 100 => query,
        _ => return (StatusCode::BAD_REQUEST, render(ErrorPage {
            title: "Check your search", message: "Search by name using at most 100 characters. Return to the catalog to try again.",
        })).into_response(),
    };
    render(Creatures {
        creatures: application::search(&catalog, &query.q),
        query: &query.q,
    })
}

async fn detail(State(catalog): State<Arc<Catalog>>, Path(id): Path<String>) -> Response {
    match application::find(&catalog, &id) {
        Some(creature) => render(Detail { creature }),
        None => not_found().await,
    }
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, render(ErrorPage { title: "Page not found", message: "We could not find that page or creature. Browse the catalog to find an available entry." })).into_response()
}

async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../static/style.css"),
    )
}
