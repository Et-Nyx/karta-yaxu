-- Compatibility bookkeeping only: no authored objects, actors, or audit events.
CREATE TABLE card_legacy_boundaries (
 namespace TEXT NOT NULL, scope TEXT NOT NULL, id TEXT NOT NULL,
 maximum_revision INTEGER NOT NULL CHECK(maximum_revision>0),
 PRIMARY KEY(namespace,scope,id)
) STRICT;
INSERT INTO card_legacy_boundaries SELECT 'form','',form_id,max(revision) FROM form_versions GROUP BY form_id;
INSERT INTO card_legacy_boundaries SELECT 'variant',world_id,variant_id,max(revision) FROM variant_revisions GROUP BY world_id,variant_id;
INSERT INTO card_legacy_boundaries SELECT 'type','',type_id,max(revision) FROM type_versions GROUP BY type_id;
INSERT INTO card_legacy_boundaries SELECT 'type',world_id,type_id,max(revision) FROM world_type_versions GROUP BY world_id,type_id;
INSERT INTO card_legacy_boundaries SELECT 'move',scope,move_id,max(revision) FROM move_versions GROUP BY scope,move_id;

CREATE TABLE card_field_definitions (
 scope TEXT NOT NULL, id TEXT NOT NULL, world_id TEXT REFERENCES worlds(id),
 slug TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(scope,id), UNIQUE(scope,slug),
 CHECK(scope=coalesce(world_id,'')),
 FOREIGN KEY(scope,id,revision) REFERENCES card_field_versions(scope,field_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE card_field_versions (
 scope TEXT NOT NULL, field_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 source_id TEXT, source_revision INTEGER,
 slug TEXT NOT NULL, display_name TEXT NOT NULL, description TEXT NOT NULL,
 archived_at INTEGER, kind TEXT NOT NULL CHECK(kind IN ('number','text','boolean','choice','card_reference')),
 whole_only INTEGER NOT NULL CHECK(whole_only IN (0,1)),
 minimum_coefficient INTEGER, minimum_scale INTEGER, maximum_coefficient INTEGER, maximum_scale INTEGER,
 max_length INTEGER,
 audit_id TEXT NOT NULL REFERENCES audit_events(id), previous_revision INTEGER,
 before_json TEXT, after_json TEXT NOT NULL,
 PRIMARY KEY(scope,field_id,revision),
 FOREIGN KEY(scope,field_id) REFERENCES card_field_definitions(scope,id) DEFERRABLE INITIALLY DEFERRED,
 CHECK((source_id IS NULL)=(source_revision IS NULL)),
 CHECK(source_id IS NULL OR scope<>''),
 CHECK((minimum_coefficient IS NULL)=(minimum_scale IS NULL)),
 CHECK((maximum_coefficient IS NULL)=(maximum_scale IS NULL)),
 CHECK(minimum_scale IS NULL OR minimum_scale BETWEEN 0 AND 6),
 CHECK(maximum_scale IS NULL OR maximum_scale BETWEEN 0 AND 6),
 CHECK((kind='text')=(max_length IS NOT NULL)),
 CHECK(max_length IS NULL OR max_length BETWEEN 0 AND 2000)
) STRICT;
CREATE TABLE card_field_options (
 scope TEXT NOT NULL, field_id TEXT NOT NULL, revision INTEGER NOT NULL,
 slot INTEGER NOT NULL CHECK(slot>=0), option_id TEXT NOT NULL, label TEXT NOT NULL,
 PRIMARY KEY(scope,field_id,revision,slot), UNIQUE(scope,field_id,revision,option_id),
 FOREIGN KEY(scope,field_id,revision) REFERENCES card_field_versions(scope,field_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE standalone_cards (
 scope TEXT NOT NULL, id TEXT NOT NULL, world_id TEXT REFERENCES worlds(id),
 slug TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 PRIMARY KEY(scope,id), UNIQUE(scope,slug), CHECK(scope=coalesce(world_id,'')),
 FOREIGN KEY(scope,id,revision) REFERENCES standalone_card_versions(scope,card_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE standalone_card_versions (
 scope TEXT NOT NULL, card_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 slug TEXT NOT NULL, display_name TEXT NOT NULL, description TEXT NOT NULL, archived_at INTEGER,
 source_id TEXT, source_revision INTEGER,
 audit_id TEXT NOT NULL REFERENCES audit_events(id), previous_revision INTEGER,
 before_json TEXT, after_json TEXT NOT NULL,
 PRIMARY KEY(scope,card_id,revision),
 FOREIGN KEY(scope,card_id) REFERENCES standalone_cards(scope,id) DEFERRABLE INITIALLY DEFERRED,
 CHECK((source_id IS NULL)=(source_revision IS NULL)), CHECK(source_id IS NULL OR scope<>'')
) STRICT;
-- New native revisions are staged then sealed with their owning native audit ID.
CREATE TABLE card_components (
 namespace TEXT NOT NULL CHECK(namespace IN ('form','variant','type','move','standalone')),
 scope TEXT NOT NULL, card_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 fields_defined INTEGER NOT NULL CHECK(fields_defined IN (0,1)),
 rules_text TEXT, labels_defined INTEGER NOT NULL CHECK(labels_defined IN (0,1)),
 audit_id TEXT REFERENCES audit_events(id), snapshot_json TEXT,
 PRIMARY KEY(namespace,scope,card_id,revision),
 CHECK((audit_id IS NULL)=(snapshot_json IS NULL))
) STRICT;
CREATE TABLE card_component_labels (
 namespace TEXT NOT NULL, scope TEXT NOT NULL, card_id TEXT NOT NULL, revision INTEGER NOT NULL,
 slot INTEGER NOT NULL CHECK(slot>=0), label TEXT NOT NULL,
 PRIMARY KEY(namespace,scope,card_id,revision,slot), UNIQUE(namespace,scope,card_id,revision,label),
 FOREIGN KEY(namespace,scope,card_id,revision) REFERENCES card_components(namespace,scope,card_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE card_component_fields (
 namespace TEXT NOT NULL, scope TEXT NOT NULL, card_id TEXT NOT NULL, revision INTEGER NOT NULL,
 slot INTEGER NOT NULL CHECK(slot>=0), field_id TEXT NOT NULL, field_revision INTEGER NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('missing','unknown','not_applicable','number','text','boolean','choice','card_reference')),
 coefficient INTEGER, scale INTEGER, text_value TEXT, boolean_value INTEGER CHECK(boolean_value IN (0,1)), option_id TEXT,
 ref_namespace TEXT, ref_scope TEXT, ref_id TEXT, ref_revision INTEGER,
 PRIMARY KEY(namespace,scope,card_id,revision,slot), UNIQUE(namespace,scope,card_id,revision,field_id),
 FOREIGN KEY(namespace,scope,card_id,revision) REFERENCES card_components(namespace,scope,card_id,revision) DEFERRABLE INITIALLY DEFERRED,
 FOREIGN KEY(scope,field_id,field_revision) REFERENCES card_field_versions(scope,field_id,revision) DEFERRABLE INITIALLY DEFERRED,
 CHECK((state='number')=(coefficient IS NOT NULL)), CHECK((state='number')=(scale IS NOT NULL)),
 CHECK(scale IS NULL OR scale BETWEEN 0 AND 6), CHECK((state='text')=(text_value IS NOT NULL)),
 CHECK((state='boolean')=(boolean_value IS NOT NULL)), CHECK((state='choice')=(option_id IS NOT NULL)),
 CHECK((state='card_reference')=(ref_namespace IS NOT NULL)), CHECK((state='card_reference')=(ref_scope IS NOT NULL)),
 CHECK((state='card_reference')=(ref_id IS NOT NULL)), CHECK((state='card_reference')=(ref_revision IS NOT NULL))
) STRICT;

CREATE TRIGGER card_legacy_boundaries_immutable_insert BEFORE INSERT ON card_legacy_boundaries BEGIN SELECT RAISE(ABORT,'fixed legacy card boundary'); END;
CREATE TRIGGER card_legacy_boundaries_immutable_update BEFORE UPDATE ON card_legacy_boundaries BEGIN SELECT RAISE(ABORT,'fixed legacy card boundary'); END;
CREATE TRIGGER card_legacy_boundaries_immutable_delete BEFORE DELETE ON card_legacy_boundaries BEGIN SELECT RAISE(ABORT,'fixed legacy card boundary'); END;
CREATE TRIGGER card_components_legacy_boundary BEFORE INSERT ON card_components
 WHEN EXISTS(SELECT 1 FROM card_legacy_boundaries WHERE namespace=NEW.namespace AND scope=NEW.scope AND id=NEW.card_id AND maximum_revision>=NEW.revision)
 BEGIN SELECT RAISE(ABORT,'legacy card components remain unmodeled'); END;
CREATE TRIGGER card_field_identity BEFORE UPDATE OF scope,id,world_id ON card_field_definitions
 WHEN NEW.scope<>OLD.scope OR NEW.id<>OLD.id OR NEW.world_id IS NOT OLD.world_id BEGIN SELECT RAISE(ABORT,'immutable field identity'); END;
CREATE TRIGGER standalone_card_identity BEFORE UPDATE OF scope,id,world_id ON standalone_cards
 WHEN NEW.scope<>OLD.scope OR NEW.id<>OLD.id OR NEW.world_id IS NOT OLD.world_id BEGIN SELECT RAISE(ABORT,'immutable standalone identity'); END;
CREATE TRIGGER card_field_advance BEFORE UPDATE OF revision ON card_field_definitions WHEN NEW.revision<>OLD.revision+1 BEGIN SELECT RAISE(ABORT,'field revision must advance once'); END;
CREATE TRIGGER standalone_card_advance BEFORE UPDATE OF revision ON standalone_cards WHEN NEW.revision<>OLD.revision+1 BEGIN SELECT RAISE(ABORT,'standalone revision must advance once'); END;

CREATE TRIGGER card_field_versions_immutable_update BEFORE UPDATE ON card_field_versions BEGIN SELECT RAISE(ABORT,'immutable field revision'); END;
CREATE TRIGGER card_field_versions_immutable_delete BEFORE DELETE ON card_field_versions BEGIN SELECT RAISE(ABORT,'immutable field revision'); END;
CREATE TRIGGER card_field_options_sealed_insert BEFORE INSERT ON card_field_options
 WHEN EXISTS(SELECT 1 FROM card_field_versions WHERE scope=NEW.scope AND field_id=NEW.field_id AND revision=NEW.revision)
 BEGIN SELECT RAISE(ABORT,'sealed field options'); END;
CREATE TRIGGER card_field_options_immutable_update BEFORE UPDATE ON card_field_options BEGIN SELECT RAISE(ABORT,'immutable field options'); END;
CREATE TRIGGER card_field_options_immutable_delete BEFORE DELETE ON card_field_options BEGIN SELECT RAISE(ABORT,'immutable field options'); END;
CREATE TRIGGER standalone_card_versions_immutable_update BEFORE UPDATE ON standalone_card_versions BEGIN SELECT RAISE(ABORT,'immutable standalone revision'); END;
CREATE TRIGGER standalone_card_versions_immutable_delete BEFORE DELETE ON standalone_card_versions BEGIN SELECT RAISE(ABORT,'immutable standalone revision'); END;
CREATE TRIGGER card_components_sealed_update BEFORE UPDATE ON card_components WHEN OLD.audit_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'sealed card component'); END;
CREATE TRIGGER card_components_sealed_delete BEFORE DELETE ON card_components WHEN OLD.audit_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'sealed card component'); END;
CREATE TRIGGER card_component_labels_sealed_insert BEFORE INSERT ON card_component_labels
 WHEN EXISTS(SELECT 1 FROM card_components WHERE namespace=NEW.namespace AND scope=NEW.scope AND card_id=NEW.card_id AND revision=NEW.revision AND audit_id IS NOT NULL)
 BEGIN SELECT RAISE(ABORT,'sealed card labels'); END;
CREATE TRIGGER card_component_labels_immutable_update BEFORE UPDATE ON card_component_labels BEGIN SELECT RAISE(ABORT,'immutable card labels'); END;
CREATE TRIGGER card_component_labels_immutable_delete BEFORE DELETE ON card_component_labels BEGIN SELECT RAISE(ABORT,'immutable card labels'); END;
CREATE TRIGGER card_component_fields_sealed_insert BEFORE INSERT ON card_component_fields
 WHEN EXISTS(SELECT 1 FROM card_components WHERE namespace=NEW.namespace AND scope=NEW.scope AND card_id=NEW.card_id AND revision=NEW.revision AND audit_id IS NOT NULL)
 BEGIN SELECT RAISE(ABORT,'sealed card fields'); END;
CREATE TRIGGER card_component_fields_immutable_update BEFORE UPDATE ON card_component_fields BEGIN SELECT RAISE(ABORT,'immutable card fields'); END;
CREATE TRIGGER card_component_fields_immutable_delete BEFORE DELETE ON card_component_fields BEGIN SELECT RAISE(ABORT,'immutable card fields'); END;
