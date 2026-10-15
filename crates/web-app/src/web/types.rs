//! Type metadata and exact, ordered assignment forms. Policy lives in Types.
use super::authoring::{
    ContentForm, ContentPage, HistoryQuery, SavedState, SourceChoice, TypeComparison,
    authoring_form_error, content_response, draft_response, error_status, error_text, finish,
    history_response, invalid, library_permission, no_store, parse_content, positive, session,
    snapshot_response, submit_token, world_permission,
};
use super::*;
use crate::application::authoring::{Error, HistoryKind};
use game_data::{
    authoring::ContentFields,
    types::{TypePin, TypeRecord, TypingState},
};
use std::collections::{BTreeMap, BTreeSet};

const FORM_LIMIT: usize = 32 * 1024;
const MAX_SLOTS: usize = 32;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/types", get(catalog))
        .route("/types/new", get(type_new).post(type_create))
        .route("/types/{id}", get(type_detail))
        .route("/types/{id}/edit", get(type_edit).post(type_save))
        .route("/types/{id}/history", get(type_history))
        .route("/types/{id}/history/{revision}", get(type_snapshot))
        .route(
            "/worlds/{world_id}/types/new",
            get(world_type_new).post(world_type_create),
        )
        .route("/worlds/{world_id}/types/{id}", get(world_type_detail))
        .route(
            "/worlds/{world_id}/types/{id}/edit",
            get(world_type_edit).post(world_type_save),
        )
        .route(
            "/worlds/{world_id}/types/{id}/adopt",
            get(adoption_form).post(adoption_save),
        )
        .route(
            "/worlds/{world_id}/types/{id}/history",
            get(world_type_history),
        )
        .route(
            "/worlds/{world_id}/types/{id}/history/{revision}",
            get(world_type_snapshot),
        )
        .route(
            "/forms/{id}/typing",
            get(form_typing).post(form_typing_save),
        )
        .route(
            "/worlds/{world_id}/variants/{id}/typing",
            get(variant_typing).post(variant_typing_save),
        )
        .layer(DefaultBodyLimit::max(FORM_LIMIT))
}

pub(super) struct TypingSummary {
    pub(super) modeled: bool,
    pub(super) entries: Vec<TypingEntry>,
}

pub(super) struct TypingEntry {
    pub(super) name: String,
    pub(super) pin: TypePin,
    pub(super) url: String,
    pub(super) version_url: String,
}

impl TypingSummary {
    pub(super) fn new(state: TypingState, world_id: Option<&str>) -> Self {
        Self {
            modeled: state.assignments.is_some(),
            entries: state
                .assignments
                .unwrap_or_default()
                .into_iter()
                .map(|entry| {
                    let url = match world_id {
                        Some(world) => format!("/worlds/{world}/types/{}", entry.pin.type_id),
                        None => format!("/types/{}", entry.pin.type_id),
                    };
                    TypingEntry {
                        name: entry.fields.display_name,
                        version_url: format!("{url}/history/{}", entry.pin.version),
                        pin: entry.pin,
                        url,
                    }
                })
                .collect(),
        }
    }
}

#[derive(Template)]
#[template(path = "types.html")]
struct CatalogPage {
    types: Vec<TypeRecord>,
    can_edit_library: bool,
}

async fn catalog(State(state): State<AppState>, headers: HeaderMap) -> Response {
    finish(
        async {
            let caps = state
                .authoring
                .capabilities(session(&headers).as_deref(), None)
                .await?;
            Ok(render(CatalogPage {
                types: state.types.types().await?,
                can_edit_library: caps.can_edit_library,
            }))
        }
        .await,
    )
}

#[derive(Template)]
#[template(path = "type_detail.html")]
struct DetailPage {
    id: String,
    fields: ContentFields,
    revision: i64,
    detail_url: String,
    world: Option<WorldRecord>,
    worlds: Vec<WorldUse>,
    source: Option<TypeRecord>,
    latest: Option<TypeRecord>,
    can_edit: bool,
}

