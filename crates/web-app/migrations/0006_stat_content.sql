-- Additive authoritative creature extensions. Never rewrite sealed v1/v2 history.
ALTER TABLE form_versions ADD COLUMN stat_snapshot_version INTEGER NOT NULL DEFAULT 0 CHECK(stat_snapshot_version IN (0,3));
ALTER TABLE variant_revisions ADD COLUMN stat_snapshot_version INTEGER NOT NULL DEFAULT 0 CHECK(stat_snapshot_version IN (0,3));
CREATE UNIQUE INDEX form_stat_content_key ON form_versions(form_id,revision,stat_snapshot_version);
CREATE UNIQUE INDEX variant_stat_content_key ON variant_revisions(world_id,variant_id,revision,stat_snapshot_version);

CREATE TABLE form_stat_snapshots (
    form_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision>0),
    marker INTEGER NOT NULL DEFAULT 3 CHECK(marker=3),
    stats_defined INTEGER NOT NULL CHECK(stats_defined IN (0,1)),
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND json_type(before_json,'$.snapshot_version') IS 'integer' AND json_extract(before_json,'$.snapshot_version') IN (1,2,3))),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 3),
    PRIMARY KEY(form_id,revision),
    FOREIGN KEY(form_id,revision,marker) REFERENCES form_versions(form_id,revision,stat_snapshot_version) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(form_id,revision) REFERENCES form_content_snapshots(form_id,revision) DEFERRABLE INITIALLY DEFERRED,
    CHECK((revision=1 AND before_json IS NULL) OR (revision>1 AND before_json IS NOT NULL))
) STRICT;
CREATE TABLE variant_stat_snapshots (
    world_id TEXT NOT NULL,
    variant_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision>0),
    marker INTEGER NOT NULL DEFAULT 3 CHECK(marker=3),
    stats_defined INTEGER NOT NULL CHECK(stats_defined IN (0,1)),
    roster_id TEXT,
    roster_version INTEGER,
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND json_type(before_json,'$.snapshot_version') IS 'integer' AND json_extract(before_json,'$.snapshot_version') IN (1,2,3))),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 3),
    PRIMARY KEY(world_id,variant_id,revision),
    CHECK((stats_defined=0 AND roster_id IS NULL AND roster_version IS NULL) OR (stats_defined=1 AND roster_id IS NOT NULL AND roster_version IS NOT NULL)),
    FOREIGN KEY(world_id,roster_id,roster_version) REFERENCES stat_roster_versions(world_id,roster_id,revision),
    FOREIGN KEY(world_id,variant_id,revision,marker) REFERENCES variant_revisions(world_id,variant_id,revision,stat_snapshot_version) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(world_id,variant_id,revision) REFERENCES variant_content_snapshots(world_id,variant_id,revision) DEFERRABLE INITIALLY DEFERRED,
    CHECK((revision=1 AND before_json IS NULL) OR (revision>1 AND before_json IS NOT NULL))
) STRICT;
CREATE TABLE form_stat_bindings (
    form_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    slot INTEGER NOT NULL CHECK(slot>0),
    scope TEXT NOT NULL DEFAULT '' CHECK(scope=''),
    stat_id TEXT NOT NULL,
    stat_version INTEGER NOT NULL,
    coefficient INTEGER,
    scale INTEGER,
    CHECK((coefficient IS NULL AND scale IS NULL) OR (coefficient IS NOT NULL AND scale BETWEEN 0 AND 6 AND scale IS NOT NULL AND (scale=0 OR coefficient%10<>0))),
    PRIMARY KEY(form_id,revision,slot),
    UNIQUE(form_id,revision,stat_id),
    FOREIGN KEY(scope,stat_id,stat_version) REFERENCES stat_versions(scope,stat_id,revision),
    FOREIGN KEY(form_id,revision) REFERENCES form_stat_snapshots(form_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE variant_stat_bindings (
    world_id TEXT NOT NULL,
    variant_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    slot INTEGER NOT NULL CHECK(slot>0),
    scope TEXT NOT NULL CHECK(scope=world_id),
    stat_id TEXT NOT NULL,
    stat_version INTEGER NOT NULL,
    coefficient INTEGER,
    scale INTEGER,
    CHECK((coefficient IS NULL AND scale IS NULL) OR (coefficient IS NOT NULL AND scale BETWEEN 0 AND 6 AND scale IS NOT NULL AND (scale=0 OR coefficient%10<>0))),
    PRIMARY KEY(world_id,variant_id,revision,slot),
    UNIQUE(world_id,variant_id,revision,stat_id),
    FOREIGN KEY(scope,stat_id,stat_version) REFERENCES stat_versions(scope,stat_id,revision),
    FOREIGN KEY(world_id,variant_id,revision) REFERENCES variant_stat_snapshots(world_id,variant_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE TRIGGER form_stat_insert BEFORE INSERT ON form_stat_snapshots BEGIN
    SELECT RAISE(ABORT,'already sealed') WHERE EXISTS(SELECT 1 FROM form_versions WHERE form_id=NEW.form_id AND revision=NEW.revision);
    SELECT RAISE(ABORT,'stats seal mismatch') WHERE
      json_type(NEW.after_json,'$.content.stats.roster') IS NOT 'null'
      OR json_type(NEW.after_json,'$.content.stats.assignments') IS NOT CASE NEW.stats_defined WHEN 1 THEN 'array' ELSE 'null' END
      OR (SELECT count(*) FROM form_stat_bindings WHERE form_id=NEW.form_id AND revision=NEW.revision) <> CASE NEW.stats_defined WHEN 1 THEN json_array_length(NEW.after_json,'$.content.stats.assignments') ELSE 0 END
      OR EXISTS(SELECT 1 FROM form_stat_bindings b WHERE b.form_id=NEW.form_id AND b.revision=NEW.revision AND (
        json_extract(NEW.after_json,'$.content.stats.assignments[' || (b.slot-1) || '].definition.id') IS NOT b.stat_id
        OR json_extract(NEW.after_json,'$.content.stats.assignments[' || (b.slot-1) || '].definition.revision') IS NOT b.stat_version
        OR json_type(NEW.after_json,'$.content.stats.assignments[' || (b.slot-1) || '].definition.world_id') IS NOT 'null'
        OR json_type(NEW.after_json,'$.content.stats.assignments[' || (b.slot-1) || '].value') IS NOT CASE WHEN b.coefficient IS NULL THEN 'null' ELSE 'text' END));
END;
CREATE TRIGGER variant_stat_insert BEFORE INSERT ON variant_stat_snapshots BEGIN
    SELECT RAISE(ABORT,'already sealed') WHERE EXISTS(SELECT 1 FROM variant_revisions WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND revision=NEW.revision);
    SELECT RAISE(ABORT,'stats seal mismatch') WHERE
      json_type(NEW.after_json,'$.content.stats.assignments') IS NOT CASE NEW.stats_defined WHEN 1 THEN 'array' ELSE 'null' END
      OR (NEW.stats_defined=0 AND json_type(NEW.after_json,'$.content.stats.roster') IS NOT 'null')
      OR (NEW.stats_defined=1 AND (json_extract(NEW.after_json,'$.content.stats.roster.roster_id') IS NOT NEW.roster_id OR json_extract(NEW.after_json,'$.content.stats.roster.version') IS NOT NEW.roster_version))
      OR (SELECT count(*) FROM variant_stat_bindings WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND revision=NEW.revision) <> CASE NEW.stats_defined WHEN 1 THEN json_array_length(NEW.after_json,'$.content.stats.assignments') ELSE 0 END
      OR EXISTS(SELECT 1 FROM variant_stat_bindings b WHERE b.world_id=NEW.world_id AND b.variant_id=NEW.variant_id AND b.revision=NEW.revision AND (
        json_extract(NEW.after_json,'$.content.stats.assignments[' || (b.slot-1) || '].definition.id') IS NOT b.stat_id
        OR json_extract(NEW.after_json,'$.content.stats.assignments[' || (b.slot-1) || '].definition.revision') IS NOT b.stat_version
        OR json_extract(NEW.after_json,'$.content.stats.assignments[' || (b.slot-1) || '].definition.world_id') IS NOT NEW.world_id
        OR json_type(NEW.after_json,'$.content.stats.assignments[' || (b.slot-1) || '].value') IS NOT CASE WHEN b.coefficient IS NULL THEN 'null' ELSE 'text' END));
END;
CREATE TRIGGER form_stat_guard BEFORE INSERT ON form_versions WHEN NEW.stat_snapshot_version<>3 OR NEW.content_snapshot_version<>2 OR NOT EXISTS(SELECT 1 FROM form_stat_snapshots WHERE form_id=NEW.form_id AND revision=NEW.revision AND marker=3) BEGIN SELECT RAISE(ABORT,'required stat snapshot'); END;
CREATE TRIGGER variant_stat_guard BEFORE INSERT ON variant_revisions WHEN NEW.stat_snapshot_version<>3 OR NEW.content_snapshot_version<>2 OR NOT EXISTS(SELECT 1 FROM variant_stat_snapshots WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND revision=NEW.revision AND marker=3) BEGIN SELECT RAISE(ABORT,'required stat snapshot'); END;
CREATE TRIGGER form_stat_binding_insert BEFORE INSERT ON form_stat_bindings WHEN EXISTS(SELECT 1 FROM form_stat_snapshots WHERE form_id=NEW.form_id AND revision=NEW.revision) OR EXISTS(SELECT 1 FROM form_versions WHERE form_id=NEW.form_id AND revision=NEW.revision) BEGIN SELECT RAISE(ABORT,'sealed stats'); END;
CREATE TRIGGER variant_stat_binding_insert BEFORE INSERT ON variant_stat_bindings WHEN EXISTS(SELECT 1 FROM variant_stat_snapshots WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND revision=NEW.revision) OR EXISTS(SELECT 1 FROM variant_revisions WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND revision=NEW.revision) BEGIN SELECT RAISE(ABORT,'sealed stats'); END;
CREATE TRIGGER form_stat_binding_update BEFORE UPDATE ON form_stat_bindings BEGIN SELECT RAISE(ABORT,'immutable stats'); END;
CREATE TRIGGER form_stat_binding_delete BEFORE DELETE ON form_stat_bindings BEGIN SELECT RAISE(ABORT,'retained stats'); END;
CREATE TRIGGER variant_stat_binding_update BEFORE UPDATE ON variant_stat_bindings BEGIN SELECT RAISE(ABORT,'immutable stats'); END;
CREATE TRIGGER variant_stat_binding_delete BEFORE DELETE ON variant_stat_bindings BEGIN SELECT RAISE(ABORT,'retained stats'); END;
CREATE TRIGGER form_stat_update BEFORE UPDATE ON form_stat_snapshots BEGIN SELECT RAISE(ABORT,'immutable snapshot'); END;
CREATE TRIGGER form_stat_delete BEFORE DELETE ON form_stat_snapshots BEGIN SELECT RAISE(ABORT,'retained snapshot'); END;
CREATE TRIGGER variant_stat_update BEFORE UPDATE ON variant_stat_snapshots BEGIN SELECT RAISE(ABORT,'immutable snapshot'); END;
CREATE TRIGGER variant_stat_delete BEFORE DELETE ON variant_stat_snapshots BEGIN SELECT RAISE(ABORT,'retained snapshot'); END;
