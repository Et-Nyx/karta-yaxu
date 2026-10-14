//! Native authoring forms. Policy and transactional authorization live in the service.
use super::*;
use crate::application::authoring::{
    AccessState, Edit, Error, HistoryEntry, HistoryKind, SourcePin, VariantRecord,
};
use game_data::authoring::ContentFields;
use std::collections::BTreeMap;

const FORM_LIMIT: usize = 32 * 1024;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/creatures/new", get(species_new).post(species_create))
        .route("/creatures/{id}/edit", get(species_edit).post(species_save))
        .route("/creatures/{id}/forms/new", get(form_new).post(form_create))
        .route("/forms/{id}", get(form_detail))
        .route("/forms/{id}/edit", get(form_edit).post(form_save))
        .route("/worlds", get(worlds))
        .route("/worlds/new", get(world_new).post(world_create))
        .route("/worlds/{id}", get(world_detail))
        .route("/worlds/{id}/edit", get(world_edit).post(world_save))
        .route(
            "/worlds/{world_id}/variants/new",
            get(variant_new).post(variant_create),
        )
        .route("/worlds/{world_id}/variants/{id}", get(variant_detail))
        .route(
            "/worlds/{world_id}/variants/{id}/edit",
            get(variant_edit).post(variant_save),
        )
        .route(
            "/worlds/{world_id}/variants/{id}/adopt",
            get(adoption_form).post(adoption_save),
        )
        .route("/worlds/{id}/access", get(access))
        .route("/worlds/{id}/access/grant", post(access_grant))
        .route("/worlds/{id}/access/revoke", post(access_revoke))
        .route("/creatures/{id}/history", get(species_history))
        .route("/creatures/{id}/history/{revision}", get(species_snapshot))
        .route("/forms/{id}/history", get(form_history))
        .route("/forms/{id}/history/{revision}", get(form_snapshot))
        .route("/worlds/{id}/history", get(world_history))
        .route("/worlds/{id}/history/{revision}", get(world_snapshot))
        .route(
            "/worlds/{world_id}/variants/{id}/history",
            get(variant_history),
        )
        .route(
            "/worlds/{world_id}/variants/{id}/history/{revision}",
            get(variant_snapshot),
        )
        .layer(DefaultBodyLimit::max(FORM_LIMIT))
}

pub(super) fn finish(result: Result<Response, Error>) -> Response {
    result.unwrap_or_else(read_error)
}

fn error_status(error: &Error) -> StatusCode {
    match error {
        Error::Unauthorized => StatusCode::UNAUTHORIZED,
        Error::Forbidden => StatusCode::FORBIDDEN,
        Error::NotFound => StatusCode::NOT_FOUND,
        Error::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}

fn error_text(error: &Error) -> &'static str {
    match error {
        Error::Unauthorized => {
            "Your session is no longer active. Sign in, then return to this draft to try again."
        }
        Error::Forbidden => {
            "This change is not permitted. Check your account permissions; if the form expired, reload it before trying again."
        }
        Error::NotFound => "This content or pinned version could not be found.",
        Error::Invalid(_) => {
            "Check the fields below. Your entered values have been kept; nothing was saved."
        }
        Error::Conflict => {
            "Someone changed this record. Nothing was saved. Compare the current saved state with your draft, then explicitly save reviewed changes."
        }
        Error::Unavailable => {
            "Content storage is temporarily unavailable. Keep this draft, check the saved record and history, then try again."
        }
    }
}

pub(super) fn read_error(error: Error) -> Response {
    message(
        error_status(&error),
        "Content could not be loaded",
        error_text(&error),
    )
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn session(headers: &HeaderMap) -> Option<String> {
    cookie_value(headers, SESSION_COOKIE)
}

async fn library_permission(state: &AppState, headers: &HeaderMap) -> Result<(), Error> {
    let token = session(headers);
    let caps = state.authoring.capabilities(token.as_deref(), None).await?;
    permission(caps.authenticated, caps.can_edit_library)
}

async fn world_permission(
    state: &AppState,
    headers: &HeaderMap,
    id: Option<&str>,
) -> Result<(), Error> {
    let token = session(headers);
    let caps = state.authoring.capabilities(token.as_deref(), id).await?;
    permission(
        caps.authenticated,
        if id.is_some() {
            caps.can_edit_world
        } else {
            caps.can_create_world
        },
    )
}

fn permission(authenticated: bool, allowed: bool) -> Result<(), Error> {
    if !authenticated {
        Err(Error::Unauthorized)
    } else if !allowed {
        Err(Error::Forbidden)
    } else {
        Ok(())
    }
}

fn invalid(field: &str, text: &str) -> Error {
    Error::Invalid(BTreeMap::from([(field.to_owned(), text.to_owned())]))
}

fn positive(value: &str, field: &str) -> Result<i64, Error> {
    value.parse::<i64>().ok().filter(|n| *n > 0).ok_or_else(|| {
        invalid(
            field,
            "Choose a positive, whole-number version or revision.",
        )
    })
}

// Unknown browser fields are ignored. In particular, no actor or owner field is used.
// Strings preserve malformed numeric input so field errors can return the whole draft.
#[derive(Default, Deserialize)]
#[serde(default)]
struct ContentForm {
    csrf: String,
    slug: String,
    display_name: String,
    description: String,
    reason: String,
    expected_revision: String,
    reviewed_revision: String,
    expected_world_revision: String,
    species_version: String,
    source_pin: Option<String>,
    form_id: String,
    form_version: String,
    target_version: String,
}

impl ContentForm {
    fn fields(&self) -> ContentFields {
        ContentFields {
            slug: self.slug.clone(),
            display_name: self.display_name.clone(),
            description: self.description.clone(),
        }
    }

    fn with_fields(fields: &ContentFields, revision: i64) -> Self {
        Self {
            slug: fields.slug.clone(),
            display_name: fields.display_name.clone(),
            description: fields.description.clone(),
            expected_revision: revision.to_string(),
            ..Self::default()
        }
    }

    fn reason(&self) -> Option<&str> {
        if self.reason.is_empty() {
            None
        } else {
            Some(&self.reason)
        }
    }

    fn edit(&self) -> Result<Edit, Error> {
        let revision = if self.reviewed_revision.is_empty() {
            &self.expected_revision
        } else {
            &self.reviewed_revision
        };
        Ok(Edit {
            expected_revision: positive(revision, "expected_revision")?,
            fields: self.fields(),
            reason: self.reason().map(str::to_owned),
        })
    }

    fn world_revision(&self) -> Result<i64, Error> {
        positive(
            if self.reviewed_revision.is_empty() {
                &self.expected_world_revision
            } else {
                &self.reviewed_revision
            },
            "expected_world_revision",
        )
    }

    fn source(&self) -> Result<Option<SourcePin>, Error> {
        if let Some(pin) = &self.source_pin {
            if pin.is_empty() {
                return Ok(None);
            }
            let (id, version) = pin.rsplit_once('@').ok_or_else(|| {
                invalid(
                    "source_pin",
                    "Choose a concrete form and version, or world-exclusive content.",
                )
            })?;
            if id.is_empty() {
                return Err(invalid("source_pin", "A source form ID is required."));
            }
            return Ok(Some(SourcePin {
                form_id: id.to_owned(),
                version: positive(version, "source_pin")?,
            }));
        }
        match (self.form_id.is_empty(), self.form_version.is_empty()) {
            (true, true) => Ok(None),
            (false, false) => Ok(Some(SourcePin {
                form_id: self.form_id.clone(),
                version: positive(&self.form_version, "form_version")?,
            })),
            _ => Err(invalid(
                "form_id",
                "Provide both form ID and exact version, or leave both blank for world-exclusive content.",
            )),
        }
    }

    fn source_value(&self) -> String {
        self.source_pin.clone().unwrap_or_else(|| {
            if self.form_id.is_empty() && self.form_version.is_empty() {
                String::new()
            } else {
                format!("{}@{}", self.form_id, self.form_version)
            }
        })
    }
}

fn submit_token(state: &AppState, headers: &HeaderMap, csrf: &str) -> Result<String, Error> {
    if !origin_ok(headers, &state.public_origin) || !csrf_matches(headers, csrf) {
        return Err(Error::Forbidden);
    }
    Ok(session(headers).unwrap_or_default())
}

fn parse_content(
    form: Result<Form<ContentForm>, FormRejection>,
) -> Result<ContentForm, FormRejection> {
    form.map(|Form(mut form)| {
        // A newer revision becomes this draft's base only after the explicit
        // reviewed-submit button was used. Keep it if that submission fails validation.
        if !form.reviewed_revision.is_empty() {
            if form.expected_world_revision.is_empty() {
                form.expected_revision = std::mem::take(&mut form.reviewed_revision);
            } else {
                form.expected_world_revision = std::mem::take(&mut form.reviewed_revision);
            }
        }
        form
    })
}

fn authoring_form_error(error: FormRejection) -> Response {
    no_store(if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        message(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Form too large",
            "Authoring forms must be no larger than 32 KiB. Return to the form and reduce its size.",
        )
    } else {
        message(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Check the form",
            "This form could not be read. Return to the form and submit text fields using the provided controls.",
        )
    })
}

