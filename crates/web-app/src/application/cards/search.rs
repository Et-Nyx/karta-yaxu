//! Bounded public selectors. Returned coordinates are exact pins, never latest aliases.
use super::*;
use crate::persistence::card_search as search;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArchiveFilter {
    #[default]
    All,
    Active,
    Archived,
}
impl ArchiveFilter {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKindTag {
    Number,
    Text,
    Boolean,
    Choice,
    CardReference,
}
impl FieldKindTag {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Text => "text",
            Self::Boolean => "boolean",
            Self::Choice => "choice",
            Self::CardReference => "card_reference",
        }
    }
}
impl From<&FieldKind> for FieldKindTag {
    fn from(kind: &FieldKind) -> Self {
        match kind {
            FieldKind::Number(_) => Self::Number,
            FieldKind::Text { .. } => Self::Text,
            FieldKind::Boolean => Self::Boolean,
            FieldKind::Choice { .. } => Self::Choice,
            FieldKind::CardReference => Self::CardReference,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardSearch {
    pub query: String,
    pub label: String,
    pub namespace: Option<CardNamespace>,
    pub archive: ArchiveFilter,
    pub page: usize,
}
impl Default for CardSearch {
    fn default() -> Self {
        Self {
            query: String::new(),
            label: String::new(),
            namespace: None,
            archive: ArchiveFilter::All,
            page: 1,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSearch {
    pub query: String,
    pub kind: Option<FieldKindTag>,
    pub archive: ArchiveFilter,
    pub page: usize,
}
impl Default for FieldSearch {
    fn default() -> Self {
        Self {
            query: String::new(),
            kind: None,
            archive: ArchiveFilter::All,
            page: 1,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchPage<T> {
    pub items: Vec<T>,
    pub has_more: bool,
    /// Catalog/option page; revision cursor results always report page 1.
    pub page: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardSummary {
    pub pin: CardPin,
    pub fields: ContentFields,
    pub archived_at: Option<i64>,
    /// Current identity eligibility, independent of the exact revision above.
    pub current_archived_at: Option<i64>,
    pub source: Option<CardPin>,
    /// First 25 labels in authored order, with the full count below.
    pub labels: Vec<String>,
    pub label_count: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSummary {
    pub pin: FieldPin,
    pub fields: ContentFields,
    pub archived_at: Option<i64>,
    pub current_archived_at: Option<i64>,
    pub source: Option<FieldPin>,
    pub kind: FieldKindTag,
}
pub(crate) fn card_summary(row: CardRecord, current_archived_at: Option<i64>) -> CardSummary {
    let labels = row.component.labels.unwrap_or_default();
    CardSummary {
        pin: CardPin {
            identity: row.identity,
            version: row.revision,
        },
        fields: row.fields,
        archived_at: row.archived_at,
        current_archived_at,
        source: row.source,
        label_count: labels.len(),
        labels: labels.into_iter().take(25).collect(),
    }
}
pub(crate) fn field_summary(
    row: FieldDefinition,
    current_archived_at: Option<i64>,
) -> FieldSummary {
    FieldSummary {
        pin: row.pin(),
        kind: FieldKindTag::from(&row.kind),
        fields: row.fields,
        archived_at: row.archived_at,
        current_archived_at,
        source: row.source,
    }
}
fn validate_query(query: &str, page: usize) -> Result<(), Error> {
    if query.chars().count() > 100 {
        return Err(author::invalid(
            "query",
            "Use at most 100 characters for a name search.",
        ));
    }
    if !(1..=10000).contains(&page) {
        return Err(author::invalid("page", "Choose a page from 1 to 10000."));
    }
    Ok(())
}
fn validate_cursor(before: Option<i64>) -> Result<(), Error> {
    if before.is_some_and(|n| n <= 0) {
        return Err(author::invalid(
            "before",
            "Choose a positive revision cursor.",
        ));
    }
    Ok(())
}
impl Cards {
    pub async fn search_cards(
        &self,
        world: Option<&str>,
        query: &CardSearch,
    ) -> Result<SearchPage<CardSummary>, Error> {
        validate_query(&query.query, query.page)?;
        if query.label.chars().count() > 120 {
            return Err(author::invalid(
                "label",
                "Use at most 120 characters for an exact label.",
            ));
        }
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        search::cards(&mut tx, world, query).await
    }
    pub async fn search_fields(
        &self,
        world: Option<&str>,
        query: &FieldSearch,
    ) -> Result<SearchPage<FieldSummary>, Error> {
        validate_query(&query.query, query.page)?;
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        search::fields(&mut tx, world, None, query).await
    }
    pub async fn mapping_fields(
        &self,
        world: &str,
        shared_field_id: &str,
        query: &FieldSearch,
    ) -> Result<SearchPage<FieldSummary>, Error> {
        validate_query(&query.query, query.page)?;
        let mut tx = self.read().await?;
        content::world(&mut tx, world).await?;
        store::field(&mut tx, None, shared_field_id).await?;
        search::fields(&mut tx, Some(world), Some(shared_field_id), query).await
    }
    pub async fn card_revisions(
        &self,
        identity: &CardIdentity,
        before: Option<i64>,
    ) -> Result<SearchPage<CardSummary>, Error> {
        validate_cursor(before)?;
        let mut tx = self.read().await?;
        let current = store::card(&mut tx, identity).await?;
        // Existing public audit readers check exact records and attribution. One
        // additional bounded read establishes has_more without scanning history.
        let entries = store::history(&mut tx, identity, before).await?;
        let has_more = if entries.len() == 25 {
            !store::history(&mut tx, identity, entries.last().map(|e| e.revision))
                .await?
                .is_empty()
        } else {
            false
        };
        let mut items = Vec::with_capacity(entries.len());
        for entry in entries {
            let row = store::card_version(
                &mut tx,
                &CardPin {
                    identity: identity.clone(),
                    version: entry.revision,
                },
            )
            .await?;
            items.push(card_summary(row, current.archived_at));
        }
        Ok(SearchPage {
            items,
            has_more,
            page: 1,
        })
    }
    pub async fn field_revisions(
        &self,
        world: Option<&str>,
        id: &str,
        before: Option<i64>,
    ) -> Result<SearchPage<FieldSummary>, Error> {
        validate_cursor(before)?;
        let mut tx = self.read().await?;
        let current = store::field(&mut tx, world, id).await?;
        let entries = store::field_history(&mut tx, world, id, before).await?;
        let has_more = if entries.len() == 25 {
            !store::field_history(&mut tx, world, id, entries.last().map(|e| e.revision))
                .await?
                .is_empty()
        } else {
            false
        };
        let mut items = Vec::with_capacity(entries.len());
        for entry in entries {
            let row = store::field_version(&mut tx, world, id, entry.revision).await?;
            items.push(field_summary(row, current.archived_at));
        }
        Ok(SearchPage {
            items,
            has_more,
            page: 1,
        })
    }
    pub async fn choice_options(
        &self,
        pin: &FieldPin,
        query: &str,
        page: usize,
    ) -> Result<SearchPage<ChoiceOption>, Error> {
        validate_query(query, page)?;
        pin.validate().map_err(Error::Invalid)?;
        let mut tx = self.read().await?;
        let definition =
            store::field_version(&mut tx, pin.world_id.as_deref(), &pin.field_id, pin.version)
                .await?;
        definition.validate().map_err(|_| Error::Unavailable)?;
        if !matches!(definition.kind, FieldKind::Choice { .. }) {
            return Err(author::invalid(
                "field",
                "Choose an exact Choice field definition.",
            ));
        }
        search::options(&mut tx, pin, query, page).await
    }
}
