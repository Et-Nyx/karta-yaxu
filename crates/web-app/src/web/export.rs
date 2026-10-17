//! Native, authorized preview and confirmed attachment. No public package GET.
use super::authoring::{error_status, invalid, no_store, session, submit_token};
use super::*;
use crate::application::{
    authoring::{AddressKey, Error},
    export::{Confirmation, ExportResult},
};
use game_data::inspection::InspectionPayload;
use std::collections::BTreeMap;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/worlds/{world_id}/export", get(entry).post(submit))
        .layer(DefaultBodyLimit::max(4096))
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ExportForm {
    csrf: String,
    intent: String,
    reviewed_digest: String,
    confirm: String,
}

struct Item {
    kind: &'static str,
    id: String,
    revision: i64,
    label: String,
    state: &'static str,
}
struct Finding {
    category: String,
    subject: String,
    message: String,
    policy: String,
}
#[derive(Template)]
#[template(path = "export.html")]
struct ExportPage {
    id: String,
    name: String,
    csrf: String,
    digest: String,
    notice: String,
    error: String,
    roots: usize,
    records: usize,
    bindings: usize,
    bytes: usize,
    counts: Vec<(String, usize)>,
    items: Vec<Item>,
    remaining_items: usize,
    findings: Vec<Finding>,
    remaining_findings: usize,
}
impl ExportPage {
    fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            csrf: String::new(),
            digest: String::new(),
            notice: String::new(),
            error: String::new(),
            roots: 0,
            records: 0,
            bindings: 0,
            bytes: 0,
            counts: vec![],
            items: vec![],
            remaining_items: 0,
            findings: vec![],
            remaining_findings: 0,
        }
    }
    fn preview(result: &ExportResult) -> Self {
        let package = &result.package;
        let name = package
            .content
            .records
            .iter()
            .find_map(|r| match &r.payload {
                InspectionPayload::World(w) => Some(w.fields.display_name.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let mut page = Self::new(package.content.world.id.clone(), name);
        page.digest = package.manifest.content_sha256.clone();
        page.roots = package.manifest.root_count;
        page.records = package.manifest.record_count;
        page.bindings = package.content.bindings.len();
        page.bytes = result.bytes.len();
        page.items = package
            .content
            .roots
            .iter()
            .take(25)
            .map(|k| {
                let record = package.content.records.iter().find(|r| &r.key == k);
                Item {
                    kind: k.kind.as_str(),
                    id: k.id.clone(),
                    revision: k.revision,
                    label: record
                        .and_then(|r| r.payload.fields())
                        .map(|f| f.display_name.clone())
                        .unwrap_or_else(|| k.kind.as_str().into()),
                    state: "Active root",
                }
            })
            .collect();
        page.remaining_items = page.roots.saturating_sub(page.items.len());
        let mut counts = BTreeMap::new();
        for finding in &result.findings {
            *counts.entry(format!("{:?}", finding.category)).or_insert(0) += 1;
        }
        page.counts = counts.into_iter().collect();
        page.findings = result
            .findings
            .iter()
            .take(25)
            .map(|f| Finding {
                category: format!("{:?}", f.category),
                subject: format!(
                    "{} {} revision {}",
                    f.subject.kind.as_str(),
                    f.subject.id,
                    f.subject.revision
                ),
                message: f.message.clone(),
                policy: f
                    .policy
                    .map(|p| format!("Validator policy {p}"))
                    .unwrap_or_default(),
            })
            .collect();
        page.remaining_findings = result.findings.len().saturating_sub(page.findings.len());
        page
    }
}
fn response(
    state: &AppState,
    headers: &HeaderMap,
    mut page: ExportPage,
    status: StatusCode,
) -> Response {
    let (csrf, set) = match csrf_for(headers) {
        Ok(value) => value,
        Err(_) => return failure(Error::Unavailable),
    };
    page.csrf = csrf.clone();
    let mut response = no_store(render_status(page, status));
    if set {
        append_cookie(&mut response, csrf_cookie(&csrf, state.cookie_secure));
    }
    response
}
fn failure(error: Error) -> Response {
    let text = match &error {
        Error::Unauthorized => "Sign in, then return to export this world. No package was released.".into(),
        Error::Forbidden => "Package export requires an active world and an enabled owner or collaborator with editor eligibility. No package was released.".into(),
        Error::Unavailable => "Export is temporarily unavailable, busy, or could not capture all required content within its limits. Retry or review Authoring inspection. No partial package was released.".into(),
        Error::Invalid(fields) => fields.values().cloned().collect::<Vec<_>>().join(" "),
        _ => "The requested world could not be loaded. No package was released.".into(),
    };
    no_store(message(
        error_status(&error),
        "Package export unavailable",
        &text,
    ))
}
async fn entry(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(world): Path<String>,
) -> Response {
    match state
        .export
        .entry(
            &session(&headers).unwrap_or_default(),
            AddressKey::Id(world),
        )
        .await
    {
        Ok(world) => response(
            &state,
            &headers,
            ExportPage::new(world.id, world.fields.display_name),
            StatusCode::OK,
        ),
        Err(error) => failure(error),
    }
}
async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(world): Path<String>,
    form: Result<Form<ExportForm>, FormRejection>,
) -> Response {
    let Form(form) = match form {
        Ok(form) => form,
        Err(error) => {
            return no_store(message(
                if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
                    StatusCode::PAYLOAD_TOO_LARGE
                } else {
                    StatusCode::UNPROCESSABLE_ENTITY
                },
                "Check export form",
                "Use each listed export field once, within the 4 KiB limit, and preview again.",
            ));
        }
    };
    let token = match submit_token(&state, &headers, &form.csrf) {
        Ok(token) => token,
        Err(error) => return failure(error),
    };
    let result = match form.intent.as_str() {
        "preview" => state
            .export
            .preview(&token, AddressKey::Id(world))
            .await
            .map(Confirmation::Changed),
        "download" if form.confirm == "yes" => {
            state
                .export
                .confirm(&token, AddressKey::Id(world), &form.reviewed_digest)
                .await
        }
        "download" => state
            .export
            .preview(&token, AddressKey::Id(world))
            .await
            .map(Confirmation::Changed),
        _ => Err(invalid(
            "intent",
            "Choose Preview package or Confirm and download.",
        )),
    };
    match result {
        Ok(Confirmation::Download(mut result)) => {
            let filename = format!(
                "attachment; filename=\"karta-{}-{}.json\"",
                result.package.content.world.id,
                &result.package.manifest.content_sha256[..12]
            );
            let mut response = std::mem::take(&mut result.bytes).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response.headers_mut().insert(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&filename).expect("canonical filename"),
            );
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, no-store"),
            );
            response.headers_mut().insert(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );
            response
        }
        Ok(Confirmation::Changed(result)) => {
            let mut page = ExportPage::preview(&result);
            if form.intent == "download" {
                page.notice="Package inputs changed or confirmation was missing. Review this fresh preview and confirm again; no download was released.".into();
            }
            response(&state, &headers, page, StatusCode::OK)
        }
        Err(error) => failure(error),
    }
}