pub(super) struct WorldUse {
    pub(super) world: WorldRecord,
    pub(super) can_edit: bool,
}

pub(super) async fn world_uses(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Vec<WorldUse>, Error> {
    let token = session(headers);
    let mut worlds = Vec::new();
    for world in state.authoring.worlds().await? {
        let caps = state
            .authoring
            .capabilities(token.as_deref(), Some(&world.id))
            .await?;
        worlds.push(WorldUse {
            world,
            can_edit: caps.can_edit_world,
        });
    }
    Ok(worlds)
}

async fn type_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    finish(
        async {
            let entry = state.types.type_detail(&id).await?;
            let caps = state
                .authoring
                .capabilities(session(&headers).as_deref(), None)
                .await?;
            Ok(render(DetailPage {
                detail_url: format!("/types/{}", addresses::segment(&entry.fields.slug)),
                id,
                fields: entry.fields,
                revision: entry.revision,
                world: None,
                worlds: world_uses(&state, &headers).await?,
                source: None,
                latest: None,
                can_edit: caps.can_edit_library,
            }))
        }
        .await,
    )
}

async fn world_type_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world_id, id)): Path<(String, String)>,
) -> Response {
    finish(
        async {
            let entry = state.types.world_type_detail(&world_id, &id).await?;
            let world = state.authoring.world_detail(&world_id).await?;
            let (source, latest) = match entry.source {
                Some(pin) => (
                    Some(state.types.type_version(&pin.type_id, pin.version).await?),
                    Some(state.types.type_detail(&pin.type_id).await?),
                ),
                None => (None, None),
            };
            let caps = state
                .authoring
                .capabilities(session(&headers).as_deref(), Some(&world_id))
                .await?;
            Ok(render(DetailPage {
                detail_url: format!(
                    "/worlds/{}/types/{}",
                    addresses::segment(&world.fields.slug),
                    addresses::segment(&entry.fields.slug)
                ),
                id,
                fields: entry.fields,
                revision: entry.revision,
                world: Some(world),
                worlds: Vec::new(),
                source,
                latest,
                can_edit: caps.can_edit_world,
            }))
        }
        .await,
    )
}

fn metadata_page(world_id: Option<&str>, id: Option<&str>, draft: ContentForm) -> ContentPage {
    let base = match world_id {
        Some(world) => format!("/worlds/{world}/types"),
        None => "/types".into(),
    };
    let cancel = match (world_id, id) {
        (_, Some(id)) => format!("{base}/{id}"),
        (Some(world), None) => format!("/worlds/{world}"),
        _ => base.clone(),
    };
    let title = match (world_id, id) {
        (None, None) => "Create shared type",
        (None, Some(_)) => "Edit shared type",
        (Some(_), None) => "Add world type",
        (Some(_), Some(_)) => "Edit world type",
    };
    let action = match id {
        Some(id) => format!("{base}/{id}/edit"),
        None => format!("{base}/new"),
    };
    let mut page = ContentPage::new(title, action, cancel, draft);
    page.context = "Type names and descriptions are metadata. Exact assignment pins remain unchanged by metadata edits. Slugs are generated automatically.".into();
    if let Some(world) = world_id {
        page.context
            .push_str(&format!(" Canonical world ID: {world}."));
    }
    if let Some(id) = id {
        page.context.push_str(&format!(" Canonical type ID: {id}."));
    }
    if world_id.is_some() && id.is_none() {
        page.choose_source = true;
        page.source_label = "Source shared type and version";
        page.source_help = "Choose an exact shared type version or world-exclusive content. The name and description below remain explicit local values. Use a shared type's world link to start with a copy; selecting an option does not copy or merge fields.";
    }
    page.target_label = "type";
    if id.is_none() {
        page.button = title.into();
    }
    page
}

fn parse_pin(value: &str, field: &str) -> Result<TypePin, Error> {
    let (id, version) = value
        .rsplit_once('@')
        .ok_or_else(|| invalid(field, "Choose a type ID and exact positive version."))?;
    if id.is_empty() {
        return Err(invalid(field, "A type ID is required."));
    }
    Ok(TypePin {
        type_id: id.to_owned(),
        version: positive(version, field)?,
    })
}

