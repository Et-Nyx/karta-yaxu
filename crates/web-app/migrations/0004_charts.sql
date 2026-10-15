-- Independent chart aggregate. Empty scope denotes shared; every other scope is
-- the exact owning world. World content/history membership is unchanged.
CREATE TABLE charts (
    id TEXT PRIMARY KEY NOT NULL,
    scope TEXT NOT NULL,
    world_id TEXT REFERENCES worlds(id),
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    revision INTEGER NOT NULL CHECK(revision > 0),
    CHECK(scope = coalesce(world_id,'')),
    UNIQUE(scope,id), UNIQUE(scope,slug), UNIQUE(world_id),
    FOREIGN KEY(scope,id,revision) REFERENCES chart_versions(scope,chart_id,revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE chart_versions (
    scope TEXT NOT NULL,
    chart_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    source_scope TEXT NOT NULL DEFAULT '' CHECK(source_scope=''),
    source_chart_id TEXT,
    source_revision INTEGER,
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    archived_at INTEGER,
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    previous_revision INTEGER,
    before_json TEXT CHECK(before_json IS NULL OR json_valid(before_json)),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 1),
    PRIMARY KEY(scope,chart_id,revision),
    UNIQUE(scope,chart_id,revision,source_chart_id,source_revision),
    CHECK((source_chart_id IS NULL) = (source_revision IS NULL)),
    CHECK(scope <> '' OR source_chart_id IS NULL),
    CHECK((revision=1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision>1 AND previous_revision IS NOT NULL AND previous_revision=revision-1 AND before_json IS NOT NULL)),
    FOREIGN KEY(scope,chart_id) REFERENCES charts(scope,id),
    FOREIGN KEY(scope,chart_id,previous_revision) REFERENCES chart_versions(scope,chart_id,revision),
    FOREIGN KEY(source_scope,source_chart_id,source_revision) REFERENCES chart_versions(scope,chart_id,revision)
);
CREATE TABLE chart_roster (
    scope TEXT NOT NULL,
    chart_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    slot INTEGER NOT NULL CHECK(slot BETWEEN 1 AND 32),
    type_id TEXT NOT NULL,
    type_version INTEGER NOT NULL CHECK(type_version > 0),
    shared_type_id TEXT,
    world_id TEXT,
    PRIMARY KEY(scope,chart_id,revision,slot),
    UNIQUE(scope,chart_id,revision,type_id),
    CHECK((scope='' AND shared_type_id IS NOT NULL AND shared_type_id=type_id AND world_id IS NULL) OR (scope<>'' AND shared_type_id IS NULL AND world_id IS NOT NULL AND world_id=scope)),
    FOREIGN KEY(shared_type_id,type_version) REFERENCES type_versions(type_id,revision),
    FOREIGN KEY(world_id,type_id,type_version) REFERENCES world_type_versions(world_id,type_id,revision),
    FOREIGN KEY(scope,chart_id,revision) REFERENCES chart_versions(scope,chart_id,revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE chart_cells (
    scope TEXT NOT NULL,
    chart_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    attacker_type_id TEXT NOT NULL,
    defender_type_id TEXT NOT NULL,
    numerator INTEGER NOT NULL CHECK(numerator BETWEEN 0 AND 1000000),
    denominator INTEGER NOT NULL CHECK(denominator BETWEEN 1 AND 1000000),
    PRIMARY KEY(scope,chart_id,revision,attacker_type_id,defender_type_id),
    FOREIGN KEY(scope,chart_id,revision,attacker_type_id) REFERENCES chart_roster(scope,chart_id,revision,type_id),
    FOREIGN KEY(scope,chart_id,revision,defender_type_id) REFERENCES chart_roster(scope,chart_id,revision,type_id)
);
CREATE TABLE chart_mapping (
    scope TEXT NOT NULL CHECK(scope<>''),
    chart_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    source_scope TEXT NOT NULL DEFAULT '' CHECK(source_scope=''),
    source_chart_id TEXT NOT NULL,
    source_revision INTEGER NOT NULL,
    source_type_id TEXT NOT NULL,
    local_type_id TEXT NOT NULL,
    PRIMARY KEY(scope,chart_id,revision,source_type_id),
    UNIQUE(scope,chart_id,revision,local_type_id),
    FOREIGN KEY(scope,chart_id,revision,local_type_id) REFERENCES chart_roster(scope,chart_id,revision,type_id),
    FOREIGN KEY(source_scope,source_chart_id,source_revision,source_type_id) REFERENCES chart_roster(scope,chart_id,revision,type_id),
    FOREIGN KEY(scope,chart_id,revision,source_chart_id,source_revision) REFERENCES chart_versions(scope,chart_id,revision,source_chart_id,source_revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE TRIGGER chart_identity BEFORE UPDATE OF id,scope,world_id ON charts
WHEN NEW.id<>OLD.id OR NEW.scope<>OLD.scope OR NEW.world_id IS NOT OLD.world_id
BEGIN SELECT RAISE(ABORT,'immutable chart identity'); END;
CREATE TRIGGER chart_versions_update BEFORE UPDATE ON chart_versions BEGIN SELECT RAISE(ABORT,'immutable chart version'); END;
CREATE TRIGGER chart_versions_delete BEFORE DELETE ON chart_versions BEGIN SELECT RAISE(ABORT,'retained chart version'); END;
CREATE TRIGGER chart_roster_insert BEFORE INSERT ON chart_roster
WHEN EXISTS(SELECT 1 FROM chart_versions WHERE scope=NEW.scope AND chart_id=NEW.chart_id AND revision=NEW.revision)
BEGIN SELECT RAISE(ABORT,'sealed chart roster'); END;
CREATE TRIGGER chart_cells_insert BEFORE INSERT ON chart_cells
WHEN EXISTS(SELECT 1 FROM chart_versions WHERE scope=NEW.scope AND chart_id=NEW.chart_id AND revision=NEW.revision)
BEGIN SELECT RAISE(ABORT,'sealed chart cells'); END;
CREATE TRIGGER chart_mapping_insert BEFORE INSERT ON chart_mapping BEGIN
    SELECT RAISE(ABORT,'sealed chart mapping') WHERE EXISTS(SELECT 1 FROM chart_versions WHERE scope=NEW.scope AND chart_id=NEW.chart_id AND revision=NEW.revision);
    SELECT RAISE(ABORT,'incompatible mapped identity') WHERE NOT EXISTS(
        SELECT 1 FROM chart_roster r JOIN world_type_versions t ON t.world_id=r.world_id AND t.type_id=r.type_id AND t.revision=r.type_version
        WHERE r.scope=NEW.scope AND r.chart_id=NEW.chart_id AND r.revision=NEW.revision AND r.type_id=NEW.local_type_id AND t.source_type_id=NEW.source_type_id
    );
END;
CREATE TRIGGER chart_roster_update BEFORE UPDATE ON chart_roster BEGIN SELECT RAISE(ABORT,'immutable chart roster'); END;
CREATE TRIGGER chart_roster_delete BEFORE DELETE ON chart_roster BEGIN SELECT RAISE(ABORT,'retained chart roster'); END;
CREATE TRIGGER chart_cells_update BEFORE UPDATE ON chart_cells BEGIN SELECT RAISE(ABORT,'immutable chart cells'); END;
CREATE TRIGGER chart_cells_delete BEFORE DELETE ON chart_cells BEGIN SELECT RAISE(ABORT,'retained chart cells'); END;
CREATE TRIGGER chart_mapping_update BEFORE UPDATE ON chart_mapping BEGIN SELECT RAISE(ABORT,'immutable chart mapping'); END;
CREATE TRIGGER chart_mapping_delete BEFORE DELETE ON chart_mapping BEGIN SELECT RAISE(ABORT,'retained chart mapping'); END;
-- Insert the version last: it seals even empty children. Readers additionally
-- verify canonical order, rational reduction, exact metadata and dependencies.
CREATE TRIGGER chart_version_seal BEFORE INSERT ON chart_versions BEGIN
    SELECT RAISE(ABORT,'chart roster seal mismatch') WHERE
      json_type(NEW.after_json,'$.content.roster') IS NOT 'array'
      OR (SELECT count(*) FROM chart_roster WHERE scope=NEW.scope AND chart_id=NEW.chart_id AND revision=NEW.revision) <> json_array_length(NEW.after_json,'$.content.roster')
      OR EXISTS(SELECT 1 FROM chart_roster r WHERE r.scope=NEW.scope AND r.chart_id=NEW.chart_id AND r.revision=NEW.revision AND (
        json_extract(NEW.after_json,'$.content.roster['||(r.slot-1)||'].pin.type_id') IS NOT r.type_id OR
        json_extract(NEW.after_json,'$.content.roster['||(r.slot-1)||'].pin.version') IS NOT r.type_version));
    SELECT RAISE(ABORT,'chart cells seal mismatch') WHERE
      json_type(NEW.after_json,'$.content.cells') IS NOT 'array'
      OR (SELECT count(*) FROM chart_cells WHERE scope=NEW.scope AND chart_id=NEW.chart_id AND revision=NEW.revision) <> json_array_length(NEW.after_json,'$.content.cells')
      OR EXISTS(SELECT 1 FROM json_each(NEW.after_json,'$.content.cells') j WHERE NOT EXISTS(
        SELECT 1 FROM chart_cells c WHERE c.scope=NEW.scope AND c.chart_id=NEW.chart_id AND c.revision=NEW.revision AND c.attacker_type_id=json_extract(j.value,'$.attacker_type_id') AND c.defender_type_id=json_extract(j.value,'$.defender_type_id') AND c.numerator=json_extract(j.value,'$.factor.numerator') AND c.denominator=json_extract(j.value,'$.factor.denominator')));
    SELECT RAISE(ABORT,'chart mapping seal mismatch') WHERE
      json_type(NEW.after_json,'$.content.mapping') IS NOT 'array'
      OR (SELECT count(*) FROM chart_mapping WHERE scope=NEW.scope AND chart_id=NEW.chart_id AND revision=NEW.revision) <> json_array_length(NEW.after_json,'$.content.mapping')
      OR EXISTS(SELECT 1 FROM json_each(NEW.after_json,'$.content.mapping') j WHERE NOT EXISTS(
        SELECT 1 FROM chart_mapping m WHERE m.scope=NEW.scope AND m.chart_id=NEW.chart_id AND m.revision=NEW.revision AND m.source_type_id=json_extract(j.value,'$.source_type_id') AND m.local_type_id=json_extract(j.value,'$.local_type_id')));
END;
