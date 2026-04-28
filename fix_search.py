import sys

def process():
    with open('crates/backend/src/search.rs', 'r') as f:
        content = f.read()

    # Change search_page signature
    content = content.replace(
        "pub async fn search_page(session: Session, State(pool): State<SqlitePool>, Query(params): Query<SearchParams>) -> impl IntoResponse {",
        "pub async fn search_page(session: Session, State(pool): State<SqlitePool>, Query(params): Query<SearchParams>) -> Result<impl IntoResponse, crate::error::AppError> {"
    )

    # Change search_suggestions signature
    content = content.replace(
        "pub async fn search_suggestions(State(pool): State<SqlitePool>, Query(params): Query<SuggestParams>) -> impl IntoResponse {",
        "pub async fn search_suggestions(State(pool): State<SqlitePool>, Query(params): Query<SuggestParams>) -> Result<impl IntoResponse, crate::error::AppError> {"
    )
    
    # Change ends to Ok(...)
    content = content.replace(
        "render(SearchResultsTemplate {",
        "Ok(render(SearchResultsTemplate {"
    )
    
    # There is an into_response() probably for search_suggestions JSON
    content = content.replace(
        "Json(suggestions).into_response()",
        "Ok(Json(suggestions).into_response())"
    )

    # Replace .unwrap_or_default() on fetch_all with ?
    content = content.replace(".fetch_all(&pool).await.unwrap_or_default()", ".fetch_all(&pool).await?")

    # Add Default to SearchResult if it doesn't have it
    if "#[derive(Debug, Clone, Default)]" not in content:
        content = content.replace("#[derive(Debug, Clone)]\nstruct SearchResult {", "#[derive(Debug, Clone, Default)]\nstruct SearchResult {")

    with open('crates/backend/src/search.rs', 'w') as f:
        f.write(content)

process()