fn source_pin(draft: &ContentForm) -> Result<Option<TypePin>, Error> {
    let value = draft.source_value();
    if value.is_empty() {
        Ok(None)
    } else {
        parse_pin(&value, "source_pin").map(Some)
    }
}

async fn shared_choices(state: &AppState, retained: &[String]) -> Result<Vec<SourceChoice>, Error> {
    let mut choices: Vec<_> = state
        .types
        .types()
        .await?
        .into_iter()
        .map(|entry| SourceChoice {
            value: format!("{}@{}", entry.id, entry.revision),
            label: format!(
                "{} · version {} · {}",
                entry.fields.display_name, entry.revision, entry.id
            ),
        })
        .collect();
    for value in retained {
        if value.is_empty() || choices.iter().any(|choice| &choice.value == value) {
            continue;
        }
        if let Ok(pin) = parse_pin(value, "source_pin")
            && let Ok(entry) = state.types.type_version(&pin.type_id, pin.version).await
        {
            choices.push(SourceChoice {
                value: value.clone(),
                label: format!(
                    "{} · historical version {} · {}",
                    entry.fields.display_name, entry.revision, entry.id
                ),
            });
        }
    }
    retain_choices(&mut choices, retained);
    Ok(choices)
}

fn retain_choices(choices: &mut Vec<SourceChoice>, retained: &[String]) {
    for value in retained {
        if !value.is_empty() && !choices.iter().any(|choice| &choice.value == value) {
            choices.push(SourceChoice {
                value: value.clone(),
                label: format!("Retained historical/submitted exact pin: {value}"),
            });
        }
    }
}

async fn type_new(State(state): State<AppState>, headers: HeaderMap) -> Response {
    no_store(finish(
        async {
            library_permission(&state, &headers).await?;
            Ok(content_response(
                &state,
                &headers,
                metadata_page(None, None, ContentForm::default()),
                StatusCode::OK,
            ))
        }
        .await,
    ))
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct SourceQuery {
    type_id: String,
    type_version: String,
}

async fn world_type_new(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(world_id): Path<String>,
    query: Result<Query<SourceQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    no_store(finish(
        async {
            world_permission(&state, &headers, Some(&world_id)).await?;
            let Query(query) = query
                .map_err(|_| invalid("source_pin", "Check the source ID and exact version."))?;
            let world = state.authoring.world_detail(&world_id).await?;
            let mut draft = ContentForm::for_world_revision(world.revision);
            if !query.type_id.is_empty() || !query.type_version.is_empty() {
                let version = positive(&query.type_version, "source_pin")?;
                let source = state.types.type_version(&query.type_id, version).await?;
                draft.source_pin = Some(format!("{}@{version}", source.id));
                draft.display_name = source.fields.display_name;
                draft.description = source.fields.description;
            }
            let mut page = metadata_page(Some(&world_id), None, draft);
            page.choices =
                shared_choices(&state, std::slice::from_ref(&page.selected_source)).await?;
            Ok(content_response(&state, &headers, page, StatusCode::OK))
        }
        .await,
    ))
}

async fn type_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    metadata_submit(state, headers, None, None, form).await
}
async fn world_type_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(world_id): Path<String>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    metadata_submit(state, headers, Some(world_id), None, form).await
}
async fn type_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    metadata_submit(state, headers, None, Some(id), form).await
}
async fn world_type_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world_id, id)): Path<(String, String)>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    metadata_submit(state, headers, Some(world_id), Some(id), form).await
}