struct SavedState {
    fields: ContentFields,
    revision: i64,
    context: String,
}

struct SourceChoice {
    value: String,
    label: String,
}

struct DefinitionComparison {
    label: String,
    form: FormRecord,
    species: SpeciesRecord,
}

#[derive(Template)]
#[template(path = "authoring_form.html")]
struct ContentPage {
    title: String,
    action: String,
    cancel: String,
    context: String,
    draft: ContentForm,
    csrf: String,
    error: String,
    error_title: &'static str,
    errors: BTreeMap<String, String>,
    current: Option<SavedState>,
    reviewed_revision: String,
    choices: Vec<SourceChoice>,
    choose_source: bool,
    show_species_version: bool,
    selected_source: String,
    comparisons: Vec<DefinitionComparison>,
    button: String,
    blocked_reason: String,
}

impl ContentPage {
    fn new(title: &str, action: String, cancel: String, draft: ContentForm) -> Self {
        let selected_source = draft.source_value();
        Self {
            title: title.to_owned(),
            action,
            cancel,
            draft,
            csrf: String::new(),
            context: String::new(),
            error: String::new(),
            error_title: "Nothing was saved",
            errors: BTreeMap::new(),
            current: None,
            reviewed_revision: String::new(),
            choices: Vec::new(),
            choose_source: false,
            show_species_version: false,
            selected_source,
            comparisons: Vec::new(),
            button: "Save changes".to_owned(),
            blocked_reason: String::new(),
        }
    }

    fn field_error(&self, field: &str) -> &str {
        self.errors.get(field).map(String::as_str).unwrap_or("")
    }

    fn slug_information(&self) -> String {
        if !self.draft.slug.is_empty() {
            format!("Stored slug: {}", self.draft.slug)
        } else if (1..=100).contains(&self.draft.display_name.trim().chars().count())
            && !self.draft.display_name.trim().chars().any(char::is_control)
        {
            format!(
                "Generated base preview: {} (final suffix assigned when saved)",
                game_data::authoring::slug_from_name(self.draft.display_name.trim())
            )
        } else {
            "Slug: assigned when saved".into()
        }
    }

    fn failed(&mut self, error: &Error) {
        // Legacy browser input is ignored by the service and is not a stored slug.
        // Creation conflicts may compare parent membership, not a saved child.
        self.draft.slug = if self.action.ends_with("/new") {
            String::new()
        } else {
            self.current
                .as_ref()
                .map(|row| row.fields.slug.clone())
                .unwrap_or_default()
        };
        self.error = error_text(error).to_owned();
        if matches!(error, Error::Unavailable) {
            self.error_title = "Save could not be confirmed";
        }
        if let Error::Invalid(errors) = error {
            self.errors = errors.clone();
        }
    }

    fn conflict(&mut self, fields: ContentFields, revision: i64, context: String) {
        self.current = Some(SavedState {
            fields,
            revision,
            context,
        });
        self.reviewed_revision = revision.to_string();
        self.button = if self.draft.target_version.is_empty() {
            "Save reviewed changes".to_owned()
        } else {
            format!(
                "Save reviewed changes — Adopt version {} with reviewed values",
                self.draft.target_version
            )
        };
    }
}

fn content_response(
    state: &AppState,
    headers: &HeaderMap,
    mut page: ContentPage,
    status: StatusCode,
) -> Response {
    let (csrf, set) = match csrf_for(headers) {
        Ok(value) => value,
        Err(_) => {
            // A draft remains visible even if CSRF cookie preparation fails.
            page.failed(&Error::Unavailable);
            return no_store(render_status(page, StatusCode::SERVICE_UNAVAILABLE));
        }
    };
    page.csrf = csrf.clone();
    let mut response = no_store(render_status(page, status));
    if set {
        append_cookie(&mut response, csrf_cookie(&csrf, state.cookie_secure));
    }
    response
}

fn draft_response(
    state: &AppState,
    headers: &HeaderMap,
    mut page: ContentPage,
    error: Error,
) -> Response {
    page.failed(&error);
    content_response(state, headers, page, error_status(&error))
}

async fn species_new(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let result = async {
        library_permission(&state, &headers).await?;
        let mut page = ContentPage::new("Create species", "/creatures/new".into(), "/creatures".into(), ContentForm::default());
        page.context = "A species is a shared library definition. Creation also creates its default shared form.".into();
        page.button = "Create species".into();
        Ok(content_response(&state, &headers, page, StatusCode::OK))
    }.await;
    no_store(finish(result))
}

async fn species_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(value) => value,
        Err(error) => return authoring_form_error(error),
    };
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        state
            .authoring
            .create_species(&token, draft.fields(), draft.reason())
            .await
    }
    .await;
    match result {
        Ok(species) => {
            no_store(Redirect::to(&format!("/creatures/{}", species.id)).into_response())
        }
        Err(error) => {
            let mut page = ContentPage::new(
                "Create species",
                "/creatures/new".into(),
                "/creatures".into(),
                draft,
            );
            page.button = "Create species".into();
            draft_response(&state, &headers, page, error)
        }
    }
}

async fn species_edit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let result = async {
        library_permission(&state, &headers).await?;
        let species = state.authoring.species_detail(&id).await?;
        let mut page = ContentPage::new("Edit species", format!("/creatures/{id}/edit"), format!("/creatures/{id}"), ContentForm::with_fields(&species.fields, species.revision));
        page.context = format!("Canonical species ID: {id}. Starting version: {}. Existing form pins are unchanged by this edit.", species.revision);
        Ok(content_response(&state, &headers, page, StatusCode::OK))
    }.await;
    no_store(finish(result))
}

async fn species_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(value) => value,
        Err(error) => return authoring_form_error(error),
    };
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        state
            .authoring
            .save_species(&token, &id, draft.edit()?)
            .await
    }
    .await;
    match result {
        Ok(_) => no_store(Redirect::to(&format!("/creatures/{id}")).into_response()),
        Err(error) => {
            let mut page = ContentPage::new(
                "Edit species",
                format!("/creatures/{id}/edit"),
                format!("/creatures/{id}"),
                draft,
            );
            page.context = format!("Canonical species ID: {id}.");
            if matches!(error, Error::Conflict) {
                match state.authoring.species_detail(&id).await {
                    Ok(current) => page.conflict(
                        current.fields,
                        current.revision,
                        format!("Canonical species ID: {id}"),
                    ),
                    Err(load_error) => {
                        page.failed(&error);
                        return draft_response(&state, &headers, page, load_error);
                    }
                }
            }
            draft_response(&state, &headers, page, error)
        }
    }
}

#[derive(Template)]
#[template(path = "form_detail.html")]
struct FormPage {
    form: FormRecord,
    species: SpeciesRecord,
    current_species: SpeciesRecord,
    worlds: Vec<WorldRecord>,
    can_edit_library: bool,
}

