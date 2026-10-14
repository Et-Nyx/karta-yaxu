-- Typed live content, exact immutable source versions, independent world access.
-- No content is seeded by migration. All references use RESTRICT/NO ACTION.
CREATE TABLE audit_events (
    id TEXT PRIMARY KEY NOT NULL,
    actor_user_id TEXT REFERENCES users(id),
    system_key TEXT CHECK(system_key = 'bundled-fixture-loader'),
    occurred_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    action TEXT NOT NULL CHECK(action IN ('create', 'edit', 'add-variant', 'own', 'adopt', 'grant', 'revoke')),
    reason TEXT CHECK(reason IS NULL OR length(reason) <= 500),
    CHECK ((actor_user_id IS NULL) <> (system_key IS NULL))
);

CREATE TABLE species (
    id TEXT PRIMARY KEY NOT NULL,
    slug TEXT NOT NULL UNIQUE CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    revision INTEGER NOT NULL CHECK(revision > 0),
    default_form_id TEXT NOT NULL UNIQUE,
    archived_at INTEGER,
    FOREIGN KEY(default_form_id, id) REFERENCES forms(id, species_id) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(id, revision) REFERENCES species_versions(species_id, revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE species_versions (
    species_id TEXT NOT NULL REFERENCES species(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    default_form_id TEXT NOT NULL,
    archived_at INTEGER,
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    previous_revision INTEGER,
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND coalesce(json_extract(before_json,'$.snapshot_version') = 1, 0))),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND coalesce(json_extract(after_json,'$.snapshot_version') = 1, 0)),
    PRIMARY KEY(species_id, revision),
    FOREIGN KEY(species_id, previous_revision) REFERENCES species_versions(species_id, revision),
    FOREIGN KEY(default_form_id, species_id) REFERENCES forms(id, species_id) DEFERRABLE INITIALLY DEFERRED,
    CHECK ((revision = 1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision > 1 AND previous_revision IS NOT NULL AND previous_revision = revision - 1 AND before_json IS NOT NULL))
);
CREATE TABLE forms (
    id TEXT PRIMARY KEY NOT NULL,
    species_id TEXT NOT NULL REFERENCES species(id),
    species_version INTEGER NOT NULL,
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    revision INTEGER NOT NULL CHECK(revision > 0),
    is_default INTEGER NOT NULL CHECK(is_default IN (0,1)),
    archived_at INTEGER,
    UNIQUE(species_id, slug),
    UNIQUE(id, species_id),
    FOREIGN KEY(species_id, species_version) REFERENCES species_versions(species_id, revision),
    FOREIGN KEY(id, revision) REFERENCES form_versions(form_id, revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX species_initial_slug ON species_versions(slug) WHERE revision = 1;
CREATE UNIQUE INDEX forms_default ON forms(species_id) WHERE is_default = 1;
CREATE TABLE form_versions (
    form_id TEXT NOT NULL,
    species_id TEXT NOT NULL,
    species_version INTEGER NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    is_default INTEGER NOT NULL CHECK(is_default IN (0,1)),
    archived_at INTEGER,
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    previous_revision INTEGER,
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND coalesce(json_extract(before_json,'$.snapshot_version') = 1, 0))),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND coalesce(json_extract(after_json,'$.snapshot_version') = 1, 0)),
    PRIMARY KEY(form_id, revision),
    FOREIGN KEY(form_id, species_id) REFERENCES forms(id, species_id),
    FOREIGN KEY(species_id, species_version) REFERENCES species_versions(species_id, revision),
    FOREIGN KEY(form_id, previous_revision) REFERENCES form_versions(form_id, revision),
    CHECK ((revision = 1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision > 1 AND previous_revision IS NOT NULL AND previous_revision = revision - 1 AND before_json IS NOT NULL))
);
CREATE TABLE worlds (
    id TEXT PRIMARY KEY NOT NULL,
    owner_user_id TEXT NOT NULL REFERENCES users(id),
    slug TEXT NOT NULL UNIQUE CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    revision INTEGER NOT NULL CHECK(revision > 0),
    access_revision INTEGER NOT NULL CHECK(access_revision > 0),
    archived_at INTEGER,
    FOREIGN KEY(id, revision) REFERENCES world_revisions(world_id, revision) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(id, access_revision) REFERENCES world_access_revisions(world_id, revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE variants (
    id TEXT PRIMARY KEY NOT NULL,
    world_id TEXT NOT NULL REFERENCES worlds(id),
    source_form_id TEXT,
    source_form_version INTEGER,
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    revision INTEGER NOT NULL CHECK(revision > 0),
    archived_at INTEGER,
    UNIQUE(world_id, slug),
    UNIQUE(world_id, id),
    CHECK((source_form_id IS NULL) = (source_form_version IS NULL)),
    FOREIGN KEY(source_form_id, source_form_version) REFERENCES form_versions(form_id, revision),
    FOREIGN KEY(world_id, id, revision) REFERENCES variant_revisions(world_id, variant_id, revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX variants_source ON variants(source_form_id, source_form_version);
CREATE TABLE world_revisions (
    world_id TEXT NOT NULL REFERENCES worlds(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    previous_revision INTEGER,
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND coalesce(json_extract(before_json,'$.snapshot_version') = 1, 0))),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND coalesce(json_extract(after_json,'$.snapshot_version') = 1, 0)),
    PRIMARY KEY(world_id, revision),
    FOREIGN KEY(world_id, previous_revision) REFERENCES world_revisions(world_id, revision),
    CHECK ((revision = 1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision > 1 AND previous_revision IS NOT NULL AND previous_revision = revision - 1 AND before_json IS NOT NULL))
);
CREATE TABLE variant_revisions (
    world_id TEXT NOT NULL,
    variant_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    previous_revision INTEGER,
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND coalesce(json_extract(before_json,'$.snapshot_version') = 1, 0))),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND coalesce(json_extract(after_json,'$.snapshot_version') = 1, 0)),
    PRIMARY KEY(world_id, variant_id, revision),
    FOREIGN KEY(world_id, variant_id) REFERENCES variants(world_id, id),
    FOREIGN KEY(world_id, variant_id, previous_revision) REFERENCES variant_revisions(world_id, variant_id, revision),
    CHECK ((revision = 1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision > 1 AND previous_revision IS NOT NULL AND previous_revision = revision - 1 AND before_json IS NOT NULL))
);
CREATE TABLE world_collaborators (
    world_id TEXT NOT NULL REFERENCES worlds(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    PRIMARY KEY(world_id, user_id)
);
CREATE INDEX variant_history ON variant_revisions(variant_id, revision);
CREATE TABLE world_access_revisions (
    world_id TEXT NOT NULL REFERENCES worlds(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    previous_revision INTEGER,
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND coalesce(json_extract(before_json,'$.snapshot_version') = 1, 0))),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND coalesce(json_extract(after_json,'$.snapshot_version') = 1, 0)),
    PRIMARY KEY(world_id, revision),
    FOREIGN KEY(world_id, previous_revision) REFERENCES world_access_revisions(world_id, revision),
    CHECK ((revision = 1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision > 1 AND previous_revision IS NOT NULL AND previous_revision = revision - 1 AND before_json IS NOT NULL))
);