async fn metadata_submit(
    state: AppState,
    headers: HeaderMap,
    world_id: Option<String>,
    id: Option<String>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(draft) => draft,
        Err(error) => return authoring_form_error(error),
    };
    let result: Result<String, Error> = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        match (world_id.as_deref(), id.as_deref()) {
            (None, None) => Ok(format!(
                "/types/{}",
                state
                    .types
                    .create_type(&token, draft.fields(), draft.reason())
                    .await?
                    .id
            )),
            (None, Some(id)) => {
                state.types.save_type(&token, id, draft.edit()?).await?;
                Ok(format!("/types/{id}"))
            }
            (Some(world), None) => Ok(format!(
                "/worlds/{world}/types/{}",
                state
                    .types
                    .create_world_type(
                        &token,
                        world,
                        draft.world_revision()?,
                        source_pin(&draft)?,
                        draft.fields(),
                        draft.reason()
                    )
                    .await?
                    .id
            )),
            (Some(world), Some(id)) => {
                state
                    .types
                    .save_world_type(&token, world, id, draft.edit()?)
                    .await?;
                Ok(format!("/worlds/{world}/types/{id}"))
            }
        }
    }
    .await;
    match result {
        Ok(url) => no_store(Redirect::to(&url).into_response()),
        Err(error) => {
            let mut page = metadata_page(world_id.as_deref(), id.as_deref(), draft);
            if page.choose_source {
                page.choices = shared_choices(&state, std::slice::from_ref(&page.selected_source))
                    .await
                    .unwrap_or_else(|_| {
                        vec![SourceChoice {
                            value: page.selected_source.clone(),
                            label: format!("Retained submitted pin: {}", page.selected_source),
                        }]
                    });
            }
            if matches!(error, Error::Conflict) {
                let current = match (world_id.as_deref(), id.as_deref()) {
                    (None, Some(id)) => state
                        .types
                        .type_detail(id)
                        .await
                        .map(|entry| (entry.fields, entry.revision)),
                    (Some(world), Some(id)) => state
                        .types
                        .world_type_detail(world, id)
                        .await
                        .map(|entry| (entry.fields, entry.revision)),
                    (Some(world), None) => state
                        .authoring
                        .world_detail(world)
                        .await
                        .map(|entry| (entry.fields, entry.revision)),
                    _ => Err(Error::Unavailable),
                };
                match current {
                    Ok((fields, revision)) => page.conflict(
                        fields,
                        revision,
                        "Current saved metadata and revision. Review before resubmitting.".into(),
                    ),
                    Err(load_error) => return draft_response(&state, &headers, page, load_error),
                }
            }
            draft_response(&state, &headers, page, error)
        }
    }
}

async fn type_edit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    no_store(finish(
        async {
            library_permission(&state, &headers).await?;
            let entry = state.types.type_detail(&id).await?;
            Ok(content_response(
                &state,
                &headers,
                metadata_page(
                    None,
                    Some(&id),
                    ContentForm::with_fields(&entry.fields, entry.revision),
                ),
                StatusCode::OK,
            ))
        }
        .await,
    ))
}
async fn world_type_edit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world_id, id)): Path<(String, String)>,
) -> Response {
    no_store(finish(
        async {
            world_permission(&state, &headers, Some(&world_id)).await?;
            let entry = state.types.world_type_detail(&world_id, &id).await?;
            Ok(content_response(
                &state,
                &headers,
                metadata_page(
                    Some(&world_id),
                    Some(&id),
                    ContentForm::with_fields(&entry.fields, entry.revision),
                ),
                StatusCode::OK,
            ))
        }
        .await,
    ))
}

#[derive(Deserialize)]
struct AdoptionQuery {
    target_version: String,
}

fn adoption_page(world: &str, id: &str, draft: ContentForm) -> ContentPage {
    let mut page = ContentPage::new(
        "Review type source adoption",
        format!("/worlds/{world}/types/{id}/adopt"),
        format!("/worlds/{world}/types/{id}"),
        draft,
    );
    page.target_label = "type";
    page.button = format!(
        "Adopt version {} with reviewed values",
        page.draft.target_version
    );
    page.context = "Review old shared source, current local values, and new shared source. Choose local fields explicitly; adoption does not merge fields or retarget existing variant type pins.".into();
    page
}