async fn form_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let result = async {
        let form = state.authoring.form_detail(&id).await?;
        let current_species = state.authoring.species_detail(&form.species_id).await?;
        let species = state
            .authoring
            .species_version(&form.species_id, form.species_version)
            .await?;
        let worlds = state.authoring.worlds().await?;
        let caps = state
            .authoring
            .capabilities(session(&headers).as_deref(), None)
            .await?;
        Ok(render(FormPage {
            form,
            species,
            current_species,
            worlds,
            can_edit_library: caps.can_edit_library,
        }))
    }
    .await;
    finish(result)
}

async fn form_new(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let result = async {
        library_permission(&state, &headers).await?;
        let species = state.authoring.species_detail(&id).await?;
        let draft = ContentForm {
            species_version: species.revision.to_string(),
            ..ContentForm::default()
        };
        let mut page = ContentPage::new(
            "Create shared form",
            format!("/creatures/{id}/forms/new"),
            format!("/creatures/{id}"),
            draft,
        );
        page.show_species_version = true;
        page.context = format!(
            "Species: {}. Canonical parent ID: {id}. Pinned species version: {}.",
            species.fields.display_name, species.revision
        );
        page.button = "Create shared form".into();
        Ok(content_response(&state, &headers, page, StatusCode::OK))
    }
    .await;
    no_store(finish(result))
}

async fn form_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(value) => value,
        Err(error) => return authoring_form_error(error),
    };
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        state
            .authoring
            .create_form(
                &token,
                &id,
                positive(&draft.species_version, "species_version")?,
                draft.fields(),
                draft.reason(),
            )
            .await
    }
    .await;
    match result {
        Ok(form) => no_store(Redirect::to(&format!("/forms/{}", form.id)).into_response()),
        Err(error) => {
            let mut page = ContentPage::new(
                "Create shared form",
                format!("/creatures/{id}/forms/new"),
                format!("/creatures/{id}"),
                draft,
            );
            page.show_species_version = true;
            page.context = format!("Canonical species parent ID: {id}.");
            page.button = "Create shared form".into();
            draft_response(&state, &headers, page, error)
        }
    }
}

async fn form_edit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let result = async {
        library_permission(&state, &headers).await?;
        let form = state.authoring.form_detail(&id).await?;
        let mut draft = ContentForm::with_fields(&form.fields, form.revision);
        draft.species_version = form.species_version.to_string();
        let mut page = ContentPage::new("Edit shared form", format!("/forms/{id}/edit"), format!("/forms/{id}"), draft);
        page.show_species_version = true;
        page.context = format!("Canonical form ID: {id}. Canonical species parent ID: {}. Starting form version: {}. Existing world pins will not change.", form.species_id, form.revision);
        Ok(content_response(&state, &headers, page, StatusCode::OK))
    }.await;
    no_store(finish(result))
}

async fn form_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(value) => value,
        Err(error) => return authoring_form_error(error),
    };
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        state
            .authoring
            .save_form(
                &token,
                &id,
                positive(&draft.species_version, "species_version")?,
                draft.edit()?,
            )
            .await
    }
    .await;
    match result {
        Ok(_) => no_store(Redirect::to(&format!("/forms/{id}")).into_response()),
        Err(error) => {
            let mut page = ContentPage::new(
                "Edit shared form",
                format!("/forms/{id}/edit"),
                format!("/forms/{id}"),
                draft,
            );
            page.show_species_version = true;
            page.context = format!("Canonical form ID: {id}. Existing world pins will not change.");
            if matches!(error, Error::Conflict) {
                match state.authoring.form_detail(&id).await {
                    Ok(current) => page.conflict(
                        current.fields,
                        current.revision,
                        format!(
                            "Canonical species parent ID: {}. Pinned species version: {}.",
                            current.species_id, current.species_version
                        ),
                    ),
                    Err(load_error) => return draft_response(&state, &headers, page, load_error),
                }
            }
            draft_response(&state, &headers, page, error)
        }
    }
}

#[derive(Template)]
#[template(path = "worlds.html")]
struct WorldsPage {
    worlds: Vec<WorldRecord>,
    can_create_world: bool,
}

async fn worlds(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let result = async {
        let worlds = state.authoring.worlds().await?;
        let caps = state
            .authoring
            .capabilities(session(&headers).as_deref(), None)
            .await?;
        Ok(render(WorldsPage {
            worlds,
            can_create_world: caps.can_create_world,
        }))
    }
    .await;
    finish(result)
}

#[derive(Template)]
#[template(path = "world_detail.html")]
struct WorldPage {
    world: WorldRecord,
    variants: Vec<VariantRecord>,
    forms: Vec<FormRecord>,
    can_edit_world: bool,
    can_manage_access: bool,
}

async fn all_forms(state: &AppState) -> Result<Vec<FormRecord>, Error> {
    let mut forms = Vec::new();
    for species in state.authoring.species().await? {
        forms.extend(state.authoring.forms(&species.id).await?);
    }
    Ok(forms)
}

async fn world_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let result = async {
        let world = state.authoring.world_detail(&id).await?;
        let variants = state.authoring.variants(&id).await?;
        let forms = all_forms(&state).await?;
        let caps = state
            .authoring
            .capabilities(session(&headers).as_deref(), Some(&id))
            .await?;
        Ok(render(WorldPage {
            world,
            variants,
            forms,
            can_edit_world: caps.can_edit_world,
            can_manage_access: caps.can_manage_access,
        }))
    }
    .await;
    finish(result)
}

async fn world_new(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let result = async {
        world_permission(&state, &headers, None).await?;
        let mut page = ContentPage::new("Create world", "/worlds/new".into(), "/worlds".into(), ContentForm::default());
        page.context = "A world is an authored definition, not a player save. Your authenticated account will own this world.".into();
        page.button = "Create world".into();
        Ok(content_response(&state, &headers, page, StatusCode::OK))
    }.await;
    no_store(finish(result))
}

async fn world_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(value) => value,
        Err(error) => return authoring_form_error(error),
    };
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        state
            .authoring
            .create_world(&token, draft.fields(), draft.reason())
            .await
    }
    .await;
    match result {
        Ok(world) => no_store(Redirect::to(&format!("/worlds/{}", world.id)).into_response()),
        Err(error) => {
            let mut page = ContentPage::new(
                "Create world",
                "/worlds/new".into(),
                "/worlds".into(),
                draft,
            );
            page.button = "Create world".into();
            draft_response(&state, &headers, page, error)
        }
    }
}

async fn world_edit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let result = async {
        world_permission(&state, &headers, Some(&id)).await?;
        let world = state.authoring.world_detail(&id).await?;
        let mut page = ContentPage::new("Edit world", format!("/worlds/{id}/edit"), format!("/worlds/{id}"), ContentForm::with_fields(&world.fields, world.revision));
        page.context = format!("Canonical world ID: {id}. Starting content revision: {}. Variants keep their independent fields and pins.", world.revision);
        Ok(content_response(&state, &headers, page, StatusCode::OK))
    }.await;
    no_store(finish(result))
}

