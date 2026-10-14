mod common;

use axum::http::StatusCode;
use game_data::{BUNDLED_FIXTURES, Catalog};

#[tokio::test]
async fn browse_search_and_follow_detail() {
    let app = common::app().await;

    let (status, home) = common::text(&app.router, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(home.contains("Karta Yaxu"));

    let (status, catalog) = common::text(&app.router, "/creatures").await;
    assert_eq!(status, StatusCode::OK);
    for creature in Catalog::from_json(BUNDLED_FIXTURES).unwrap().creatures() {
        let path = format!("/creatures/{}", creature.slug());
        assert!(catalog.contains(&format!("href=\"{path}\"")));
        let (status, detail) = common::text(&app.router, &path).await;
        assert_eq!(status, StatusCode::OK);
        assert!(detail.contains(creature.display_name()));
        assert!(detail.contains(creature.description()));
        assert!(detail.contains("Development placeholder"));
    }

    let (status, filtered) = common::text(&app.router, "/creatures?q=%20EMBER%20").await;
    assert_eq!(status, StatusCode::OK);
    assert!(filtered.contains("Emberling"));
    assert!(!filtered.contains("Mosswhisk"));
}

#[tokio::test]
async fn empty_search_errors_and_read_only_routes() {
    let app = common::app().await;

    for path in ["/creatures?q=missing"] {
        let (status, body) = common::text(&app.router, path).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("No creatures found"));
    }

    for path in ["/missing", "/creatures/missing"] {
        let (status, body) = common::text(&app.router, path).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("Browse creatures"));
    }

    let too_long = format!("/creatures?q={}", "a".repeat(101));
    let (status, body) = common::text(&app.router, &too_long).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("Check your search"));

    let response = common::send(&app.router, common::post_form("/creatures", None, None, "")).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn escapes_fixture_text_and_reflected_search() {
    let json = BUNDLED_FIXTURES.replace("Emberling", "<script>alert(1)</script>");
    let app = common::app_with_catalog(Catalog::from_json(&json).unwrap(), None).await;

    let (_, body) = common::text(&app.router, "/creatures").await;
    assert!(!body.contains("<script>"));
    assert!(body.contains("alert(1)"));

    let (_, body) = common::text(&app.router, "/creatures?q=%22%3E%3Cscript%3E").await;
    assert!(!body.contains("<script>"));

    let (_, body) = common::text(&app.router, "/creatures/emberling").await;
    assert!(!body.contains("<script>"));
}