async fn adoption_comparisons(
    state: &AppState,
    world: &str,
    id: &str,
    target: i64,
    page: &mut ContentPage,
) -> Result<(), Error> {
    let preview = state.types.adoption_preview(world, id, target).await?;
    page.type_comparisons = vec![
        TypeComparison {
            label: "Old shared source",
            entry: preview.old_source,
        },
        TypeComparison {
            label: "New shared source",
            entry: preview.new_source,
        },
    ];
    page.current = Some(SavedState {
        fields: preview.entry.fields,
        revision: preview.entry.revision,
        context: format!(
            "Current local effective values. Source pin: {}",
            preview
                .entry
                .source
                .map(|pin| format!("{}@{}", pin.type_id, pin.version))
                .unwrap_or_default()
        ),
    });
    Ok(())
}

async fn adoption_form(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world, id)): Path<(String, String)>,
    query: Result<Query<AdoptionQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    no_store(finish(
        async {
            world_permission(&state, &headers, Some(&world)).await?;
            let Query(query) = query.map_err(|_| {
                invalid(
                    "target_version",
                    "Choose an explicit newer version of the same shared type.",
                )
            })?;
            let target = positive(&query.target_version, "target_version")?;
            let entry = state.types.world_type_detail(&world, &id).await?;
            let mut draft = ContentForm::with_fields(&entry.fields, entry.revision);
            draft.target_version = query.target_version;
            let mut page = adoption_page(&world, &id, draft);
            adoption_comparisons(&state, &world, &id, target, &mut page).await?;
            Ok(content_response(&state, &headers, page, StatusCode::OK))
        }
        .await,
    ))
}

async fn adoption_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world, id)): Path<(String, String)>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(draft) => draft,
        Err(error) => return authoring_form_error(error),
    };
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        state
            .types
            .adopt_world_type(
                &token,
                &world,
                &id,
                positive(&draft.target_version, "target_version")?,
                draft.edit()?,
            )
            .await
    }
    .await;
    match result {
        Ok(_) => no_store(Redirect::to(&format!("/worlds/{world}/types/{id}")).into_response()),
        Err(error) => {
            let mut page = adoption_page(&world, &id, draft);
            if let Ok(target) = positive(&page.draft.target_version, "target_version") {
                let _ = adoption_comparisons(&state, &world, &id, target, &mut page).await;
                if matches!(error, Error::Conflict) {
                    match state.types.world_type_detail(&world, &id).await {
                        Ok(current) => {
                            if current
                                .source
                                .as_ref()
                                .is_some_and(|pin| pin.version >= target)
                            {
                                page.blocked_reason = "Another editor has already adopted this source version or a later one. Your draft is retained; return to the type to review a newer target or edit local values.".into();
                            }
                            let context = format!(
                                "Current source pin: {}",
                                current
                                    .source
                                    .map(|pin| format!("{}@{}", pin.type_id, pin.version))
                                    .unwrap_or_default()
                            );
                            page.conflict(current.fields, current.revision, context);
                        }
                        Err(load_error) => {
                            return draft_response(&state, &headers, page, load_error);
                        }
                    }
                }
            }
            draft_response(&state, &headers, page, error)
        }
    }
}

async fn type_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
    query: Result<Query<HistoryQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    finish(
        async {
            state.types.type_detail(&id).await?;
            history_response(
                &state,
                HistoryKind::Type,
                id.clone(),
                format!("/types/{id}"),
                query,
            )
            .await
        }
        .await,
    )
}
async fn type_snapshot(
    State(state): State<AppState>,
    Path((id, revision)): Path<(String, String)>,
) -> Response {
    finish(
        async {
            state.types.type_detail(&id).await?;
            snapshot_response(
                &state,
                HistoryKind::Type,
                id.clone(),
                revision,
                format!("/types/{id}/history"),
            )
            .await
        }
        .await,
    )
}
async fn world_type_history(
    State(state): State<AppState>,
    Path((world, id)): Path<(String, String)>,
    query: Result<Query<HistoryQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    finish(
        async {
            state.types.world_type_detail(&world, &id).await?;
            history_response(
                &state,
                HistoryKind::WorldType,
                id.clone(),
                format!("/worlds/{world}/types/{id}"),
                query,
            )
            .await
        }
        .await,
    )
}
async fn world_type_snapshot(
    State(state): State<AppState>,
    Path((world, id, revision)): Path<(String, String, String)>,
) -> Response {
    finish(
        async {
            state.types.world_type_detail(&world, &id).await?;
            snapshot_response(
                &state,
                HistoryKind::WorldType,
                id.clone(),
                revision,
                format!("/worlds/{world}/types/{id}/history"),
            )
            .await
        }
        .await,
    )
}

