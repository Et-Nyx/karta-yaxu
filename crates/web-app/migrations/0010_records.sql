-- Additive evidence only. Existing native/scalar bytes and audit events are untouched.
CREATE TABLE card_relation_legacy_boundaries (
 namespace TEXT NOT NULL, scope TEXT NOT NULL, id TEXT NOT NULL,
 maximum_revision INTEGER NOT NULL CHECK(maximum_revision>0), PRIMARY KEY(namespace,scope,id)
) STRICT;
INSERT INTO card_relation_legacy_boundaries SELECT 'form','',form_id,max(revision) FROM form_versions GROUP BY form_id;
INSERT INTO card_relation_legacy_boundaries SELECT 'variant',world_id,variant_id,max(revision) FROM variant_revisions GROUP BY world_id,variant_id;
INSERT INTO card_relation_legacy_boundaries SELECT 'type','',type_id,max(revision) FROM type_versions GROUP BY type_id;
INSERT INTO card_relation_legacy_boundaries SELECT 'type',world_id,type_id,max(revision) FROM world_type_versions GROUP BY world_id,type_id;
INSERT INTO card_relation_legacy_boundaries SELECT 'move',scope,move_id,max(revision) FROM move_versions GROUP BY scope,move_id;
INSERT INTO card_relation_legacy_boundaries SELECT 'standalone',scope,card_id,max(revision) FROM standalone_card_versions GROUP BY scope,card_id;

