//! GET/HEAD address translation before the ID-oriented router matches.
use super::*;
use crate::application::authoring::{AddressKey, Error};
use axum::{body::Body, http::Uri};

fn is_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The discriminator is URL syntax only; stored slugs remain unchanged.
pub(super) fn segment(slug: &str) -> String {
    if slug == "new" || is_id(slug) {
        format!("~{slug}")
    } else {
        slug.to_owned()
    }
}

fn key(value: &str) -> Result<AddressKey, Error> {
    if let Some(slug) = value.strip_prefix('~') {
        if slug == "new" || is_id(slug) {
            return Ok(AddressKey::Slug(slug.to_owned()));
        }
        return Err(Error::NotFound);
    }
    if is_id(value) {
        Ok(AddressKey::Id(value.to_owned()))
    } else if !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        Ok(AddressKey::Slug(value.to_owned()))
    } else {
        Err(Error::NotFound)
    }
}

// Decode each segment exactly once. Never let an encoded slash change scope.
fn decode(value: &str) -> Result<String, Error> {
    let mut bytes = value.bytes();
    let mut result = String::new();
    while let Some(byte) = bytes.next() {
        let byte = if byte == b'%' {
            let hi = bytes.next().and_then(|b| (b as char).to_digit(16));
            let lo = bytes.next().and_then(|b| (b as char).to_digit(16));
            match (hi, lo) {
                (Some(hi), Some(lo)) => (hi * 16 + lo) as u8,
                _ => return Err(Error::NotFound),
            }
        } else {
            byte
        };
        if !(byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&byte)) {
            return Err(Error::NotFound);
        }
        result.push(byte as char);
    }
    if matches!(result.as_str(), "." | "..") {
        return Err(Error::NotFound);
    }
    Ok(result)
}

fn content_suffix(parts: &[&str]) -> bool {
    matches!(parts, [] | ["edit"] | ["history"])
        || matches!(parts, ["history", revision] if !revision.is_empty())
}