async fn world_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(value) => value,
        Err(error) => return authoring_form_error(error),
    };
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        state.authoring.save_world(&token, &id, draft.edit()?).await
    }
    .await;
    match result {
        Ok(_) => no_store(Redirect::to(&format!("/worlds/{id}")).into_response()),
        Err(error) => {
            let mut page = ContentPage::new(
                "Edit world",
                format!("/worlds/{id}/edit"),
                format!("/worlds/{id}"),
                draft,
            );
            page.context = format!("Canonical world ID: {id}.");
            if matches!(error, Error::Conflict) {
                match state.authoring.world_detail(&id).await {
                    Ok(current) => page.conflict(current.fields, current.revision, format!("Canonical world ID: {id}. World membership also advances this revision.")),
                    Err(load_error) => return draft_response(&state, &headers, page, load_error),
                }
            }
            draft_response(&state, &headers, page, error)
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct SourceQuery {
    form_id: String,
    form_version: String,
}

async fn source_choices(state: &AppState, selected: &str) -> Result<Vec<SourceChoice>, Error> {
    let mut choices: Vec<_> = all_forms(state)
        .await?
        .into_iter()
        .map(|form| SourceChoice {
            value: format!("{}@{}", form.id, form.revision),
            label: format!(
                "{} — {} · version {} · species {} version {}",
                form.fields.display_name,
                form.id,
                form.revision,
                form.species_id,
                form.species_version
            ),
        })
        .collect();
    if !selected.is_empty() && !choices.iter().any(|choice| choice.value == selected) {
        choices.insert(
            0,
            SourceChoice {
                value: selected.to_owned(),
                label: format!("Submitted historical/custom pin: {selected}"),
            },
        );
    }
    Ok(choices)
}

async fn variant_new(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(world_id): Path<String>,
    query: Result<Query<SourceQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let result = async {
        world_permission(&state, &headers, Some(&world_id)).await?;
        let Query(query) = query.map_err(|_| invalid("form_id", "Check the source ID and exact version in the link."))?;
        let world = state.authoring.world_detail(&world_id).await?;
        let mut draft = ContentForm { expected_world_revision: world.revision.to_string(), form_id: query.form_id, form_version: query.form_version, ..ContentForm::default() };
        let mut context = format!("Canonical world ID: {world_id}. Starting world membership revision: {}. Blank source creates world-exclusive content. Local slugs are generated automatically. Multiple variants may use the same form.", world.revision);
        if let Some(pin) = draft.source()? {
            let source = state.authoring.form_version(&pin.form_id, pin.version).await?;
            draft.display_name = source.fields.display_name;
            draft.description = source.fields.description;
            context.push_str(&format!(" Source form ID: {}. Pinned version: {}. Species ID: {}. Pinned species version: {}.", source.id, source.revision, source.species_id, source.species_version));
        }
        let mut page = ContentPage::new("Add world variant", format!("/worlds/{world_id}/variants/new"), format!("/worlds/{world_id}"), draft);
        page.context = context;
        page.choose_source = true;
        page.choices = source_choices(&state, &page.selected_source).await?;
        page.button = "Create variant".into();
        Ok(content_response(&state, &headers, page, StatusCode::OK))
    }.await;
    no_store(finish(result))
}

async fn variant_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(world_id): Path<String>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(value) => value,
        Err(error) => return authoring_form_error(error),
    };
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        state
            .authoring
            .create_variant(
                &token,
                &world_id,
                draft.world_revision()?,
                draft.source()?,
                draft.fields(),
                draft.reason(),
            )
            .await
    }
    .await;
    match result {
        Ok(variant) => no_store(
            Redirect::to(&format!("/worlds/{world_id}/variants/{}", variant.id)).into_response(),
        ),
        Err(error) => {
            let mut page = ContentPage::new(
                "Add world variant",
                format!("/worlds/{world_id}/variants/new"),
                format!("/worlds/{world_id}"),
                draft,
            );
            page.context = format!(
                "Canonical world ID: {world_id}. Local slugs are generated automatically. Source selection is an explicit pin; no latest version is inferred."
            );
            page.choose_source = true;
            page.button = "Create variant".into();
            match source_choices(&state, &page.selected_source).await {
                Ok(choices) => page.choices = choices,
                Err(_) => {
                    if !page.selected_source.is_empty() {
                        page.choices.push(SourceChoice {
                            value: page.selected_source.clone(),
                            label: format!("Retained submitted pin: {}", page.selected_source),
                        });
                    }
                }
            }
            if matches!(error, Error::Conflict) {
                match state.authoring.world_detail(&world_id).await {
                    Ok(current) => page.conflict(current.fields, current.revision, "Current world metadata/membership revision. Review this world before adding your variant.".into()),
                    Err(load_error) => return draft_response(&state, &headers, page, load_error),
                }
            }
            draft_response(&state, &headers, page, error)
        }
    }
}

#[derive(Template)]
#[template(path = "variant_detail.html")]
struct VariantPage {
    variant: VariantRecord,
    world: WorldRecord,
    source: Option<FormRecord>,
    latest: Option<FormRecord>,
    can_edit_world: bool,
}

async fn variant_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world_id, id)): Path<(String, String)>,
) -> Response {
    let result = async {
        let variant = state.authoring.variant_detail(&world_id, &id).await?;
        let world = state.authoring.world_detail(&world_id).await?;
        let (source, latest) = if let Some(pin) = &variant.source {
            (
                Some(
                    state
                        .authoring
                        .form_version(&pin.form_id, pin.version)
                        .await?,
                ),
                Some(state.authoring.form_detail(&pin.form_id).await?),
            )
        } else {
            (None, None)
        };
        let caps = state
            .authoring
            .capabilities(session(&headers).as_deref(), Some(&world_id))
            .await?;
        Ok(render(VariantPage {
            variant,
            world,
            source,
            latest,
            can_edit_world: caps.can_edit_world,
        }))
    }
    .await;
    finish(result)
}

fn variant_context(entry: &VariantRecord) -> String {
    let source = match &entry.source {
        Some(pin) => format!(
            "Source form ID: {}. Pinned form version: {}. Saving local fields does not change this pin; adoption is a separate action.",
            pin.form_id, pin.version
        ),
        None => "World-exclusive content: no shared source pin.".into(),
    };
    format!(
        "Canonical world ID: {}. Canonical variant ID: {}. Revision: {}. {source}",
        entry.world_id, entry.id, entry.revision
    )
}

async fn variant_edit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world_id, id)): Path<(String, String)>,
) -> Response {
    let result = async {
        world_permission(&state, &headers, Some(&world_id)).await?;
        let variant = state.authoring.variant_detail(&world_id, &id).await?;
        let mut page = ContentPage::new(
            "Edit world variant",
            format!("/worlds/{world_id}/variants/{id}/edit"),
            format!("/worlds/{world_id}/variants/{id}"),
            ContentForm::with_fields(&variant.fields, variant.revision),
        );
        page.context = variant_context(&variant);
        Ok(content_response(&state, &headers, page, StatusCode::OK))
    }
    .await;
    no_store(finish(result))
}

async fn variant_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world_id, id)): Path<(String, String)>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(value) => value,
        Err(error) => return authoring_form_error(error),
    };
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        state
            .authoring
            .save_variant(&token, &world_id, &id, draft.edit()?)
            .await
    }
    .await;
    match result {
        Ok(_) => {
            no_store(Redirect::to(&format!("/worlds/{world_id}/variants/{id}")).into_response())
        }
        Err(error) => {
            let mut page = ContentPage::new(
                "Edit world variant",
                format!("/worlds/{world_id}/variants/{id}/edit"),
                format!("/worlds/{world_id}/variants/{id}"),
                draft,
            );
            page.context = format!(
                "Canonical world ID: {world_id}. Canonical variant ID: {id}. Editing does not change the source pin."
            );
            if matches!(error, Error::Conflict) {
                match state.authoring.variant_detail(&world_id, &id).await {
                    Ok(current) => {
                        let context = variant_context(&current);
                        page.conflict(current.fields, current.revision, context);
                    }
                    Err(load_error) => return draft_response(&state, &headers, page, load_error),
                }
            }
            draft_response(&state, &headers, page, error)
        }
    }
}

#[derive(Deserialize)]
struct AdoptionQuery {
    target_version: String,
}

async fn adoption_comparisons(
    state: &AppState,
    world_id: &str,
    id: &str,
    target: i64,
    page: &mut ContentPage,
) -> Result<VariantRecord, Error> {
    let preview = state
        .authoring
        .adoption_preview(world_id, id, target)
        .await?;
    page.context = format!(
        "{}. Review old source, your local effective values, and new source below. The editable draft starts with LOCAL values. Choose the name and description manually; adoption changes the baseline, not an automatic merge. A name change also regenerates the local slug.",
        variant_context(&preview.entry)
    );
    page.comparisons = vec![
        DefinitionComparison {
            label: "Old source".into(),
            form: preview.old_source,
            species: preview.old_species,
        },
        DefinitionComparison {
            label: "New source".into(),
            form: preview.new_source,
            species: preview.new_species,
        },
    ];
    Ok(preview.entry)
}

