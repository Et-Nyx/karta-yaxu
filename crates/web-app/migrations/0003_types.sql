-- Additive full-content seals. Existing v1 rows and JSON bytes are never rewritten.
ALTER TABLE form_versions ADD COLUMN content_snapshot_version INTEGER NOT NULL DEFAULT 1 CHECK(content_snapshot_version IN (1,2));
ALTER TABLE variant_revisions ADD COLUMN content_snapshot_version INTEGER NOT NULL DEFAULT 1 CHECK(content_snapshot_version IN (1,2));
ALTER TABLE world_revisions ADD COLUMN content_snapshot_version INTEGER NOT NULL DEFAULT 1 CHECK(content_snapshot_version IN (1,2));
CREATE UNIQUE INDEX form_content_key ON form_versions(form_id, revision, content_snapshot_version);
CREATE UNIQUE INDEX variant_content_key ON variant_revisions(world_id, variant_id, revision, content_snapshot_version);
CREATE UNIQUE INDEX world_content_key ON world_revisions(world_id, revision, content_snapshot_version);

CREATE TABLE types (
    id TEXT PRIMARY KEY NOT NULL,
    slug TEXT NOT NULL UNIQUE CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    revision INTEGER NOT NULL CHECK(revision > 0),
    archived_at INTEGER,
    FOREIGN KEY(id, revision) REFERENCES type_versions(type_id, revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE type_versions (
    type_id TEXT NOT NULL REFERENCES types(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    archived_at INTEGER,
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    previous_revision INTEGER,
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND json_extract(before_json,'$.snapshot_version') IS 1)),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 1),
    PRIMARY KEY(type_id, revision),
    FOREIGN KEY(type_id, previous_revision) REFERENCES type_versions(type_id, revision),
    CHECK((revision = 1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision > 1 AND previous_revision = revision-1 AND previous_revision IS NOT NULL AND before_json IS NOT NULL))
);
CREATE TABLE world_types (
    id TEXT PRIMARY KEY NOT NULL,
    world_id TEXT NOT NULL REFERENCES worlds(id),
    source_type_id TEXT,
    source_type_version INTEGER,
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    revision INTEGER NOT NULL CHECK(revision > 0),
    archived_at INTEGER,
    UNIQUE(world_id, id),
    UNIQUE(world_id, slug),
    CHECK((source_type_id IS NULL) = (source_type_version IS NULL)),
    FOREIGN KEY(source_type_id, source_type_version) REFERENCES type_versions(type_id, revision),
    FOREIGN KEY(world_id, id, revision) REFERENCES world_type_versions(world_id, type_id, revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE world_type_versions (
    world_id TEXT NOT NULL,
    type_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    source_type_id TEXT,
    source_type_version INTEGER,
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    archived_at INTEGER,
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    previous_revision INTEGER,
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND json_extract(before_json,'$.snapshot_version') IS 1)),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 1),
    PRIMARY KEY(world_id, type_id, revision),
    CHECK((source_type_id IS NULL) = (source_type_version IS NULL)),
    FOREIGN KEY(source_type_id, source_type_version) REFERENCES type_versions(type_id, revision),
    FOREIGN KEY(world_id, type_id) REFERENCES world_types(world_id, id),
    FOREIGN KEY(world_id, type_id, previous_revision) REFERENCES world_type_versions(world_id, type_id, revision),
    CHECK((revision = 1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision > 1 AND previous_revision = revision-1 AND previous_revision IS NOT NULL AND before_json IS NOT NULL))
);
CREATE INDEX world_type_history ON world_type_versions(type_id, revision);
CREATE TRIGGER type_identity BEFORE UPDATE OF id ON types WHEN NEW.id <> OLD.id BEGIN SELECT RAISE(ABORT,'immutable type'); END;
CREATE TRIGGER world_type_identity BEFORE UPDATE OF id, world_id ON world_types WHEN NEW.id <> OLD.id OR NEW.world_id <> OLD.world_id BEGIN SELECT RAISE(ABORT,'immutable type scope'); END;
CREATE TRIGGER type_versions_update BEFORE UPDATE ON type_versions BEGIN SELECT RAISE(ABORT,'immutable version'); END;
CREATE TRIGGER type_versions_delete BEFORE DELETE ON type_versions BEGIN SELECT RAISE(ABORT,'retained version'); END;
CREATE TRIGGER world_type_versions_update BEFORE UPDATE ON world_type_versions BEGIN SELECT RAISE(ABORT,'immutable version'); END;
CREATE TRIGGER world_type_versions_delete BEFORE DELETE ON world_type_versions BEGIN SELECT RAISE(ABORT,'retained version'); END;