CREATE TABLE record_layouts (
 scope TEXT NOT NULL, id TEXT NOT NULL, world_id TEXT REFERENCES worlds(id), slug TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision>0), PRIMARY KEY(scope,id), UNIQUE(scope,slug),
 CHECK(scope=coalesce(world_id,'')),
 FOREIGN KEY(scope,id,revision) REFERENCES record_layout_versions(scope,id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE record_layout_versions (
 scope TEXT NOT NULL, id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 source_id TEXT, source_revision INTEGER, slug TEXT NOT NULL, display_name TEXT NOT NULL, description TEXT NOT NULL,
 archived_at INTEGER, audit_id TEXT NOT NULL REFERENCES audit_events(id), previous_revision INTEGER,
 before_json TEXT, after_json TEXT NOT NULL, PRIMARY KEY(scope,id,revision),
 FOREIGN KEY(scope,id) REFERENCES record_layouts(scope,id) DEFERRABLE INITIALLY DEFERRED,
 CHECK((source_id IS NULL)=(source_revision IS NULL)), CHECK(source_id IS NULL OR scope<>'')
) STRICT;
CREATE TABLE record_layout_columns (
 scope TEXT NOT NULL, layout_id TEXT NOT NULL, revision INTEGER NOT NULL,
 slot INTEGER NOT NULL CHECK(slot>=0), column_id TEXT NOT NULL, label TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('field','reference')), field_id TEXT, field_revision INTEGER,
 PRIMARY KEY(scope,layout_id,revision,slot), UNIQUE(scope,layout_id,revision,column_id),
 FOREIGN KEY(scope,layout_id,revision) REFERENCES record_layout_versions(scope,id,revision) DEFERRABLE INITIALLY DEFERRED,
 FOREIGN KEY(scope,field_id,field_revision) REFERENCES card_field_versions(scope,field_id,revision) DEFERRABLE INITIALLY DEFERRED,
 CHECK((kind='field')=(field_id IS NOT NULL)), CHECK((kind='field')=(field_revision IS NOT NULL))
) STRICT;
CREATE TABLE record_column_reference_kinds (
 scope TEXT NOT NULL, layout_id TEXT NOT NULL, revision INTEGER NOT NULL, column_slot INTEGER NOT NULL,
 slot INTEGER NOT NULL CHECK(slot>=0), kind TEXT NOT NULL CHECK(kind IN ('card','stat','field')),
 PRIMARY KEY(scope,layout_id,revision,column_slot,slot), UNIQUE(scope,layout_id,revision,column_slot,kind),
 FOREIGN KEY(scope,layout_id,revision,column_slot) REFERENCES record_layout_columns(scope,layout_id,revision,slot) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE record_collection_definitions (
 scope TEXT NOT NULL, id TEXT NOT NULL, world_id TEXT REFERENCES worlds(id), slug TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision>0), PRIMARY KEY(scope,id), UNIQUE(scope,slug), CHECK(scope=coalesce(world_id,'')),
 FOREIGN KEY(scope,id,revision) REFERENCES record_collection_versions(scope,id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE record_collection_versions (
 scope TEXT NOT NULL, id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 source_id TEXT, source_revision INTEGER, slug TEXT NOT NULL, display_name TEXT NOT NULL, description TEXT NOT NULL,
 archived_at INTEGER, layout_id TEXT NOT NULL, layout_revision INTEGER NOT NULL,
 audit_id TEXT NOT NULL REFERENCES audit_events(id), previous_revision INTEGER, before_json TEXT, after_json TEXT NOT NULL,
 PRIMARY KEY(scope,id,revision),
 FOREIGN KEY(scope,id) REFERENCES record_collection_definitions(scope,id) DEFERRABLE INITIALLY DEFERRED,
 FOREIGN KEY(scope,layout_id,layout_revision) REFERENCES record_layout_versions(scope,id,revision) DEFERRABLE INITIALLY DEFERRED,
 CHECK((source_id IS NULL)=(source_revision IS NULL)), CHECK(source_id IS NULL OR scope<>'')
) STRICT;
CREATE TABLE card_relation_versions (
 namespace TEXT NOT NULL CHECK(namespace IN ('form','variant','type','move','standalone')),
 scope TEXT NOT NULL, card_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 collections_defined INTEGER NOT NULL CHECK(collections_defined IN (0,1)),
 audit_id TEXT REFERENCES audit_events(id), previous_revision INTEGER, before_json TEXT, snapshot_json TEXT,
 PRIMARY KEY(namespace,scope,card_id,revision), CHECK((audit_id IS NULL)=(snapshot_json IS NULL))
) STRICT;
CREATE TABLE card_relation_collections (
 namespace TEXT NOT NULL, scope TEXT NOT NULL, card_id TEXT NOT NULL, revision INTEGER NOT NULL,
 slot INTEGER NOT NULL CHECK(slot>=0), collection_id TEXT NOT NULL, collection_revision INTEGER NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('missing','unknown','not_applicable','entries')),
 PRIMARY KEY(namespace,scope,card_id,revision,slot), UNIQUE(namespace,scope,card_id,revision,collection_id),
 FOREIGN KEY(namespace,scope,card_id,revision) REFERENCES card_relation_versions(namespace,scope,card_id,revision) DEFERRABLE INITIALLY DEFERRED,
 FOREIGN KEY(scope,collection_id,collection_revision) REFERENCES record_collection_versions(scope,id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE card_relation_entries (
 namespace TEXT NOT NULL, scope TEXT NOT NULL, card_id TEXT NOT NULL, revision INTEGER NOT NULL, collection_slot INTEGER NOT NULL,
 slot INTEGER NOT NULL CHECK(slot>=0), entry_id TEXT NOT NULL,
 PRIMARY KEY(namespace,scope,card_id,revision,collection_slot,slot), UNIQUE(namespace,scope,card_id,revision,collection_slot,entry_id),
 FOREIGN KEY(namespace,scope,card_id,revision,collection_slot) REFERENCES card_relation_collections(namespace,scope,card_id,revision,slot) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE card_relation_cells (
 namespace TEXT NOT NULL, scope TEXT NOT NULL, card_id TEXT NOT NULL, revision INTEGER NOT NULL, collection_slot INTEGER NOT NULL, entry_slot INTEGER NOT NULL,
 slot INTEGER NOT NULL CHECK(slot>=0), column_id TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('missing','unknown','not_applicable','number','text','boolean','choice','reference')),
 coefficient INTEGER, scale INTEGER, text_value TEXT, boolean_value INTEGER CHECK(boolean_value IN (0,1)), option_id TEXT,
 ref_kind TEXT CHECK(ref_kind IN ('card','stat','field')), ref_namespace TEXT, ref_scope TEXT, ref_id TEXT, ref_revision INTEGER,
 PRIMARY KEY(namespace,scope,card_id,revision,collection_slot,entry_slot,slot), UNIQUE(namespace,scope,card_id,revision,collection_slot,entry_slot,column_id),
 FOREIGN KEY(namespace,scope,card_id,revision,collection_slot,entry_slot) REFERENCES card_relation_entries(namespace,scope,card_id,revision,collection_slot,slot) DEFERRABLE INITIALLY DEFERRED,
 CHECK((state='number')=(coefficient IS NOT NULL)), CHECK((state='number')=(scale IS NOT NULL)), CHECK(scale IS NULL OR scale BETWEEN 0 AND 6),
 CHECK((state='text')=(text_value IS NOT NULL)), CHECK((state='boolean')=(boolean_value IS NOT NULL)), CHECK((state='choice')=(option_id IS NOT NULL)),
 CHECK((state='reference')=(ref_kind IS NOT NULL)), CHECK((state='reference')=(ref_scope IS NOT NULL)), CHECK((state='reference')=(ref_id IS NOT NULL)), CHECK((state='reference')=(ref_revision IS NOT NULL)),
 CHECK((ref_kind IS 'card')=(ref_namespace IS NOT NULL)), CHECK(ref_revision IS NULL OR ref_revision>0)
) STRICT;

CREATE TRIGGER relation_boundary_insert BEFORE INSERT ON card_relation_legacy_boundaries BEGIN SELECT RAISE(ABORT,'fixed record boundary'); END;
CREATE TRIGGER relation_boundary_update BEFORE UPDATE ON card_relation_legacy_boundaries BEGIN SELECT RAISE(ABORT,'fixed record boundary'); END;
CREATE TRIGGER relation_boundary_delete BEFORE DELETE ON card_relation_legacy_boundaries BEGIN SELECT RAISE(ABORT,'fixed record boundary'); END;
CREATE TRIGGER relation_legacy_insert BEFORE INSERT ON card_relation_versions WHEN EXISTS(SELECT 1 FROM card_relation_legacy_boundaries WHERE namespace=NEW.namespace AND scope=NEW.scope AND id=NEW.card_id AND maximum_revision>=NEW.revision) BEGIN SELECT RAISE(ABORT,'legacy records remain unmodeled'); END;
CREATE TRIGGER record_layout_identity BEFORE UPDATE OF scope,id,world_id ON record_layouts WHEN NEW.scope<>OLD.scope OR NEW.id<>OLD.id OR NEW.world_id IS NOT OLD.world_id BEGIN SELECT RAISE(ABORT,'immutable layout identity'); END;
CREATE TRIGGER record_collection_identity BEFORE UPDATE OF scope,id,world_id ON record_collection_definitions WHEN NEW.scope<>OLD.scope OR NEW.id<>OLD.id OR NEW.world_id IS NOT OLD.world_id BEGIN SELECT RAISE(ABORT,'immutable collection identity'); END;
CREATE TRIGGER record_layout_advance BEFORE UPDATE OF revision ON record_layouts WHEN NEW.revision<>OLD.revision+1 BEGIN SELECT RAISE(ABORT,'layout must advance once'); END;
CREATE TRIGGER record_collection_advance BEFORE UPDATE OF revision ON record_collection_definitions WHEN NEW.revision<>OLD.revision+1 BEGIN SELECT RAISE(ABORT,'collection must advance once'); END;
CREATE TRIGGER record_layout_versions_update BEFORE UPDATE ON record_layout_versions BEGIN SELECT RAISE(ABORT,'immutable layout revision'); END;
CREATE TRIGGER record_layout_versions_delete BEFORE DELETE ON record_layout_versions BEGIN SELECT RAISE(ABORT,'immutable layout revision'); END;
CREATE TRIGGER record_collection_versions_update BEFORE UPDATE ON record_collection_versions BEGIN SELECT RAISE(ABORT,'immutable collection revision'); END;
CREATE TRIGGER record_collection_versions_delete BEFORE DELETE ON record_collection_versions BEGIN SELECT RAISE(ABORT,'immutable collection revision'); END;
CREATE TRIGGER record_layout_columns_insert BEFORE INSERT ON record_layout_columns WHEN EXISTS(SELECT 1 FROM record_layout_versions WHERE scope=NEW.scope AND id=NEW.layout_id AND revision=NEW.revision) BEGIN SELECT RAISE(ABORT,'sealed layout columns'); END;
CREATE TRIGGER record_layout_columns_update BEFORE UPDATE ON record_layout_columns BEGIN SELECT RAISE(ABORT,'immutable layout columns'); END;
CREATE TRIGGER record_layout_columns_delete BEFORE DELETE ON record_layout_columns BEGIN SELECT RAISE(ABORT,'immutable layout columns'); END;
CREATE TRIGGER record_reference_kinds_insert BEFORE INSERT ON record_column_reference_kinds WHEN EXISTS(SELECT 1 FROM record_layout_versions WHERE scope=NEW.scope AND id=NEW.layout_id AND revision=NEW.revision) BEGIN SELECT RAISE(ABORT,'sealed reference kinds'); END;
CREATE TRIGGER record_reference_kinds_update BEFORE UPDATE ON record_column_reference_kinds BEGIN SELECT RAISE(ABORT,'immutable reference kinds'); END;
CREATE TRIGGER record_reference_kinds_delete BEFORE DELETE ON record_column_reference_kinds BEGIN SELECT RAISE(ABORT,'immutable reference kinds'); END;
CREATE TRIGGER card_relation_versions_update BEFORE UPDATE ON card_relation_versions WHEN OLD.audit_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'sealed card records'); END;
CREATE TRIGGER card_relation_versions_delete BEFORE DELETE ON card_relation_versions WHEN OLD.audit_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'sealed card records'); END;
CREATE TRIGGER card_relation_collections_insert BEFORE INSERT ON card_relation_collections WHEN EXISTS(SELECT 1 FROM card_relation_versions WHERE namespace=NEW.namespace AND scope=NEW.scope AND card_id=NEW.card_id AND revision=NEW.revision AND audit_id IS NOT NULL) BEGIN SELECT RAISE(ABORT,'sealed card collections'); END;
CREATE TRIGGER card_relation_collections_update BEFORE UPDATE ON card_relation_collections BEGIN SELECT RAISE(ABORT,'immutable card collections'); END;
CREATE TRIGGER card_relation_collections_delete BEFORE DELETE ON card_relation_collections BEGIN SELECT RAISE(ABORT,'immutable card collections'); END;
CREATE TRIGGER card_relation_entries_insert BEFORE INSERT ON card_relation_entries WHEN EXISTS(SELECT 1 FROM card_relation_versions WHERE namespace=NEW.namespace AND scope=NEW.scope AND card_id=NEW.card_id AND revision=NEW.revision AND audit_id IS NOT NULL) BEGIN SELECT RAISE(ABORT,'sealed record entries'); END;
CREATE TRIGGER card_relation_entries_update BEFORE UPDATE ON card_relation_entries BEGIN SELECT RAISE(ABORT,'immutable record entries'); END;
CREATE TRIGGER card_relation_entries_delete BEFORE DELETE ON card_relation_entries BEGIN SELECT RAISE(ABORT,'immutable record entries'); END;
CREATE TRIGGER card_relation_cells_insert BEFORE INSERT ON card_relation_cells WHEN EXISTS(SELECT 1 FROM card_relation_versions WHERE namespace=NEW.namespace AND scope=NEW.scope AND card_id=NEW.card_id AND revision=NEW.revision AND audit_id IS NOT NULL) BEGIN SELECT RAISE(ABORT,'sealed record cells'); END;
CREATE TRIGGER card_relation_cells_update BEFORE UPDATE ON card_relation_cells BEGIN SELECT RAISE(ABORT,'immutable record cells'); END;
CREATE TRIGGER card_relation_cells_delete BEFORE DELETE ON card_relation_cells BEGIN SELECT RAISE(ABORT,'immutable record cells'); END;