async fn adoption_form(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world_id, id)): Path<(String, String)>,
    query: Result<Query<AdoptionQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let result = async {
        world_permission(&state, &headers, Some(&world_id)).await?;
        let Query(query) = query.map_err(|_| {
            invalid(
                "target_version",
                "Choose an explicit newer version to adopt.",
            )
        })?;
        let target = positive(&query.target_version, "target_version")?;
        let mut page = ContentPage::new(
            "Review source adoption",
            format!("/worlds/{world_id}/variants/{id}/adopt"),
            format!("/worlds/{world_id}/variants/{id}"),
            ContentForm::default(),
        );
        let entry = adoption_comparisons(&state, &world_id, &id, target, &mut page).await?;
        page.draft = ContentForm {
            target_version: query.target_version,
            ..ContentForm::with_fields(&entry.fields, entry.revision)
        };
        page.current = Some(SavedState {
            fields: entry.fields,
            revision: entry.revision,
            context: "Your currently saved local effective values".into(),
        });
        page.button = format!("Adopt version {target} with reviewed values");
        Ok(content_response(&state, &headers, page, StatusCode::OK))
    }
    .await;
    no_store(finish(result))
}

async fn adoption_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world_id, id)): Path<(String, String)>,
    form: Result<Form<ContentForm>, FormRejection>,
) -> Response {
    let draft = match parse_content(form) {
        Ok(value) => value,
        Err(error) => return authoring_form_error(error),
    };
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        state
            .authoring
            .adopt_variant(
                &token,
                &world_id,
                &id,
                positive(&draft.target_version, "target_version")?,
                draft.edit()?,
            )
            .await
    }
    .await;
    match result {
        Ok(_) => {
            no_store(Redirect::to(&format!("/worlds/{world_id}/variants/{id}")).into_response())
        }
        Err(error) => {
            let mut page = ContentPage::new(
                "Review source adoption",
                format!("/worlds/{world_id}/variants/{id}/adopt"),
                format!("/worlds/{world_id}/variants/{id}"),
                draft,
            );
            page.button = format!(
                "Adopt version {} with reviewed values",
                page.draft.target_version
            );
            if let Ok(target) = positive(&page.draft.target_version, "target_version") {
                match adoption_comparisons(&state, &world_id, &id, target, &mut page).await {
                    Ok(current) => {
                        if matches!(error, Error::Conflict) {
                            let context = variant_context(&current);
                            page.conflict(current.fields, current.revision, context);
                        } else {
                            page.current = Some(SavedState {
                                fields: current.fields,
                                revision: current.revision,
                                context: "Current local effective values".into(),
                            });
                        }
                    }
                    Err(_) if matches!(error, Error::Conflict) => {
                        match state.authoring.variant_detail(&world_id, &id).await {
                            Ok(current) => {
                                let context = variant_context(&current);
                                if current
                                    .source
                                    .as_ref()
                                    .is_some_and(|source| source.version >= target)
                                {
                                    page.blocked_reason = "Another editor has already adopted this source version or a later one. Your draft is retained below. Return to the variant to review a newer target, or copy these values into its local-values edit form; the source will not be silently downgraded.".into();
                                }
                                page.conflict(current.fields, current.revision, context);
                            }
                            Err(load_error) => {
                                return draft_response(&state, &headers, page, load_error);
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
            draft_response(&state, &headers, page, error)
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct AccessForm {
    csrf: String,
    expected_access_revision: String,
    reviewed_access_revision: String,
    user_id: String,
    reason: String,
}

#[derive(Template)]
#[template(path = "access.html")]
struct AccessPage {
    world_id: String,
    state: Option<AccessState>,
    draft: AccessForm,
    csrf: String,
    error: String,
    error_title: &'static str,
    errors: BTreeMap<String, String>,
    operation: String,
    reviewed_revision: String,
}

impl AccessPage {
    fn field_error(&self, field: &str) -> &str {
        self.errors.get(field).map(String::as_str).unwrap_or("")
    }
}

fn access_response(
    state: &AppState,
    headers: &HeaderMap,
    mut page: AccessPage,
    status: StatusCode,
) -> Response {
    let (csrf, set) = match csrf_for(headers) {
        Ok(value) => value,
        Err(_) => {
            page.error = error_text(&Error::Unavailable).into();
            page.error_title = "Access update could not be confirmed";
            return no_store(render_status(page, StatusCode::SERVICE_UNAVAILABLE));
        }
    };
    page.csrf = csrf.clone();
    let mut response = no_store(render_status(page, status));
    if set {
        append_cookie(&mut response, csrf_cookie(&csrf, state.cookie_secure));
    }
    response
}

async fn access(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let result = async {
        let access = state
            .authoring
            .access(session(&headers).as_deref().unwrap_or(""), &id)
            .await?;
        let draft = AccessForm {
            expected_access_revision: access.world.access_revision.to_string(),
            ..AccessForm::default()
        };
        Ok(access_response(
            &state,
            &headers,
            AccessPage {
                world_id: id,
                state: Some(access),
                draft,
                csrf: String::new(),
                error: String::new(),
                error_title: "Access was not changed",
                errors: BTreeMap::new(),
                operation: "grant".into(),
                reviewed_revision: String::new(),
            },
            StatusCode::OK,
        ))
    }
    .await;
    no_store(finish(result))
}

async fn access_grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    form: Result<Form<AccessForm>, FormRejection>,
) -> Response {
    access_submit(state, headers, id, form, true).await
}

async fn access_revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    form: Result<Form<AccessForm>, FormRejection>,
) -> Response {
    access_submit(state, headers, id, form, false).await
}

async fn access_submit(
    state: AppState,
    headers: HeaderMap,
    id: String,
    form: Result<Form<AccessForm>, FormRejection>,
    grant: bool,
) -> Response {
    let mut draft = match form {
        Ok(Form(value)) => value,
        Err(error) => return authoring_form_error(error),
    };
    if !draft.reviewed_access_revision.is_empty() {
        draft.expected_access_revision = std::mem::take(&mut draft.reviewed_access_revision);
    }
    let result = async {
        let token = submit_token(&state, &headers, &draft.csrf)?;
        let revision = positive(
            if draft.reviewed_access_revision.is_empty() {
                &draft.expected_access_revision
            } else {
                &draft.reviewed_access_revision
            },
            "expected_access_revision",
        )?;
        let reason = if draft.reason.is_empty() {
            None
        } else {
            Some(draft.reason.as_str())
        };
        if grant {
            state
                .authoring
                .grant_collaborator(&token, &id, revision, &draft.user_id, reason)
                .await
        } else {
            state
                .authoring
                .revoke_collaborator(&token, &id, revision, &draft.user_id, reason)
                .await
        }
    }
    .await;
    match result {
        Ok(()) => no_store(Redirect::to(&format!("/worlds/{id}/access")).into_response()),
        Err(error) => {
            let mut page = AccessPage {
                world_id: id.clone(),
                state: None,
                draft,
                csrf: String::new(),
                error: error_text(&error).into(),
                error_title: if matches!(error, Error::Unavailable) {
                    "Access update could not be confirmed"
                } else {
                    "Access was not changed"
                },
                errors: BTreeMap::new(),
                operation: if grant { "grant" } else { "revoke" }.into(),
                reviewed_revision: String::new(),
            };
            if let Error::Invalid(errors) = &error {
                page.errors = errors.clone();
            }
            // Private state is loaded only through the owner-authorized use case.
            if let Ok(current) = state
                .authoring
                .access(session(&headers).as_deref().unwrap_or(""), &id)
                .await
            {
                if matches!(error, Error::Conflict) {
                    page.reviewed_revision = current.world.access_revision.to_string();
                }
                page.state = Some(current);
            }
            access_response(&state, &headers, page, error_status(&error))
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct HistoryQuery {
    before: String,
}

#[derive(Template)]
#[template(path = "history.html")]
struct HistoryPage {
    object_id: String,
    object_label: &'static str,
    detail_url: String,
    history_url: String,
    entries: Vec<HistoryView>,
    next_before: Option<i64>,
}

#[derive(Template)]
#[template(path = "snapshot.html")]
struct SnapshotPage {
    object_id: String,
    object_label: &'static str,
    history_url: String,
    entry: HistoryEntry,
    changes: Vec<FieldChange>,
}

struct FieldChange {
    label: &'static str,
    before: String,
    after: String,
}

struct HistoryView {
    entry: HistoryEntry,
    changes: Vec<FieldChange>,
}

fn history_changes(entry: &HistoryEntry) -> Result<Vec<FieldChange>, Error> {
    let after: serde_json::Value =
        serde_json::from_str(&entry.after_json).map_err(|_| Error::Unavailable)?;
    let before: Option<serde_json::Value> = entry
        .before_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|_| Error::Unavailable)?;
    let mut changes = Vec::new();
    // Presentation of known snapshot fields only; relationship semantics remain
    // in the application service. JSON snapshots have already been validated.
    for (label, path) in [
        ("Slug", "/content/fields/slug"),
        ("Display name", "/content/fields/display_name"),
        ("Description", "/content/fields/description"),
        ("Pinned species version", "/content/species_version"),
        ("Shared form source", "/content/source"),
        ("World variant membership", "/content/variant_ids"),
    ] {
        let old = before.as_ref().and_then(|value| value.pointer(path));
        let new = after.pointer(path);
        if old != new {
            changes.push(FieldChange {
                label,
                before: history_value(old),
                after: history_value(new),
            });
        }
    }
    Ok(changes)
}

fn history_value(value: Option<&serde_json::Value>) -> String {
    use serde_json::Value;
    match value {
        None => "Not present (initial creation)".into(),
        Some(Value::Null) => "World-exclusive — no shared source".into(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(ids)) => {
            let visible = ids
                .iter()
                .take(12)
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{} variants: {}{}",
                ids.len(),
                visible,
                if ids.len() > 12 {
                    " … see the full snapshot for all IDs"
                } else {
                    ""
                }
            )
        }
        Some(Value::Object(pin)) => format!(
            "Form {} · version {}",
            pin.get("form_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            pin.get("version").map(Value::to_string).unwrap_or_default()
        ),
        Some(value) => value.to_string(),
    }
}

fn history_label(kind: HistoryKind) -> &'static str {
    match kind {
        HistoryKind::Species => "Species",
        HistoryKind::Form => "Shared form",
        HistoryKind::World => "World",
        HistoryKind::Variant => "World variant",
    }
}

async fn history_response(
    state: &AppState,
    kind: HistoryKind,
    id: String,
    detail_url: String,
    query: Result<Query<HistoryQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, Error> {
    let Query(query) =
        query.map_err(|_| invalid("before", "Use a positive revision as the history cursor."))?;
    let before = if query.before.is_empty() {
        None
    } else {
        Some(positive(&query.before, "before")?)
    };
    let mut entries = state.authoring.history(kind, &id, before).await?;
    entries.truncate(50);
    let next_before = if entries.len() == 50 {
        entries.last().map(|entry| entry.revision)
    } else {
        None
    };
    let history_url = format!("{detail_url}/history");
    let entries = entries
        .into_iter()
        .map(|entry| {
            let changes = history_changes(&entry)?;
            Ok(HistoryView { entry, changes })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(render(HistoryPage {
        object_id: id,
        object_label: history_label(kind),
        detail_url,
        history_url,
        entries,
        next_before,
    }))
}

async fn snapshot_response(
    state: &AppState,
    kind: HistoryKind,
    id: String,
    revision: String,
    history_url: String,
) -> Result<Response, Error> {
    let entry = state
        .authoring
        .history_entry(kind, &id, positive(&revision, "revision")?)
        .await?;
    Ok(render(SnapshotPage {
        object_id: id,
        object_label: history_label(kind),
        history_url,
        changes: history_changes(&entry)?,
        entry,
    }))
}

async fn species_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
    query: Result<Query<HistoryQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let result = async {
        state.authoring.species_detail(&id).await?;
        let url = format!("/creatures/{id}");
        history_response(&state, HistoryKind::Species, id, url, query).await
    }
    .await;
    finish(result)
}

async fn form_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
    query: Result<Query<HistoryQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let result = async {
        state.authoring.form_detail(&id).await?;
        let url = format!("/forms/{id}");
        history_response(&state, HistoryKind::Form, id, url, query).await
    }
    .await;
    finish(result)
}

async fn world_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
    query: Result<Query<HistoryQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let result = async {
        state.authoring.world_detail(&id).await?;
        let url = format!("/worlds/{id}");
        history_response(&state, HistoryKind::World, id, url, query).await
    }
    .await;
    finish(result)
}

async fn variant_history(
    State(state): State<AppState>,
    Path((world_id, id)): Path<(String, String)>,
    query: Result<Query<HistoryQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let result = async {
        state.authoring.variant_detail(&world_id, &id).await?;
        let url = format!("/worlds/{world_id}/variants/{id}");
        history_response(&state, HistoryKind::Variant, id, url, query).await
    }
    .await;
    finish(result)
}

async fn species_snapshot(
    State(state): State<AppState>,
    Path((id, revision)): Path<(String, String)>,
) -> Response {
    let result = async {
        state.authoring.species_detail(&id).await?;
        let url = format!("/creatures/{id}/history");
        snapshot_response(&state, HistoryKind::Species, id, revision, url).await
    }
    .await;
    finish(result)
}

async fn form_snapshot(
    State(state): State<AppState>,
    Path((id, revision)): Path<(String, String)>,
) -> Response {
    let result = async {
        state.authoring.form_detail(&id).await?;
        let url = format!("/forms/{id}/history");
        snapshot_response(&state, HistoryKind::Form, id, revision, url).await
    }
    .await;
    finish(result)
}

async fn world_snapshot(
    State(state): State<AppState>,
    Path((id, revision)): Path<(String, String)>,
) -> Response {
    let result = async {
        state.authoring.world_detail(&id).await?;
        let url = format!("/worlds/{id}/history");
        snapshot_response(&state, HistoryKind::World, id, revision, url).await
    }
    .await;
    finish(result)
}

async fn variant_snapshot(
    State(state): State<AppState>,
    Path((world_id, id, revision)): Path<(String, String, String)>,
) -> Response {
    let result = async {
        state.authoring.variant_detail(&world_id, &id).await?;
        let url = format!("/worlds/{world_id}/variants/{id}/history");
        snapshot_response(&state, HistoryKind::Variant, id, revision, url).await
    }
    .await;
    finish(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::{Role, Subject},
        persistence,
    };
    use axum::body::{Body, to_bytes};
    use tempfile::TempDir;
    use tower::ServiceExt;

    struct TestSite {
        state: AppState,
        router: Router,
        token: String,
        _directory: TempDir,
    }

    async fn site() -> TestSite {
        let directory = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}/authoring.db", directory.path().display());
        let pool = persistence::connect_for_migration(&url).await.unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool, 3600).unwrap();
        accounts
            .invite("test", "owner", Role::Administrator, None)
            .await
            .unwrap();
        let actor = accounts
            .sign_in(Subject {
                provider: "test".into(),
                provider_subject: "owner".into(),
                display_name: "Owner".into(),
            })
            .await
            .unwrap();
        let (token, _) = accounts.start_session(&actor.user_id).await.unwrap();
        let state = AppState::new(accounts, None, "http://127.0.0.1:3000".into(), false);
        TestSite {
            router: super::super::router(state.clone()),
            state,
            token,
            _directory: directory,
        }
    }

    fn fields(slug: &str, name: &str) -> ContentFields {
        ContentFields {
            slug: slug.into(),
            display_name: name.into(),
            description: "Plain text\n\nSecond paragraph".into(),
        }
    }

    impl TestSite {
        async fn request(
            &self,
            method: &str,
            path: &str,
            body: &str,
            authenticated: bool,
        ) -> (StatusCode, HeaderMap, String) {
            let csrf = "a".repeat(64);
            let cookie = if authenticated {
                format!("karta_csrf={csrf}; karta_session={}", self.token)
            } else {
                format!("karta_csrf={csrf}")
            };
            let mut path = path.to_owned();
            let mut hops = 0;
            let response = loop {
                let request = axum::http::Request::builder()
                    .method(method)
                    .uri(&path)
                    .header(header::ORIGIN, "http://127.0.0.1:3000")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body.to_owned()))
                    .unwrap();
                let response = self.router.clone().oneshot(request).await.unwrap();
                if method != "GET" || response.status() != StatusCode::TEMPORARY_REDIRECT {
                    break response;
                }
                hops += 1;
                assert!(hops <= 4, "GET redirect loop");
                path = response.headers()[header::LOCATION]
                    .to_str()
                    .unwrap()
                    .to_owned();
                assert!(path.starts_with('/') && !path.starts_with("//"));
            };
            let status = response.status();
            let headers = response.headers().clone();
            let bytes = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
            (status, headers, String::from_utf8(bytes.to_vec()).unwrap())
        }

        fn body(extra: &str) -> String {
            format!(
                "csrf={}&slug=draft&display_name=Submitted+draft&description=Draft+description&reason=Retained+reason&{extra}",
                "a".repeat(64)
            )
        }
    }

    #[tokio::test]
    async fn authoring_posts_require_origin_csrf_and_bounded_forms_and_ignore_actor_fields() {
        let site = site().await;
        let csrf = "a".repeat(64);
        let body = TestSite::body("owner_user_id=forged&actor_id=forged");
        let cookie = format!("karta_csrf={csrf}; karta_session={}", site.token);
        for (origin, cookies, data, expected_status) in [
            (None, cookie.clone(), body.clone(), StatusCode::FORBIDDEN),
            (
                Some("https://another.example"),
                cookie.clone(),
                body.clone(),
                StatusCode::FORBIDDEN,
            ),
            (
                Some("http://127.0.0.1:3000"),
                format!("karta_csrf=short; karta_session={}", site.token),
                body.clone(),
                StatusCode::FORBIDDEN,
            ),
            (
                Some("http://127.0.0.1:3000"),
                format!("karta_csrf={csrf}; {cookie}"),
                body.clone(),
                StatusCode::FORBIDDEN,
            ),
            (
                Some("http://127.0.0.1:3000"),
                cookie.clone(),
                format!("{body}&padding={}", "x".repeat(FORM_LIMIT)),
                StatusCode::PAYLOAD_TOO_LARGE,
            ),
        ] {
            let mut request = axum::http::Request::builder()
                .method("POST")
                .uri("/worlds/new")
                .header(header::COOKIE, cookies)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
            if let Some(origin) = origin {
                request = request.header(header::ORIGIN, origin);
            }
            let response = site
                .router
                .clone()
                .oneshot(request.body(Body::from(data)).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), expected_status);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            assert!(site.state.authoring.worlds().await.unwrap().is_empty());
        }
        let actor = site
            .state
            .accounts
            .authenticate(&site.token)
            .await
            .unwrap()
            .unwrap();
        let (status, _, _) = site.request("POST", "/worlds/new", &body, true).await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        let world = site.state.authoring.worlds().await.unwrap().remove(0);
        assert_eq!(world.owner_user_id, actor.user_id);
        let (status, _, history) = site
            .request("GET", &format!("/worlds/{}/history/1", world.id), "", false)
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(history.contains("Field changes"));
        assert!(history.contains("Submitted draft"));
        assert!(!history.contains("owner_user_id"));
    }

    #[tokio::test]
    async fn competing_adoption_shows_current_pin_and_blocks_an_obsolete_target() {
        let site = site().await;
        let species = site
            .state
            .authoring
            .create_species(&site.token, fields("source", "Source"), None)
            .await
            .unwrap();
        let world = site
            .state
            .authoring
            .create_world(&site.token, fields("world", "World"), None)
            .await
            .unwrap();
        let variant = site
            .state
            .authoring
            .create_variant(
                &site.token,
                &world.id,
                1,
                Some(SourcePin {
                    form_id: species.default_form_id.clone(),
                    version: 1,
                }),
                fields("local", "Local"),
                None,
            )
            .await
            .unwrap();
        site.state
            .authoring
            .save_form(
                &site.token,
                &species.default_form_id,
                1,
                Edit {
                    expected_revision: 1,
                    fields: fields("source", "New source"),
                    reason: None,
                },
            )
            .await
            .unwrap();
        site.state
            .authoring
            .adopt_variant(
                &site.token,
                &world.id,
                &variant.id,
                2,
                Edit {
                    expected_revision: 1,
                    fields: variant.fields.clone(),
                    reason: None,
                },
            )
            .await
            .unwrap();
        let url = format!("/worlds/{}/variants/{}/adopt", world.id, variant.id);
        let (status, _, html) = site
            .request(
                "POST",
                &url,
                &TestSite::body("expected_revision=1&target_version=2"),
                true,
            )
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(html.contains("already adopted this source version or a later one"));
        assert!(html.contains("Current saved state — revision 2"));
        assert!(html.contains("value=\"Submitted draft\""));
        assert!(html.contains("name=\"reviewed_revision\" value=\"2\" disabled"));
        let current = site
            .state
            .authoring
            .variant_detail(&world.id, &variant.id)
            .await
            .unwrap();
        assert_eq!(current.revision, 2);
        assert_eq!(current.fields, variant.fields);
        assert_eq!(current.source.unwrap().version, 2);
    }

    #[tokio::test]
    async fn edit_conflict_keeps_draft_and_requires_reviewed_submission() {
        let site = site().await;
        let species = site
            .state
            .authoring
            .create_species(&site.token, fields("original", "Original"), None)
            .await
            .unwrap();
        let path = format!("/creatures/{}/edit", species.id);
        site.state
            .authoring
            .save_species(
                &site.token,
                &species.id,
                Edit {
                    expected_revision: 1,
                    fields: fields("current", "Current saved name"),
                    reason: None,
                },
            )
            .await
            .unwrap();

        let (status, headers, html) = site
            .request("POST", &path, &TestSite::body("expected_revision=1"), true)
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert!(html.contains("Current saved name"));
        assert!(html.contains("value=\"Submitted draft\""));
        assert!(html.contains("value=\"Retained reason\""));
        assert!(html.contains("name=\"expected_revision\" value=\"1\""));
        assert!(html.contains("name=\"reviewed_revision\" value=\"2\""));
        assert!(html.contains("Save reviewed changes"));
        assert_eq!(
            site.state
                .authoring
                .species_detail(&species.id)
                .await
                .unwrap()
                .revision,
            2
        );

        let reviewed_invalid = TestSite::body("expected_revision=1&reviewed_revision=2").replace(
            "reason=Retained+reason",
            &format!("reason={}", "x".repeat(501)),
        );
        let (status, _, html) = site.request("POST", &path, &reviewed_invalid, true).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(html.contains("name=\"expected_revision\" value=\"2\""));
        assert!(html.contains(&"x".repeat(501)));

        let (status, _, _) = site
            .request(
                "POST",
                &path,
                &TestSite::body("expected_revision=1&reviewed_revision=2"),
                true,
            )
            .await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        let saved = site
            .state
            .authoring
            .species_detail(&species.id)
            .await
            .unwrap();
        assert_eq!(saved.fields.display_name, "Submitted draft");
        assert_eq!(saved.revision, 3);
    }

    #[tokio::test]
    async fn errors_keep_text_reason_and_return_field_and_auth_statuses() {
        let site = site().await;
        let species = site
            .state
            .authoring
            .create_species(&site.token, fields("original", "Original"), None)
            .await
            .unwrap();
        let path = format!("/creatures/{}/edit", species.id);
        let body = format!(
            "csrf={}&slug=BAD+SLUG&display_name=%3Cscript%3Edraft%3C%2Fscript%3E&description=Draft%0A%0Aparagraph%00&reason=Keep+this+reason&expected_revision=1",
            "a".repeat(64)
        );
        let (status, _, html) = site.request("POST", &path, &body, true).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(html.contains("description-error"));
        assert!(html.contains("aria-invalid=\"true\""));
        assert!(!html.contains("BAD SLUG"));
        assert!(html.contains("Keep this reason"));
        assert!(html.contains("Draft\n\nparagraph"));
        assert!(!html.contains("<script>draft</script>"));

        site.state.accounts.sign_out(&site.token).await.unwrap();
        let (status, headers, html) = site.request("POST", &path, &body, true).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert!(html.contains("Keep this reason"));
        assert!(!html.contains("BAD SLUG"));
        let (status, _, _) = site.request("GET", &path, "", true).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let expired_form = body.replace(&"a".repeat(64), &"b".repeat(64));
        let (status, headers, html) = site.request("POST", &path, &expired_form, true).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert!(html.contains("Keep this reason"));

        site.state.accounts.pool().close().await;
        let (status, headers, html) = site.request("POST", &path, &body, true).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert!(html.contains("Keep this reason"));
        assert!(!html.contains("BAD SLUG"));
    }

    #[tokio::test]
    async fn variants_pin_versions_adoption_starts_local_and_history_is_scoped() {
        let site = site().await;
        let species = site
            .state
            .authoring
            .create_species(&site.token, fields("source", "Source one"), None)
            .await
            .unwrap();
        let world = site
            .state
            .authoring
            .create_world(&site.token, fields("world", "World"), None)
            .await
            .unwrap();
        let pin = format!("{}%401", species.default_form_id);
        let create_url = format!("/worlds/{}/variants/new", world.id);
        let (status, _, _) = site.request("POST", &create_url, &TestSite::body(&format!("expected_world_revision=1&source_pin={pin}&actor_id=ignored&owner_user_id=ignored")), true).await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        let variants = site.state.authoring.variants(&world.id).await.unwrap();
        let variant = &variants[0];
        assert_eq!(variant.source.as_ref().unwrap().version, 1);
        let form = site
            .state
            .authoring
            .form_detail(&species.default_form_id)
            .await
            .unwrap();
        site.state
            .authoring
            .save_form(
                &site.token,
                &form.id,
                form.species_version,
                Edit {
                    expected_revision: 1,
                    fields: fields("source-new", "Source two"),
                    reason: None,
                },
            )
            .await
            .unwrap();
        let adopt_url = format!("/worlds/{}/variants/{}/adopt", world.id, variant.id);
        let (status, headers, html) = site
            .request("GET", &format!("{adopt_url}?target_version=2"), "", true)
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert!(html.contains("Old source"));
        assert!(html.contains("New source"));
        assert!(html.contains("Source two"));
        assert!(html.contains("value=\"Submitted draft\""));
        assert!(html.contains("Adopt version 2 with reviewed values"));
        let (status, _, _) = site
            .request(
                "POST",
                &adopt_url,
                &TestSite::body("expected_revision=1&target_version=2"),
                true,
            )
            .await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        let saved = site
            .state
            .authoring
            .variant_detail(&world.id, &variant.id)
            .await
            .unwrap();
        assert_eq!(saved.source.unwrap().version, 2);
        assert_eq!(saved.fields.display_name, "Submitted draft");
        assert_eq!(saved.fields.slug, "submitted-draft");

        let current_world = site.state.authoring.world_detail(&world.id).await.unwrap();
        let second = TestSite::body(&format!(
            "expected_world_revision={}&source_pin={pin}",
            current_world.revision
        ))
        .replace("slug=draft", "slug=second-variant");
        assert_eq!(
            site.request("POST", &create_url, &second, true).await.0,
            StatusCode::SEE_OTHER
        );
        let current_world = site.state.authoring.world_detail(&world.id).await.unwrap();
        let exclusive = TestSite::body(&format!(
            "expected_world_revision={}&source_pin=",
            current_world.revision
        ))
        .replace("slug=draft", "slug=world-exclusive");
        assert_eq!(
            site.request("POST", &create_url, &exclusive, true).await.0,
            StatusCode::SEE_OTHER
        );
        let entries = site.state.authoring.variants(&world.id).await.unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().any(|entry| entry.source.is_none()));
        assert!(entries.iter().any(|entry| {
            entry
                .source
                .as_ref()
                .is_some_and(|source| source.version == 1)
        }));

        let other_world = site
            .state
            .authoring
            .create_world(&site.token, fields("other", "Other world"), None)
            .await
            .unwrap();
        let (status, _, _) = site
            .request(
                "GET",
                &format!("/worlds/{}/variants/{}/history", other_world.id, variant.id),
                "",
                false,
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _, html) = site
            .request(
                "GET",
                &format!("/worlds/{}/variants/{}/history/2", world.id, variant.id),
                "",
                false,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains("Before"));
        assert!(html.contains("After"));
        assert!(html.contains("Submitted draft"));
    }

    #[tokio::test]
    async fn owner_access_conflict_retains_target_and_private_history_is_denied_to_collaborator() {
        let mut site = site().await;
        site.state
            .accounts
            .invite("test", "editor", Role::Editor, None)
            .await
            .unwrap();
        let editor = site
            .state
            .accounts
            .sign_in(Subject {
                provider: "test".into(),
                provider_subject: "editor".into(),
                display_name: "Editor".into(),
            })
            .await
            .unwrap();
        let (editor_token, _) = site
            .state
            .accounts
            .start_session(&editor.user_id)
            .await
            .unwrap();
        let world = site
            .state
            .authoring
            .create_world(&site.token, fields("world", "World"), None)
            .await
            .unwrap();
        site.state
            .authoring
            .grant_collaborator(
                &site.token,
                &world.id,
                1,
                &editor.user_id,
                Some("Private grant reason"),
            )
            .await
            .unwrap();
        let body = format!(
            "csrf={}&expected_access_revision=1&user_id={}&reason=Retained+access+reason&actor_id=ignored",
            "a".repeat(64),
            editor.user_id
        );
        let grant_path = format!("/worlds/{}/access/grant", world.id);
        let (status, headers, html) = site.request("POST", &grant_path, &body, true).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert!(html.contains("Retained access reason"));
        assert!(html.contains(&format!("value=\"{}\"", editor.user_id)));
        assert!(html.contains("name=\"expected_access_revision\" value=\"1\""));
        assert!(html.contains("name=\"reviewed_access_revision\" value=\"2\""));
        assert!(html.contains("Private grant reason"));
        let owner_token = site.token.clone();
        site.token = editor_token;
        assert_eq!(
            site.request("GET", &format!("/worlds/{}/edit", world.id), "", true)
                .await
                .0,
            StatusCode::OK
        );
        let (status, headers, html) = site
            .request("GET", &format!("/worlds/{}/access", world.id), "", true)
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert!(!html.contains("Private grant reason"));
        let (_, _, public) = site
            .request("GET", &format!("/worlds/{}/history/1", world.id), "", false)
            .await;
        assert!(!public.contains("Private grant reason"));
        assert!(!public.contains("collaborator_ids"));

        site.token = owner_token;
        let revoke_path = format!("/worlds/{}/access/revoke", world.id);
        let reviewed = format!("{body}&reviewed_access_revision=2");
        assert_eq!(
            site.request("POST", &revoke_path, &reviewed, true).await.0,
            StatusCode::SEE_OTHER
        );
        assert!(
            site.state
                .authoring
                .access(&site.token, &world.id)
                .await
                .unwrap()
                .collaborators
                .is_empty()
        );
    }
}