enum Object<'a> {
    Species(&'a str),
    Form(&'a str, &'a str),
    LegacyForm(&'a str),
    World(&'a str),
    Variant(&'a str, &'a str),
    Type(&'a str),
    WorldType(&'a str, &'a str),
}

struct Address<'a> {
    object: Object<'a>,
    suffix: &'a [&'a str],
}

fn classify<'a>(parts: &'a [&'a str]) -> Option<Address<'a>> {
    let (object, suffix) = match parts {
        ["creatures", "new"] | ["worlds", "new"] | ["types", "new"] => return None,
        ["types", value, suffix @ ..] if content_suffix(suffix) => (Object::Type(value), suffix),
        ["creatures", species, "forms", "new"] => (Object::Species(species), &parts[2..]),
        ["creatures", species, "forms", form, suffix @ ..]
            if content_suffix(suffix) || matches!(suffix, ["typing"]) =>
        {
            (Object::Form(species, form), suffix)
        }
        ["creatures", species, suffix @ ..] if content_suffix(suffix) => {
            (Object::Species(species), suffix)
        }
        ["forms", id, suffix @ ..] if content_suffix(suffix) || matches!(suffix, ["typing"]) => {
            (Object::LegacyForm(id), suffix)
        }
        ["worlds", world, "types", "new"] => (Object::World(world), &parts[2..]),
        ["worlds", world, "types", value, suffix @ ..]
            if content_suffix(suffix) || matches!(suffix, ["adopt"]) =>
        {
            (Object::WorldType(world, value), suffix)
        }
        ["worlds", world, "variants", "new"] => (Object::World(world), &parts[2..]),
        ["worlds", world, "variants", variant, suffix @ ..]
            if content_suffix(suffix) || matches!(suffix, ["adopt"] | ["typing"]) =>
        {
            (Object::Variant(world, variant), suffix)
        }
        ["worlds", world, suffix @ ..]
            if content_suffix(suffix) || matches!(suffix, ["access"]) =>
        {
            (Object::World(world), suffix)
        }
        _ => return None,
    };
    Some(Address { object, suffix })
}

async fn resolve(state: &AppState, address: Address<'_>) -> Result<(String, String), Error> {
    let (pretty, inner) = match address.object {
        Object::Type(value) => {
            let row = state.types.resolve_type(key(value)?).await?;
            (
                format!("/types/{}", segment(&row.fields.slug)),
                format!("/types/{}", row.id),
            )
        }
        Object::WorldType(parent, value) => {
            let (parent, row) = state
                .types
                .resolve_world_type(key(parent)?, key(value)?)
                .await?;
            (
                format!(
                    "/worlds/{}/types/{}",
                    segment(&parent.fields.slug),
                    segment(&row.fields.slug)
                ),
                format!("/worlds/{}/types/{}", parent.id, row.id),
            )
        }
        Object::Species(value) => {
            let row = state.authoring.resolve_species(key(value)?).await?;
            (
                format!("/creatures/{}", segment(&row.fields.slug)),
                format!("/creatures/{}", row.id),
            )
        }
        Object::Form(parent, value) => {
            let (parent, row) = state
                .authoring
                .resolve_form(key(parent)?, key(value)?)
                .await?;
            (
                format!(
                    "/creatures/{}/forms/{}",
                    segment(&parent.fields.slug),
                    segment(&row.fields.slug)
                ),
                format!("/forms/{}", row.id),
            )
        }
        Object::LegacyForm(value) => {
            if !is_id(value) {
                return Err(Error::NotFound);
            }
            let (parent, row) = state.authoring.resolve_legacy_form(value).await?;
            (
                format!(
                    "/creatures/{}/forms/{}",
                    segment(&parent.fields.slug),
                    segment(&row.fields.slug)
                ),
                format!("/forms/{}", row.id),
            )
        }
        Object::World(value) => {
            let row = state.authoring.resolve_world(key(value)?).await?;
            (
                format!("/worlds/{}", segment(&row.fields.slug)),
                format!("/worlds/{}", row.id),
            )
        }
        Object::Variant(parent, value) => {
            let (parent, row) = state
                .authoring
                .resolve_variant(key(parent)?, key(value)?)
                .await?;
            (
                format!(
                    "/worlds/{}/variants/{}",
                    segment(&parent.fields.slug),
                    segment(&row.fields.slug)
                ),
                format!("/worlds/{}/variants/{}", parent.id, row.id),
            )
        }
    };
    let suffix = if address.suffix.is_empty() {
        String::new()
    } else {
        format!("/{}", address.suffix.join("/"))
    };
    Ok((format!("{pretty}{suffix}"), format!("{inner}{suffix}")))
}

async fn translate(state: &AppState, request: &mut Request) -> Result<Option<Response>, Error> {
    let path = request.uri().path();
    let raw: Vec<_> = path.strip_prefix('/').unwrap_or(path).split('/').collect();
    let Some(root) = raw.first().and_then(|s| decode(s).ok()) else {
        return Ok(None);
    };
    if !matches!(root.as_str(), "creatures" | "forms" | "worlds" | "types") {
        return Ok(None);
    }
    // Unknown shapes/suffixes retain the inner router's behavior.
    // Decode static segments as well, so accepted ASCII escapes normalize.
    let decoded: Result<Vec<_>, _> = std::iter::once(Ok(root))
        .chain(raw.iter().skip(1).map(|s| decode(s)))
        .collect();
    let decoded = match decoded {
        Ok(parts) => parts,
        Err(error) => {
            if classify(&raw).is_some() {
                return Err(error);
            }
            return Ok(None);
        }
    };
    let parts: Vec<_> = decoded.iter().map(String::as_str).collect();
    if matches!(
        parts.as_slice(),
        ["creatures", "new"] | ["worlds", "new"] | ["types", "new"]
    ) {
        let pretty = format!("/{}", parts.join("/"));
        if path != pretty {
            let query = request
                .uri()
                .query()
                .map(|q| format!("?{q}"))
                .unwrap_or_default();
            let mut response = Redirect::temporary(&format!("{pretty}{query}")).into_response();
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            return Ok(Some(response));
        }
        return Ok(None);
    }
    let Some(address) = classify(&parts) else {
        return Ok(None);
    };
    let (pretty, inner) = resolve(state, address).await?;
    let query = request
        .uri()
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();
    if path != pretty {
        let mut response = Redirect::temporary(&format!("{pretty}{query}")).into_response();
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        return Ok(Some(response));
    }
    *request.uri_mut() = format!("{inner}{query}")
        .parse::<Uri>()
        .map_err(|_| Error::NotFound)?;
    Ok(None)
}

pub(super) async fn adapt(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return next.run(request).await;
    }
    let head = request.method() == Method::HEAD;
    let mut response = match translate(&state, &mut request).await {
        Ok(Some(response)) => response,
        Ok(None) => next.run(request).await,
        Err(error) => authoring::no_store(authoring::read_error(error)),
    };
    // Outer errors bypass Axum's GET method router, which normally strips HEAD bodies.
    if head {
        *response.body_mut() = Body::empty();
    }
    response
}
