use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use game_data::{BUNDLED_FIXTURES, Catalog};
use tower::ServiceExt;

async fn get(path: &str, json: &str) -> (StatusCode, String) {
    let response = web_app::router(Catalog::from_json(json).unwrap())
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 100_000).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn browse_search_and_follow_detail() {
    let (status, home) = get("/", BUNDLED_FIXTURES).await;
    assert_eq!(status, StatusCode::OK);
    assert!(home.contains("Karta Yaxu"));
    let (status, catalog) = get("/creatures", BUNDLED_FIXTURES).await;
    assert_eq!(status, StatusCode::OK);
    for creature in Catalog::from_json(BUNDLED_FIXTURES).unwrap().creatures() {
        let path = format!("/creatures/{}", creature.id());
        assert!(catalog.contains(&format!("href=\"{path}\"")));
        let (status, detail) = get(&path, BUNDLED_FIXTURES).await;
        assert_eq!(status, StatusCode::OK);
        assert!(detail.contains(creature.display_name()));
        assert!(detail.contains(creature.description()));
        assert!(detail.contains("Development placeholder"));
    }
    let (status, filtered) = get("/creatures?q=%20EMBER%20", BUNDLED_FIXTURES).await;
    assert_eq!(status, StatusCode::OK);
    assert!(filtered.contains("Emberling"));
    assert!(!filtered.contains("Mosswhisk"));
}

#[tokio::test]
async fn empty_search_errors_and_read_only_routes() {
    for (path, json) in [
        ("/creatures?q=missing", BUNDLED_FIXTURES),
        ("/creatures", "[]"),
    ] {
        let (status, body) = get(path, json).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("No creatures found"));
    }
    for path in ["/missing", "/creatures/missing"] {
        let (status, body) = get(path, BUNDLED_FIXTURES).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("Browse creatures"));
    }
    let (status, body) = get(
        &format!("/creatures?q={}", "a".repeat(101)),
        BUNDLED_FIXTURES,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("Check your search"));
    let response = web_app::router(Catalog::from_json(BUNDLED_FIXTURES).unwrap())
        .oneshot(Request::post("/creatures").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn escapes_fixture_text_and_reflected_search() {
    let json = BUNDLED_FIXTURES.replace("Emberling", "<script>alert(1)</script>");
    let (_, body) = get("/creatures", &json).await;
    assert!(!body.contains("<script>"));
    assert!(body.contains("alert(1)"));
    let (_, body) = get("/creatures?q=%22%3E%3Cscript%3E", BUNDLED_FIXTURES).await;
    assert!(!body.contains("<script>"));
    let (_, body) = get("/creatures/6a64df7c88604b3f9381536710449a8b", &json).await;
    assert!(!body.contains("<script>"));
}