// Dynamic native dropdowns; the only values sent to the service are exact pins.
struct TypingDraft {
    csrf: String,
    expected_revision: String,
    mode: String,
    reason: String,
    slots: Vec<String>,
    slots_input: String,
    extra_slots: BTreeMap<String, String>,
}

impl TypingDraft {
    fn saved(state: &TypingState, revision: i64, slots: usize) -> Self {
        let mut values: Vec<_> = state
            .assignments
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|entry| format!("{}@{}", entry.pin.type_id, entry.pin.version))
            .collect();
        values.resize(slots.max(values.len()), String::new());
        Self {
            csrf: String::new(),
            expected_revision: revision.to_string(),
            mode: if state.assignments.is_some() {
                "defined"
            } else {
                "unmodeled"
            }
            .into(),
            reason: String::new(),
            slots: values,
            slots_input: slots
                .max(state.assignments.as_ref().map_or(0, Vec::len))
                .to_string(),
            extra_slots: BTreeMap::new(),
        }
    }

    fn submitted(mut values: BTreeMap<String, String>) -> (Self, Option<Error>) {
        // Slot count is a software work bound; it has no gameplay meaning.
        let slots_input = values.remove("slots").unwrap_or_else(|| "3".into());
        let parsed_count = slot_count(&slots_input);
        let count = parsed_count.as_ref().copied().unwrap_or(MAX_SLOTS);
        let mut error = parsed_count.err();
        let reviewed = values.remove("reviewed_revision").unwrap_or_default();
        let expected = values.remove("expected_revision").unwrap_or_default();
        let slots = (1..=count)
            .map(|index| values.remove(&format!("slot_{index}")).unwrap_or_default())
            .collect();
        let extra_slots: BTreeMap<_, _> = values
            .iter()
            .filter(|(key, _)| key.starts_with("slot_"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        if !extra_slots.is_empty() {
            error = Some(invalid(
                "slots",
                "Use only the displayed, numbered slots (at most 32). Extra submitted values are retained below. Use Show slots to reload a valid editor before editing again.",
            ));
        }
        (
            Self {
                csrf: values.remove("csrf").unwrap_or_default(),
                expected_revision: if reviewed.is_empty() {
                    expected
                } else {
                    reviewed
                },
                mode: values.remove("mode").unwrap_or_default(),
                reason: values.remove("reason").unwrap_or_default(),
                slots,
                slots_input,
                extra_slots,
            },
            error,
        )
    }

    fn pins(&self) -> Result<Option<Vec<TypePin>>, Error> {
        if self.mode == "unmodeled" {
            return Ok(None);
        }
        if self.mode != "defined" {
            return Err(invalid("mode", "Choose Unmodeled or Defined."));
        }
        let mut ids = BTreeSet::new();
        let mut pins = Vec::new();
        for (index, value) in self.slots.iter().enumerate() {
            if value.is_empty() {
                continue;
            }
            let field = format!("slot_{}", index + 1);
            let pin = parse_pin(value, &field)?;
            if !ids.insert(pin.type_id.clone()) {
                return Err(invalid(
                    &field,
                    "This type identity was already selected. Choose each type only once, even across versions.",
                ));
            }
            pins.push(pin);
        }
        Ok(Some(pins))
    }
}

fn slot_count(value: &str) -> Result<usize, Error> {
    value.parse::<usize>().ok().filter(|count| (1..=MAX_SLOTS).contains(count)).ok_or_else(|| invalid("slots", "Choose from 1 to 32 displayed slots. This is a software work bound, not a game rule."))
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct SlotsQuery {
    slots: String,
}

struct Slot {
    index: usize,
    value: String,
    error: String,
}

#[derive(Template)]
#[template(path = "typing_form.html")]
struct TypingPage {
    action: String,
    cancel: String,
    name: String,
    draft: TypingDraft,
    slots: Vec<Slot>,
    choices: Vec<SourceChoice>,
    error: String,
    errors: BTreeMap<String, String>,
    reviewed_revision: String,
    current: Option<TypingSummary>,
    current_name: String,
    local: bool,
}

impl TypingPage {
    fn field_error(&self, field: &str) -> &str {
        self.errors.get(field).map(String::as_str).unwrap_or("")
    }
}

async fn typing_current(
    state: &AppState,
    world: Option<&str>,
    id: &str,
) -> Result<(String, i64, TypingState), Error> {
    match world {
        None => {
            let entry = state.authoring.form_detail(id).await?;
            Ok((
                entry.fields.display_name,
                entry.revision,
                state.types.form_typing(id, None).await?,
            ))
        }
        Some(world) => {
            let entry = state.authoring.variant_detail(world, id).await?;
            Ok((
                entry.fields.display_name,
                entry.revision,
                state.types.variant_typing(world, id, None).await?,
            ))
        }
    }
}

async fn typing_choices(
    state: &AppState,
    world: Option<&str>,
    current: &TypingState,
    retained: &[String],
) -> Result<Vec<SourceChoice>, Error> {
    let mut choices = if let Some(world) = world {
        state
            .types
            .world_types(world)
            .await?
            .into_iter()
            .map(|entry| SourceChoice {
                value: format!("{}@{}", entry.id, entry.revision),
                label: format!(
                    "{} · local version {} · {}",
                    entry.fields.display_name, entry.revision, entry.id
                ),
            })
            .collect()
    } else {
        shared_choices(state, &[]).await?
    };
    for entry in current.assignments.as_deref().unwrap_or_default() {
        let value = format!("{}@{}", entry.pin.type_id, entry.pin.version);
        if !choices.iter().any(|choice| choice.value == value) {
            choices.push(SourceChoice {
                value,
                label: format!(
                    "{} · historical version {} · {}",
                    entry.fields.display_name, entry.pin.version, entry.pin.type_id
                ),
            });
        }
    }
    retain_choices(&mut choices, retained);
    Ok(choices)
}

fn typing_page(world: Option<&str>, id: &str, name: String, draft: TypingDraft) -> TypingPage {
    let cancel = match world {
        Some(world) => format!("/worlds/{world}/variants/{id}"),
        None => format!("/forms/{id}"),
    };
    TypingPage {
        action: format!("{cancel}/typing"),
        cancel,
        name,
        draft,
        slots: Vec::new(),
        choices: Vec::new(),
        error: String::new(),
        errors: BTreeMap::new(),
        reviewed_revision: String::new(),
        current: None,
        current_name: String::new(),
        local: world.is_some(),
    }
}

fn typing_response(
    state: &AppState,
    headers: &HeaderMap,
    mut page: TypingPage,
    status: StatusCode,
) -> Response {
    page.slots = page
        .draft
        .slots
        .iter()
        .enumerate()
        .map(|(index, value)| Slot {
            index: index + 1,
            value: value.clone(),
            error: page.field_error(&format!("slot_{}", index + 1)).to_owned(),
        })
        .collect();
    let (csrf, set) = match csrf_for(headers) {
        Ok(value) => value,
        Err(_) => {
            page.error = error_text(&Error::Unavailable).into();
            return no_store(render_status(page, StatusCode::SERVICE_UNAVAILABLE));
        }
    };
    page.draft.csrf = csrf.clone();
    let mut response = no_store(render_status(page, status));
    if set {
        append_cookie(&mut response, csrf_cookie(&csrf, state.cookie_secure));
    }
    response
}

async fn form_typing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    query: Result<Query<SlotsQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    typing_get(state, headers, None, id, query).await
}
async fn variant_typing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world, id)): Path<(String, String)>,
    query: Result<Query<SlotsQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    typing_get(state, headers, Some(world), id, query).await
}