CREATE TABLE form_content_snapshots (
    form_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    marker INTEGER NOT NULL DEFAULT 2 CHECK(marker = 2),
    typing_defined INTEGER NOT NULL CHECK(typing_defined IN (0,1)),
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND json_type(before_json,'$.snapshot_version') IS 'integer' AND (json_extract(before_json,'$.snapshot_version') IS 1 OR json_extract(before_json,'$.snapshot_version') IS 2))),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 2),
    PRIMARY KEY(form_id, revision),
    FOREIGN KEY(form_id, revision, marker) REFERENCES form_versions(form_id, revision, content_snapshot_version) DEFERRABLE INITIALLY DEFERRED,
    CHECK((revision = 1 AND before_json IS NULL) OR (revision > 1 AND before_json IS NOT NULL))
);
CREATE TABLE variant_content_snapshots (
    world_id TEXT NOT NULL,
    variant_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    marker INTEGER NOT NULL DEFAULT 2 CHECK(marker = 2),
    typing_defined INTEGER NOT NULL CHECK(typing_defined IN (0,1)),
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND json_type(before_json,'$.snapshot_version') IS 'integer' AND (json_extract(before_json,'$.snapshot_version') IS 1 OR json_extract(before_json,'$.snapshot_version') IS 2))),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 2),
    PRIMARY KEY(world_id, variant_id, revision),
    FOREIGN KEY(world_id, variant_id, revision, marker) REFERENCES variant_revisions(world_id, variant_id, revision, content_snapshot_version) DEFERRABLE INITIALLY DEFERRED,
    CHECK((revision = 1 AND before_json IS NULL) OR (revision > 1 AND before_json IS NOT NULL))
);
CREATE TABLE world_content_snapshots (
    world_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    marker INTEGER NOT NULL DEFAULT 2 CHECK(marker = 2),
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND json_type(before_json,'$.snapshot_version') IS 'integer' AND (json_extract(before_json,'$.snapshot_version') IS 1 OR json_extract(before_json,'$.snapshot_version') IS 2))),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 2),
    PRIMARY KEY(world_id, revision),
    FOREIGN KEY(world_id, revision, marker) REFERENCES world_revisions(world_id, revision, content_snapshot_version) DEFERRABLE INITIALLY DEFERRED,
    CHECK((revision = 1 AND before_json IS NULL) OR (revision > 1 AND before_json IS NOT NULL))
);
CREATE TABLE form_type_bindings (
    form_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    slot INTEGER NOT NULL CHECK(slot BETWEEN 1 AND 32),
    type_id TEXT NOT NULL,
    type_version INTEGER NOT NULL,
    PRIMARY KEY(form_id, revision, slot),
    UNIQUE(form_id, revision, type_id),
    FOREIGN KEY(type_id, type_version) REFERENCES type_versions(type_id, revision),
    FOREIGN KEY(form_id, revision) REFERENCES form_content_snapshots(form_id, revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE variant_type_bindings (
    world_id TEXT NOT NULL,
    variant_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    slot INTEGER NOT NULL CHECK(slot BETWEEN 1 AND 32),
    type_id TEXT NOT NULL,
    type_version INTEGER NOT NULL,
    PRIMARY KEY(world_id, variant_id, revision, slot),
    UNIQUE(world_id, variant_id, revision, type_id),
    FOREIGN KEY(world_id, type_id, type_version) REFERENCES world_type_versions(world_id, type_id, revision),
    FOREIGN KEY(world_id, variant_id, revision) REFERENCES variant_content_snapshots(world_id, variant_id, revision) DEFERRABLE INITIALLY DEFERRED
);

-- Seal only after bindings, and only before the exact marker-2 legacy row.
CREATE TRIGGER form_content_insert BEFORE INSERT ON form_content_snapshots BEGIN
    SELECT RAISE(ABORT,'already sealed') WHERE EXISTS(SELECT 1 FROM form_versions WHERE form_id = NEW.form_id AND revision = NEW.revision);
    SELECT RAISE(ABORT,'typing seal mismatch') WHERE
      json_type(NEW.after_json,'$.content.typing.assignments') IS NOT CASE NEW.typing_defined WHEN 1 THEN 'array' ELSE 'null' END
      OR (SELECT count(*) FROM form_type_bindings WHERE form_id=NEW.form_id AND revision=NEW.revision) <> CASE NEW.typing_defined WHEN 1 THEN json_array_length(NEW.after_json,'$.content.typing.assignments') ELSE 0 END
      OR EXISTS(SELECT 1 FROM form_type_bindings b WHERE b.form_id=NEW.form_id AND b.revision=NEW.revision AND (json_extract(NEW.after_json,'$.content.typing.assignments[' || (b.slot-1) || '].pin.type_id') IS NOT b.type_id OR json_extract(NEW.after_json,'$.content.typing.assignments[' || (b.slot-1) || '].pin.version') IS NOT b.type_version));
END;
CREATE TRIGGER variant_content_insert BEFORE INSERT ON variant_content_snapshots BEGIN
    SELECT RAISE(ABORT,'already sealed') WHERE EXISTS(SELECT 1 FROM variant_revisions WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND revision=NEW.revision);
    SELECT RAISE(ABORT,'typing seal mismatch') WHERE
      json_type(NEW.after_json,'$.content.typing.assignments') IS NOT CASE NEW.typing_defined WHEN 1 THEN 'array' ELSE 'null' END
      OR (SELECT count(*) FROM variant_type_bindings WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND revision=NEW.revision) <> CASE NEW.typing_defined WHEN 1 THEN json_array_length(NEW.after_json,'$.content.typing.assignments') ELSE 0 END
      OR EXISTS(SELECT 1 FROM variant_type_bindings b WHERE b.world_id=NEW.world_id AND b.variant_id=NEW.variant_id AND b.revision=NEW.revision AND (json_extract(NEW.after_json,'$.content.typing.assignments[' || (b.slot-1) || '].pin.type_id') IS NOT b.type_id OR json_extract(NEW.after_json,'$.content.typing.assignments[' || (b.slot-1) || '].pin.version') IS NOT b.type_version));
END;
CREATE TRIGGER world_content_insert BEFORE INSERT ON world_content_snapshots WHEN EXISTS(SELECT 1 FROM world_revisions WHERE world_id=NEW.world_id AND revision=NEW.revision) BEGIN SELECT RAISE(ABORT,'already sealed'); END;
CREATE TRIGGER form_content_guard BEFORE INSERT ON form_versions WHEN NEW.content_snapshot_version <> 2 OR NOT EXISTS(SELECT 1 FROM form_content_snapshots WHERE form_id=NEW.form_id AND revision=NEW.revision AND marker=2) BEGIN SELECT RAISE(ABORT,'required full snapshot'); END;
CREATE TRIGGER variant_content_guard BEFORE INSERT ON variant_revisions WHEN NEW.content_snapshot_version <> 2 OR NOT EXISTS(SELECT 1 FROM variant_content_snapshots WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND revision=NEW.revision AND marker=2) BEGIN SELECT RAISE(ABORT,'required full snapshot'); END;
CREATE TRIGGER world_content_guard BEFORE INSERT ON world_revisions WHEN NEW.content_snapshot_version <> 2 OR NOT EXISTS(SELECT 1 FROM world_content_snapshots WHERE world_id=NEW.world_id AND revision=NEW.revision AND marker=2) BEGIN SELECT RAISE(ABORT,'required full snapshot'); END;
CREATE TRIGGER form_binding_insert BEFORE INSERT ON form_type_bindings WHEN EXISTS(SELECT 1 FROM form_content_snapshots WHERE form_id=NEW.form_id AND revision=NEW.revision) OR EXISTS(SELECT 1 FROM form_versions WHERE form_id=NEW.form_id AND revision=NEW.revision) BEGIN SELECT RAISE(ABORT,'sealed bindings'); END;
CREATE TRIGGER variant_binding_insert BEFORE INSERT ON variant_type_bindings WHEN EXISTS(SELECT 1 FROM variant_content_snapshots WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND revision=NEW.revision) OR EXISTS(SELECT 1 FROM variant_revisions WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND revision=NEW.revision) BEGIN SELECT RAISE(ABORT,'sealed bindings'); END;
CREATE TRIGGER form_binding_update BEFORE UPDATE ON form_type_bindings BEGIN SELECT RAISE(ABORT,'immutable bindings'); END;
CREATE TRIGGER form_binding_delete BEFORE DELETE ON form_type_bindings BEGIN SELECT RAISE(ABORT,'retained bindings'); END;
CREATE TRIGGER variant_binding_update BEFORE UPDATE ON variant_type_bindings BEGIN SELECT RAISE(ABORT,'immutable bindings'); END;
CREATE TRIGGER variant_binding_delete BEFORE DELETE ON variant_type_bindings BEGIN SELECT RAISE(ABORT,'retained bindings'); END;
CREATE TRIGGER form_content_update BEFORE UPDATE ON form_content_snapshots BEGIN SELECT RAISE(ABORT,'immutable snapshot'); END;
CREATE TRIGGER form_content_delete BEFORE DELETE ON form_content_snapshots BEGIN SELECT RAISE(ABORT,'retained snapshot'); END;
CREATE TRIGGER variant_content_update BEFORE UPDATE ON variant_content_snapshots BEGIN SELECT RAISE(ABORT,'immutable snapshot'); END;
CREATE TRIGGER variant_content_delete BEFORE DELETE ON variant_content_snapshots BEGIN SELECT RAISE(ABORT,'retained snapshot'); END;
CREATE TRIGGER world_content_update BEFORE UPDATE ON world_content_snapshots BEGIN SELECT RAISE(ABORT,'immutable snapshot'); END;
CREATE TRIGGER world_content_delete BEFORE DELETE ON world_content_snapshots BEGIN SELECT RAISE(ABORT,'retained snapshot'); END;

-- Concrete version provenance, not a live generic entity/JSON store.
CREATE TABLE type_version_sources (
    type_id TEXT NOT NULL, revision INTEGER NOT NULL,
    endpoint_kind TEXT NOT NULL CHECK(endpoint_kind='type'), external_id INTEGER NOT NULL CHECK(external_id>0),
    source_url TEXT NOT NULL, retrieved_at TEXT NOT NULL CHECK(retrieved_at GLOB '*T*Z'), transformation_version INTEGER NOT NULL CHECK(transformation_version>0),
    PRIMARY KEY(type_id,revision), FOREIGN KEY(type_id,revision) REFERENCES type_versions(type_id,revision)
);
CREATE TABLE species_version_sources (
    species_id TEXT NOT NULL, revision INTEGER NOT NULL,
    endpoint_kind TEXT NOT NULL CHECK(endpoint_kind='pokemon-species'), external_id INTEGER NOT NULL CHECK(external_id>0),
    source_url TEXT NOT NULL, retrieved_at TEXT NOT NULL CHECK(retrieved_at GLOB '*T*Z'), transformation_version INTEGER NOT NULL CHECK(transformation_version>0),
    PRIMARY KEY(species_id,revision), FOREIGN KEY(species_id,revision) REFERENCES species_versions(species_id,revision)
);
CREATE TABLE form_version_sources (
    form_id TEXT NOT NULL, revision INTEGER NOT NULL,
    endpoint_kind TEXT NOT NULL CHECK(endpoint_kind='pokemon-form'), external_id INTEGER NOT NULL CHECK(external_id>0),
    source_url TEXT NOT NULL, retrieved_at TEXT NOT NULL CHECK(retrieved_at GLOB '*T*Z'), transformation_version INTEGER NOT NULL CHECK(transformation_version>0),
    PRIMARY KEY(form_id,revision), FOREIGN KEY(form_id,revision) REFERENCES form_versions(form_id,revision)
);
CREATE TRIGGER type_source_update BEFORE UPDATE ON type_version_sources BEGIN SELECT RAISE(ABORT,'immutable provenance'); END;
CREATE TRIGGER type_source_delete BEFORE DELETE ON type_version_sources BEGIN SELECT RAISE(ABORT,'retained provenance'); END;
CREATE TRIGGER species_source_update BEFORE UPDATE ON species_version_sources BEGIN SELECT RAISE(ABORT,'immutable provenance'); END;
CREATE TRIGGER species_source_delete BEFORE DELETE ON species_version_sources BEGIN SELECT RAISE(ABORT,'retained provenance'); END;
CREATE TRIGGER form_source_update BEFORE UPDATE ON form_version_sources BEGIN SELECT RAISE(ABORT,'immutable provenance'); END;
CREATE TRIGGER form_source_delete BEFORE DELETE ON form_version_sources BEGIN SELECT RAISE(ABORT,'retained provenance'); END;
CREATE VIEW reference_provenance AS
 SELECT 'type' AS object_kind,type_id AS object_id,revision,endpoint_kind,external_id,source_url,retrieved_at,transformation_version FROM type_version_sources
 UNION ALL SELECT 'species',species_id,revision,endpoint_kind,external_id,source_url,retrieved_at,transformation_version FROM species_version_sources
 UNION ALL SELECT 'form',form_id,revision,endpoint_kind,external_id,source_url,retrieved_at,transformation_version FROM form_version_sources;
