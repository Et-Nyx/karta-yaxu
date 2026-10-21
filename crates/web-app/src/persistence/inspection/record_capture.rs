//! Record evidence shares its parent's read transaction, caches and budgets.
use super::*;
use crate::persistence::relations;
use game_data::relations::*;

impl Capture {
    async fn record_sizes(
        &mut self,
        c: &mut SqliteConnection,
        table: &str,
        id: &str,
        w: &str,
        rev: i64,
    ) -> Result<(), CaptureGap> {
        self.prepare_card_version(c, table, "id", w, id, rev).await
    }
    async fn prepare_layouts(
        &mut self,
        c: &mut SqliteConnection,
        first: BTreeSet<LayoutPin>,
    ) -> Result<(), CaptureGap> {
        let mut pending: BTreeSet<_> = first.into_iter().map(|p| (p, true)).collect();
        let mut fields = BTreeSet::new();
        let mut visited = BTreeSet::new();
        while let Some((p, full)) = pending.pop_first() {
            if !visited.insert((p.clone(), full)) {
                continue;
            }
            p.validate()
                .map_err(|_| self.card_gap("Invalid layout pin."))?;
            if full && p.version > 1 {
                pending.insert((
                    LayoutPin {
                        version: p.version - 1,
                        ..p.clone()
                    },
                    false,
                ));
            }
            if !self.layout_preflight.insert(p.clone()) {
                continue;
            }
            self.card_node_budget()?;
            let w = p.world_id.as_deref().unwrap_or("");
            let head: i64 =
                sqlx::query_scalar("SELECT revision FROM record_layouts WHERE scope=? AND id=?")
                    .bind(w)
                    .bind(&p.layout_id)
                    .fetch_one(&mut *c)
                    .await
                    .map_err(|_| self.card_gap("Missing layout head."))?;
            if head < p.version {
                return Err(self.card_gap("Layout pin is newer than its head."));
            }
            pending.insert((
                LayoutPin {
                    version: head,
                    ..p.clone()
                },
                true,
            ));
            self.record_sizes(c, "record_layout_versions", &p.layout_id, w, p.version)
                .await?;
            for table in ["record_layout_columns", "record_column_reference_kinds"] {
                let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "SELECT count(*) FROM {table} WHERE scope=? AND layout_id=? AND revision=?"
                )))
                .bind(w)
                .bind(&p.layout_id)
                .bind(p.version)
                .fetch_one(&mut *c)
                .await
                .map_err(|_| self.card_gap("Unreadable layout children."))?;
                self.card_count(count)?;
            }
            let rows:Vec<(String,i64)>=sqlx::query_as("SELECT field_id,field_revision FROM record_layout_columns WHERE scope=? AND layout_id=? AND revision=? AND kind='field' ORDER BY slot").bind(w).bind(&p.layout_id).bind(p.version).fetch_all(&mut*c).await.map_err(|_|self.card_gap("Invalid layout scalar pins."))?;
            fields.extend(rows.into_iter().map(|(field_id, version)| FieldPin {
                world_id: p.world_id.clone(),
                field_id,
                version,
            }));
            let source:(Option<String>,Option<i64>)=sqlx::query_as("SELECT source_id,source_revision FROM record_layout_versions WHERE scope=? AND id=? AND revision=?").bind(w).bind(&p.layout_id).bind(p.version).fetch_one(&mut*c).await.map_err(|_|self.card_gap("Missing layout revision."))?;
            match source {
                (Some(layout_id), Some(version)) => {
                    pending.insert((
                        LayoutPin {
                            world_id: None,
                            layout_id,
                            version,
                        },
                        true,
                    ));
                }
                (None, None) => {}
                _ => return Err(self.card_gap("Invalid layout source.")),
            }
            if pending.len() + fields.len() > self.limits.max_nodes {
                return Err(budget(None, "Layout pending-input budget reached."));
            }
        }
        self.prepare_fields(c, fields).await
    }
    async fn prepare_collections(
        &mut self,
        c: &mut SqliteConnection,
        first: BTreeSet<CollectionPin>,
    ) -> Result<(), CaptureGap> {
        let mut pending: BTreeSet<_> = first.into_iter().map(|p| (p, true)).collect();
        let mut layouts = BTreeSet::new();
        let mut visited = BTreeSet::new();
        while let Some((p, full)) = pending.pop_first() {
            if !visited.insert((p.clone(), full)) {
                continue;
            }
            p.validate()
                .map_err(|_| self.card_gap("Invalid collection pin."))?;
            if full && p.version > 1 {
                pending.insert((
                    CollectionPin {
                        version: p.version - 1,
                        ..p.clone()
                    },
                    false,
                ));
            }
            if !self.collection_preflight.insert(p.clone()) {
                continue;
            }
            self.card_node_budget()?;
            let w = p.world_id.as_deref().unwrap_or("");
            let head: i64 = sqlx::query_scalar(
                "SELECT revision FROM record_collection_definitions WHERE scope=? AND id=?",
            )
            .bind(w)
            .bind(&p.collection_id)
            .fetch_one(&mut *c)
            .await
            .map_err(|_| self.card_gap("Missing collection head."))?;
            if head < p.version {
                return Err(self.card_gap("Collection pin is newer than its head."));
            }
            pending.insert((
                CollectionPin {
                    version: head,
                    ..p.clone()
                },
                true,
            ));
            self.record_sizes(
                c,
                "record_collection_versions",
                &p.collection_id,
                w,
                p.version,
            )
            .await?;
            let (layout_id,version,source_id,source_revision):(String,i64,Option<String>,Option<i64>)=sqlx::query_as("SELECT layout_id,layout_revision,source_id,source_revision FROM record_collection_versions WHERE scope=? AND id=? AND revision=?").bind(w).bind(&p.collection_id).bind(p.version).fetch_one(&mut*c).await.map_err(|_|self.card_gap("Invalid collection revision."))?;
            layouts.insert(LayoutPin {
                world_id: p.world_id.clone(),
                layout_id,
                version,
            });
            match (source_id, source_revision) {
                (Some(collection_id), Some(version)) => {
                    pending.insert((
                        CollectionPin {
                            world_id: None,
                            collection_id,
                            version,
                        },
                        true,
                    ));
                }
                (None, None) => {}
                _ => return Err(self.card_gap("Invalid collection source.")),
            }
            if pending.len() + layouts.len() > self.limits.max_nodes {
                return Err(budget(None, "Collection pending-input budget reached."));
            }
        }
        self.prepare_layouts(c, layouts).await
    }
    pub(super) async fn prepare_record_inputs(
        &mut self,
        c: &mut SqliteConnection,
        p: &CardPin,
        cards: &mut BTreeSet<CardPin>,
        fields: &mut BTreeSet<FieldPin>,
    ) -> Result<(), CaptureGap> {
        let mut collections = BTreeSet::new();
        for version in std::iter::once(p.version).chain((p.version > 1).then_some(p.version - 1)) {
            let p = CardPin {
                version,
                ..p.clone()
            };
            if !self.record_preflight.insert(p.clone()) {
                continue;
            }
            self.card_node_budget()?;
            let i = &p.identity;
            let w = i.world_id.as_deref().unwrap_or("");
            let sizes:Option<(Option<i64>,Option<i64>)>=sqlx::query_as("SELECT length(CAST(before_json AS BLOB)),length(CAST(snapshot_json AS BLOB)) FROM card_relation_versions WHERE namespace=? AND scope=? AND card_id=? AND revision=?").bind(i.namespace.as_str()).bind(w).bind(&i.id).bind(version).fetch_optional(&mut*c).await.map_err(|_|self.card_gap("Unreadable record companion."))?;
            if let Some((before, after)) = sizes {
                self.card_size(after.ok_or_else(|| self.card_gap("Unsealed record companion."))?)?;
                if let Some(before) = before {
                    self.card_size(before)?;
                }
            } else {
                let legacy:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM card_relation_legacy_boundaries WHERE namespace=? AND scope=? AND id=? AND maximum_revision>=?)").bind(i.namespace.as_str()).bind(w).bind(&i.id).bind(version).fetch_one(&mut*c).await.map_err(|_|self.card_gap("Unreadable record legacy boundary."))?;
                if !legacy {
                    return Err(
                        self.card_gap("Required same-revision record companion is missing.")
                    );
                }
            }
            for table in [
                "card_relation_collections",
                "card_relation_entries",
                "card_relation_cells",
            ] {
                let count:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table} WHERE namespace=? AND scope=? AND card_id=? AND revision=?"))).bind(i.namespace.as_str()).bind(w).bind(&i.id).bind(version).fetch_one(&mut*c).await.map_err(|_|self.card_gap("Unreadable record children."))?;
                self.card_count(count)?;
                if sizes.is_none() && count != 0 {
                    return Err(self.card_gap("Orphan record children."));
                }
            }
            let defs:Vec<(String,i64)>=sqlx::query_as("SELECT collection_id,collection_revision FROM card_relation_collections WHERE namespace=? AND scope=? AND card_id=? AND revision=? ORDER BY slot").bind(i.namespace.as_str()).bind(w).bind(&i.id).bind(version).fetch_all(&mut*c).await.map_err(|_|self.card_gap("Invalid record collection pins."))?;
            collections.extend(
                defs.into_iter()
                    .map(|(collection_id, version)| CollectionPin {
                        world_id: i.world_id.clone(),
                        collection_id,
                        version,
                    }),
            );
            let refs:Vec<(String,Option<String>,String,String,i64)>=sqlx::query_as("SELECT ref_kind,ref_namespace,ref_scope,ref_id,ref_revision FROM card_relation_cells WHERE namespace=? AND scope=? AND card_id=? AND revision=? AND state='reference' ORDER BY collection_slot,entry_slot,slot").bind(i.namespace.as_str()).bind(w).bind(&i.id).bind(version).fetch_all(&mut*c).await.map_err(|_|self.card_gap("Invalid record target pins."))?;
            for (kind, ns, w, id, version) in refs {
                let world_id = (!w.is_empty()).then_some(w);
                match kind.as_str() {
                    "card" => {
                        let namespace = match ns.as_deref() {
                            Some("form") => CardNamespace::Form,
                            Some("variant") => CardNamespace::Variant,
                            Some("type") => CardNamespace::Type,
                            Some("move") => CardNamespace::Move,
                            Some("standalone") => CardNamespace::Standalone,
                            _ => return Err(self.card_gap("Invalid record Card origin.")),
                        };
                        cards.insert(CardPin {
                            identity: CardIdentity {
                                namespace,
                                world_id,
                                id,
                            },
                            version,
                        });
                    }
                    "field" => {
                        fields.insert(FieldPin {
                            world_id,
                            field_id: id,
                            version,
                        });
                    }
                    "stat" => {
                        let k = key(DomainKind::Stat, world_id.as_deref(), &id, version);
                        self.prepare(c, &k).await?;
                    }
                    _ => return Err(self.card_gap("Unsupported record reference kind.")),
                }
            }
            if cards.len() + fields.len() + collections.len() > self.limits.max_nodes {
                return Err(budget(None, "Record pending-input budget reached."));
            }
        }
        self.prepare_collections(c, collections).await
    }
    pub(super) async fn capture_records(
        &mut self,
        c: &mut SqliteConnection,
        exporting: bool,
        bindings: Option<&mut Vec<game_data::package::IdentityBinding>>,
    ) -> Result<(), Error> {
        if !self.result.gaps.is_empty() {
            return Ok(());
        }
        if let Err(gap) = self.record_closure(c, exporting, bindings).await {
            self.result.gaps.push(gap);
        }
        Ok(())
    }
    async fn record_closure(
        &mut self,
        c: &mut SqliteConnection,
        exporting: bool,
        mut bindings: Option<&mut Vec<game_data::package::IdentityBinding>>,
    ) -> Result<(), CaptureGap> {
        let w = self.result.world.id.clone();
        let mut layouts = BTreeSet::new();
        let mut collections = BTreeSet::new();
        for layout in [true, false] {
            let (head, versions) = if layout {
                ("record_layouts", "record_layout_versions")
            } else {
                (
                    "record_collection_definitions",
                    "record_collection_versions",
                )
            };
            let used = self.result.roots.len()
                + self.result.card_data.card_roots.len()
                + self.result.card_data.field_roots.len()
                + self.result.card_data.records.layout_roots.len()
                + self.result.card_data.records.collection_roots.len();
            let remaining = self.limits.max_roots.saturating_sub(used);
            let active = if exporting {
                format!(
                    "AND NOT EXISTS(SELECT 1 FROM {versions} v WHERE v.scope=h.scope AND v.id=h.id AND v.revision=h.revision AND v.archived_at IS NOT NULL)"
                )
            } else {
                String::new()
            };
            let roots: Vec<(String, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT id,revision FROM {head} h WHERE scope=? {active} ORDER BY id LIMIT ?"
            )))
            .bind(&w)
            .bind(i64::try_from(remaining.saturating_add(1)).unwrap_or(i64::MAX))
            .fetch_all(&mut *c)
            .await
            .map_err(|_| self.card_gap("Unreadable current record inventory."))?;
            if roots.len() > remaining {
                return Err(budget(None, "Combined record root budget reached."));
            }
            self.card_count(roots.len() as i64)?;
            for (id, version) in roots {
                if layout {
                    let p = LayoutPin {
                        world_id: Some(w.clone()),
                        layout_id: id,
                        version,
                    };
                    layouts.insert(p.clone());
                    self.result.card_data.records.layout_roots.insert(p);
                } else {
                    let p = CollectionPin {
                        world_id: Some(w.clone()),
                        collection_id: id,
                        version,
                    };
                    collections.insert(p.clone());
                    self.result.card_data.records.collection_roots.insert(p);
                }
            }
        }
        loop {
            let mut work = CardWork::default();
            while let Some(p) = collections.pop_first() {
                if self.result.card_data.records.collections.contains_key(&p) {
                    continue;
                }
                self.prepare_collections(c, BTreeSet::from([p.clone()]))
                    .await?;
                let row = relations::collection_version(c, &p)
                    .await
                    .map_err(|_| self.card_gap("Exact collection evidence is inconsistent."))?;
                let head = relations::collection(c, p.world_id.as_deref(), &p.collection_id)
                    .await
                    .map_err(|_| self.card_gap("Collection head is inconsistent."))?;
                if !self
                    .result
                    .card_data
                    .records
                    .collection_heads
                    .iter()
                    .any(|x| x.world_id == p.world_id && x.collection_id == p.collection_id)
                {
                    self.result
                        .card_data
                        .records
                        .collection_heads
                        .push(head.pin());
                }
                layouts.insert(row.layout.pin());
                if let Some(s) = &row.source {
                    collections.insert(s.clone());
                }
                self.result.card_data.records.collections.insert(p, row);
            }
            while let Some(p) = layouts.pop_first() {
                if self.result.card_data.records.layouts.contains_key(&p) {
                    continue;
                }
                self.prepare_layouts(c, BTreeSet::from([p.clone()])).await?;
                let row = relations::layout_version(c, &p)
                    .await
                    .map_err(|_| self.card_gap("Exact layout evidence is inconsistent."))?;
                let head = relations::layout(c, p.world_id.as_deref(), &p.layout_id)
                    .await
                    .map_err(|_| self.card_gap("Layout head is inconsistent."))?;
                if !self
                    .result
                    .card_data
                    .records
                    .layout_heads
                    .iter()
                    .any(|x| x.world_id == p.world_id && x.layout_id == p.layout_id)
                {
                    self.result.card_data.records.layout_heads.push(head.pin());
                }
                if let Some(s) = &row.source {
                    layouts.insert(s.clone());
                }
                for column in &row.columns {
                    if let ColumnKind::Field(f) = &column.kind {
                        work.fields.insert(f.pin());
                    }
                }
                self.result.card_data.records.layouts.insert(p, row);
            }
            let pins = self
                .result
                .card_data
                .cards
                .keys()
                .filter(|p| !self.result.card_data.records.relations.contains_key(*p))
                .cloned()
                .collect::<Vec<_>>();
            for p in pins {
                let mut inputs = BTreeSet::new();
                let mut fields = BTreeSet::new();
                self.prepare_record_inputs(c, &p, &mut inputs, &mut fields)
                    .await?;
                // Verification companions above are deliberately not package nodes.
                let audit = cards::native_audit(c, &p)
                    .await
                    .map_err(|_| self.card_gap("Missing record audit evidence."))?;
                let row = relations::verify_seal(c, &p, &audit)
                    .await
                    .map_err(|_| self.card_gap("Same-event record companion is inconsistent."))?;
                for a in row.relations.collections.iter().flatten() {
                    collections.insert(a.definition.pin());
                    if let CollectionValue::Entries(es) = &a.value {
                        for e in es {
                            for cell in &e.cells {
                                if let RecordValue::Reference(t) = &cell.value {
                                    self.card_count(1)?;
                                    match t {
                                        DefinitionPin::Card(p) => {
                                            work.cards.insert(p.clone());
                                        }
                                        DefinitionPin::Field(p) => {
                                            work.fields.insert(p.clone());
                                        }
                                        DefinitionPin::Stat {
                                            world_id,
                                            stat_id,
                                            version,
                                        } => {
                                            work.typed.insert(key(
                                                DomainKind::Stat,
                                                world_id.as_deref(),
                                                stat_id,
                                                *version,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                self.result.card_data.records.relations.insert(p, row);
                self.card_node_budget()?;
            }
            work.cards
                .retain(|p| !self.result.card_data.cards.contains_key(p));
            work.fields
                .retain(|p| !self.result.card_data.fields.contains_key(p));
            work.typed.retain(|p| !self.result.nodes.contains_key(p));
            if work.cards.is_empty() && work.fields.is_empty() && work.typed.is_empty() {
                if collections.is_empty() && layouts.is_empty() {
                    break;
                }
            } else {
                self.extend_cards(c, exporting, &mut bindings, work).await?;
            }
            self.card_node_budget()?;
        }
        self.result
            .card_data
            .records
            .layout_heads
            .sort_by(|a, b| (&a.world_id, &a.layout_id).cmp(&(&b.world_id, &b.layout_id)));
        self.result
            .card_data
            .records
            .collection_heads
            .sort_by(|a, b| (&a.world_id, &a.collection_id).cmp(&(&b.world_id, &b.collection_id)));
        self.result
            .card_data
            .field_heads
            .sort_by(|a, b| (&a.world_id, &a.field_id).cmp(&(&b.world_id, &b.field_id)));
        Ok(())
    }
}