async fn typing_get(
    state: AppState,
    headers: HeaderMap,
    world: Option<String>,
    id: String,
    query: Result<Query<SlotsQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    no_store(finish(
        async {
            match world.as_deref() {
                None => library_permission(&state, &headers).await?,
                Some(world) => world_permission(&state, &headers, Some(world)).await?,
            }
            let Query(query) = query
                .map_err(|_| invalid("slots", "Choose a displayed slot count from 1 to 32."))?;
            let (name, revision, current) = typing_current(&state, world.as_deref(), &id).await?;
            let count = if query.slots.is_empty() {
                (current.assignments.as_ref().map_or(0, Vec::len) + 1).clamp(3, MAX_SLOTS)
            } else {
                slot_count(&query.slots)?
            };
            let draft = TypingDraft::saved(&current, revision, count);
            let mut page = typing_page(world.as_deref(), &id, name, draft);
            page.choices =
                typing_choices(&state, world.as_deref(), &current, &page.draft.slots).await?;
            Ok(typing_response(&state, &headers, page, StatusCode::OK))
        }
        .await,
    ))
}

async fn form_typing_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    form: Result<Form<BTreeMap<String, String>>, FormRejection>,
) -> Response {
    typing_submit(state, headers, None, id, form).await
}
async fn variant_typing_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world, id)): Path<(String, String)>,
    form: Result<Form<BTreeMap<String, String>>, FormRejection>,
) -> Response {
    typing_submit(state, headers, Some(world), id, form).await
}