-- Definitions and identities are retained even if no current entry references them.
CREATE TRIGGER species_identity BEFORE UPDATE OF id, default_form_id ON species
WHEN NEW.id <> OLD.id OR NEW.default_form_id <> OLD.default_form_id
BEGIN SELECT RAISE(ABORT, 'immutable species identity'); END;
CREATE TRIGGER form_default_identity BEFORE INSERT ON forms
WHEN NEW.is_default <> (NEW.id = (SELECT default_form_id FROM species WHERE id = NEW.species_id))
BEGIN SELECT RAISE(ABORT, 'default form identity mismatch'); END;
CREATE TRIGGER form_version_default BEFORE INSERT ON form_versions
WHEN NEW.is_default <> (SELECT is_default FROM forms WHERE id = NEW.form_id)
BEGIN SELECT RAISE(ABORT, 'default form version mismatch'); END;
CREATE TRIGGER form_identity BEFORE UPDATE OF id, species_id, is_default ON forms
WHEN NEW.id <> OLD.id OR NEW.species_id <> OLD.species_id OR NEW.is_default <> OLD.is_default
BEGIN SELECT RAISE(ABORT, 'immutable form identity'); END;
CREATE TRIGGER world_identity BEFORE UPDATE OF id, owner_user_id ON worlds
WHEN NEW.id <> OLD.id OR NEW.owner_user_id <> OLD.owner_user_id
BEGIN SELECT RAISE(ABORT, 'immutable world ownership'); END;
CREATE TRIGGER variant_identity BEFORE UPDATE OF id, world_id ON variants
WHEN NEW.id <> OLD.id OR NEW.world_id <> OLD.world_id
BEGIN SELECT RAISE(ABORT, 'immutable variant scope'); END;
CREATE TRIGGER species_versions_update BEFORE UPDATE ON species_versions BEGIN SELECT RAISE(ABORT, 'immutable version'); END;
CREATE TRIGGER species_versions_delete BEFORE DELETE ON species_versions BEGIN SELECT RAISE(ABORT, 'retained version'); END;
CREATE TRIGGER form_versions_update BEFORE UPDATE ON form_versions BEGIN SELECT RAISE(ABORT, 'immutable version'); END;
CREATE TRIGGER form_versions_delete BEFORE DELETE ON form_versions BEGIN SELECT RAISE(ABORT, 'retained version'); END;
CREATE TRIGGER world_revisions_update BEFORE UPDATE ON world_revisions BEGIN SELECT RAISE(ABORT, 'immutable revision'); END;
CREATE TRIGGER world_revisions_delete BEFORE DELETE ON world_revisions BEGIN SELECT RAISE(ABORT, 'retained revision'); END;
CREATE TRIGGER variant_revisions_update BEFORE UPDATE ON variant_revisions BEGIN SELECT RAISE(ABORT, 'immutable revision'); END;
CREATE TRIGGER variant_revisions_delete BEFORE DELETE ON variant_revisions BEGIN SELECT RAISE(ABORT, 'retained revision'); END;
CREATE TRIGGER access_revisions_update BEFORE UPDATE ON world_access_revisions BEGIN SELECT RAISE(ABORT, 'immutable access revision'); END;
CREATE TRIGGER access_revisions_delete BEFORE DELETE ON world_access_revisions BEGIN SELECT RAISE(ABORT, 'retained access revision'); END;
CREATE TRIGGER audit_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT, 'immutable audit'); END;
CREATE TRIGGER audit_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT, 'retained audit'); END;