async fn typing_submit(
    state: AppState,
    headers: HeaderMap,
    world: Option<String>,
    id: String,
    form: Result<Form<BTreeMap<String, String>>, FormRejection>,
) -> Response {
    let values = match form {
        Ok(Form(values)) => values,
        Err(error) => return authoring_form_error(error),
    };
    let (draft, parse_error) = TypingDraft::submitted(values);
    let result: Result<(), Error> = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        if let Some(error) = parse_error {
            return Err(error);
        }
        let revision = positive(&draft.expected_revision, "expected_revision")?;
        let pins = draft.pins()?;
        let reason = if draft.reason.is_empty() {
            None
        } else {
            Some(draft.reason.as_str())
        };
        match world.as_deref() {
            None => {
                state
                    .types
                    .save_form_typing(&token, &id, revision, pins, reason)
                    .await?;
            }
            Some(world) => {
                state
                    .types
                    .save_variant_typing(&token, world, &id, revision, pins, reason)
                    .await?;
            }
        }
        Ok(())
    }
    .await;
    let mut page = typing_page(world.as_deref(), &id, "Ordered typing".into(), draft);
    match result {
        Ok(()) => no_store(Redirect::to(&page.cancel).into_response()),
        Err(error) => {
            page.error = error_text(&error).into();
            if let Error::Invalid(errors) = &error {
                page.errors = errors.clone();
            }
            match typing_current(&state, world.as_deref(), &id).await {
                Ok((name, revision, current)) => {
                    page.name = name.clone();
                    page.choices =
                        typing_choices(&state, world.as_deref(), &current, &page.draft.slots)
                            .await
                            .unwrap_or_default();
                    if matches!(error, Error::Conflict) {
                        page.reviewed_revision = revision.to_string();
                        page.current_name = name;
                        page.current = Some(TypingSummary::new(current, world.as_deref()));
                    }
                }
                Err(_) if matches!(error, Error::Conflict) => {
                    page.error = format!(
                        "{} The current saved state could not be loaded. Keep this draft and reload the saved record before reviewing.",
                        error_text(&Error::Unavailable)
                    );
                    retain_choices(&mut page.choices, &page.draft.slots);
                    return typing_response(
                        &state,
                        &headers,
                        page,
                        StatusCode::SERVICE_UNAVAILABLE,
                    );
                }
                Err(_) => {}
            }
            retain_choices(&mut page.choices, &page.draft.slots);
            typing_response(&state, &headers, page, error_status(&error))
        }
    }
}
