-- Learning is independent of creature revisions. Children are inserted before their
-- version seal; all circular FKs are checked at the owning transaction's commit.
CREATE TABLE learnsets (
    scope TEXT NOT NULL,
    id TEXT NOT NULL,
    world_id TEXT REFERENCES worlds(id),
    form_id TEXT REFERENCES forms(id),
    variant_id TEXT,
    revision INTEGER NOT NULL CHECK(revision > 0),
    PRIMARY KEY(scope,id),
    UNIQUE(id),
    UNIQUE(scope,id,variant_id),
    UNIQUE(form_id),
    UNIQUE(world_id,variant_id),
    FOREIGN KEY(world_id,variant_id) REFERENCES variants(world_id,id),
    FOREIGN KEY(scope,id,revision) REFERENCES learnset_versions(scope,learnset_id,revision) DEFERRABLE INITIALLY DEFERRED,
    CHECK((scope='' AND world_id IS NULL AND form_id IS NOT NULL AND variant_id IS NULL)
       OR (world_id IS NOT NULL AND scope=world_id AND length(scope)>0 AND form_id IS NULL AND variant_id IS NOT NULL))
) STRICT;
CREATE TABLE learnset_versions (
    scope TEXT NOT NULL,
    learnset_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision>0),
    variant_id TEXT,
    source_scope TEXT NOT NULL DEFAULT '' CHECK(source_scope=''),
    source_learnset_id TEXT,
    source_revision INTEGER,
    records_defined INTEGER NOT NULL CHECK(records_defined IN (0,1)),
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    previous_revision INTEGER,
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND json_extract(before_json,'$.snapshot_version') IS 1)),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 1),
    PRIMARY KEY(scope,learnset_id,revision),
    UNIQUE(scope,learnset_id,revision,variant_id),
    FOREIGN KEY(scope,learnset_id) REFERENCES learnsets(scope,id),
    FOREIGN KEY(scope,learnset_id,variant_id) REFERENCES learnsets(scope,id,variant_id),
    FOREIGN KEY(source_scope,source_learnset_id,source_revision) REFERENCES learnset_versions(scope,learnset_id,revision),
    FOREIGN KEY(scope,learnset_id,previous_revision) REFERENCES learnset_versions(scope,learnset_id,revision),
    CHECK((source_learnset_id IS NULL AND source_revision IS NULL) OR (scope<>'' AND source_learnset_id IS NOT NULL AND source_revision IS NOT NULL AND source_revision>0)),
    CHECK((revision=1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision>1 AND previous_revision IS revision-1 AND before_json IS NOT NULL))
) STRICT;
CREATE TABLE learnset_records (
    scope TEXT NOT NULL,
    learnset_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    slot INTEGER NOT NULL CHECK(slot>0),
    record_id TEXT NOT NULL,
    move_id TEXT NOT NULL,
    move_version INTEGER NOT NULL,
    method TEXT,
    context TEXT,
    condition TEXT,
    threshold_state TEXT NOT NULL CHECK(threshold_state IN ('known','unknown','not_applicable')),
    coefficient INTEGER,
    scale INTEGER,
    PRIMARY KEY(scope,learnset_id,revision,slot),
    UNIQUE(scope,learnset_id,revision,record_id),
    FOREIGN KEY(scope,learnset_id,revision) REFERENCES learnset_versions(scope,learnset_id,revision) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(scope,move_id,move_version) REFERENCES move_versions(scope,move_id,revision),
    CHECK((threshold_state='known' AND coefficient IS NOT NULL AND scale IS NOT NULL AND scale BETWEEN 0 AND 6 AND (scale=0 OR coefficient%10<>0))
       OR (threshold_state<>'known' AND coefficient IS NULL AND scale IS NULL))
) STRICT;
CREATE TABLE learnset_move_mappings (
    scope TEXT NOT NULL CHECK(scope<>''),
    learnset_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    slot INTEGER NOT NULL CHECK(slot>0),
    shared_scope TEXT NOT NULL DEFAULT '' CHECK(shared_scope=''),
    shared_move_id TEXT NOT NULL,
    shared_move_version INTEGER NOT NULL,
    local_move_id TEXT NOT NULL,
    local_move_version INTEGER NOT NULL,
    PRIMARY KEY(scope,learnset_id,revision,slot),
    UNIQUE(scope,learnset_id,revision,shared_move_id),
    UNIQUE(scope,learnset_id,revision,local_move_id),
    FOREIGN KEY(scope,learnset_id,revision) REFERENCES learnset_versions(scope,learnset_id,revision) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(shared_scope,shared_move_id,shared_move_version) REFERENCES move_versions(scope,move_id,revision),
    FOREIGN KEY(scope,local_move_id,local_move_version) REFERENCES move_versions(scope,move_id,revision)
) STRICT;
CREATE TABLE moveset_presets (
    world_id TEXT NOT NULL,
    variant_id TEXT NOT NULL,
    id TEXT NOT NULL,
    slug TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision>0),
    PRIMARY KEY(world_id,variant_id,id),
    UNIQUE(id),
    UNIQUE(world_id,variant_id,slug),
    FOREIGN KEY(world_id,variant_id) REFERENCES variants(world_id,id),
    FOREIGN KEY(world_id,variant_id,id,revision) REFERENCES moveset_preset_versions(world_id,variant_id,preset_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE moveset_preset_versions (
    world_id TEXT NOT NULL,
    variant_id TEXT NOT NULL,
    preset_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision>0),
    slug TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    archived_at INTEGER,
    learnset_id TEXT NOT NULL,
    learnset_version INTEGER NOT NULL,
    ruleset_id TEXT NOT NULL,
    ruleset_version INTEGER NOT NULL,
    validator_version INTEGER NOT NULL CHECK(validator_version>0),
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    previous_revision INTEGER,
    before_json TEXT CHECK(before_json IS NULL OR (json_valid(before_json) AND json_extract(before_json,'$.snapshot_version') IS 1)),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 1),
    PRIMARY KEY(world_id,variant_id,preset_id,revision),
    UNIQUE(world_id,variant_id,preset_id,revision,learnset_id,learnset_version),
    FOREIGN KEY(world_id,variant_id,preset_id) REFERENCES moveset_presets(world_id,variant_id,id),
    FOREIGN KEY(world_id,learnset_id,learnset_version,variant_id) REFERENCES learnset_versions(scope,learnset_id,revision,variant_id),
    FOREIGN KEY(world_id,ruleset_id,ruleset_version) REFERENCES world_ruleset_versions(world_id,ruleset_id,revision),
    FOREIGN KEY(world_id,variant_id,preset_id,previous_revision) REFERENCES moveset_preset_versions(world_id,variant_id,preset_id,revision),
    CHECK((revision=1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision>1 AND previous_revision IS revision-1 AND before_json IS NOT NULL))
) STRICT;
CREATE TABLE moveset_preset_entries (
    world_id TEXT NOT NULL,
    variant_id TEXT NOT NULL,
    preset_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    slot INTEGER NOT NULL CHECK(slot>0),
    learnset_id TEXT NOT NULL,
    learnset_version INTEGER NOT NULL,
    record_id TEXT NOT NULL,
    PRIMARY KEY(world_id,variant_id,preset_id,revision,slot),
    FOREIGN KEY(world_id,variant_id,preset_id,revision,learnset_id,learnset_version) REFERENCES moveset_preset_versions(world_id,variant_id,preset_id,revision,learnset_id,learnset_version) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(world_id,learnset_id,learnset_version,record_id) REFERENCES learnset_records(scope,learnset_id,revision,record_id)
) STRICT;

CREATE TRIGGER learnset_records_sealed_insert BEFORE INSERT ON learnset_records
WHEN EXISTS(SELECT 1 FROM learnset_versions WHERE scope=NEW.scope AND learnset_id=NEW.learnset_id AND revision=NEW.revision)
BEGIN SELECT RAISE(ABORT,'learnset revision is sealed'); END;
CREATE TRIGGER learnset_records_sealed_update BEFORE UPDATE ON learnset_records BEGIN SELECT RAISE(ABORT,'immutable learning record'); END;
CREATE TRIGGER learnset_records_sealed_delete BEFORE DELETE ON learnset_records BEGIN SELECT RAISE(ABORT,'immutable learning record'); END;
CREATE TRIGGER learnset_mappings_sealed_insert BEFORE INSERT ON learnset_move_mappings
WHEN EXISTS(SELECT 1 FROM learnset_versions WHERE scope=NEW.scope AND learnset_id=NEW.learnset_id AND revision=NEW.revision)
BEGIN SELECT RAISE(ABORT,'learnset revision is sealed'); END;
CREATE TRIGGER learnset_mappings_sealed_update BEFORE UPDATE ON learnset_move_mappings BEGIN SELECT RAISE(ABORT,'immutable move mapping'); END;
CREATE TRIGGER learnset_mappings_sealed_delete BEFORE DELETE ON learnset_move_mappings BEGIN SELECT RAISE(ABORT,'immutable move mapping'); END;
CREATE TRIGGER learnset_versions_sealed_update BEFORE UPDATE ON learnset_versions BEGIN SELECT RAISE(ABORT,'immutable learnset version'); END;
CREATE TRIGGER learnset_versions_sealed_delete BEFORE DELETE ON learnset_versions BEGIN SELECT RAISE(ABORT,'immutable learnset version'); END;
CREATE TRIGGER learnset_versions_seal BEFORE INSERT ON learnset_versions
WHEN (NEW.records_defined=0 AND EXISTS(SELECT 1 FROM learnset_records WHERE scope=NEW.scope AND learnset_id=NEW.learnset_id AND revision=NEW.revision))
 OR (NEW.source_learnset_id IS NULL AND EXISTS(SELECT 1 FROM learnset_move_mappings WHERE scope=NEW.scope AND learnset_id=NEW.learnset_id AND revision=NEW.revision))
 OR NEW.variant_id IS NOT (SELECT variant_id FROM learnsets WHERE scope=NEW.scope AND id=NEW.learnset_id)
BEGIN SELECT RAISE(ABORT,'invalid learnset seal'); END;
CREATE TRIGGER preset_entries_sealed_insert BEFORE INSERT ON moveset_preset_entries
WHEN EXISTS(SELECT 1 FROM moveset_preset_versions WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND preset_id=NEW.preset_id AND revision=NEW.revision)
BEGIN SELECT RAISE(ABORT,'preset revision is sealed'); END;
CREATE TRIGGER preset_entries_sealed_update BEFORE UPDATE ON moveset_preset_entries BEGIN SELECT RAISE(ABORT,'immutable preset entry'); END;
CREATE TRIGGER preset_entries_sealed_delete BEFORE DELETE ON moveset_preset_entries BEGIN SELECT RAISE(ABORT,'immutable preset entry'); END;
CREATE TRIGGER preset_versions_sealed_update BEFORE UPDATE ON moveset_preset_versions BEGIN SELECT RAISE(ABORT,'immutable preset version'); END;
CREATE TRIGGER preset_versions_sealed_delete BEFORE DELETE ON moveset_preset_versions BEGIN SELECT RAISE(ABORT,'immutable preset version'); END;
CREATE TRIGGER learnset_owner_immutable BEFORE UPDATE ON learnsets
WHEN NEW.scope IS NOT OLD.scope OR NEW.id IS NOT OLD.id OR NEW.world_id IS NOT OLD.world_id OR NEW.form_id IS NOT OLD.form_id OR NEW.variant_id IS NOT OLD.variant_id OR NEW.revision<>OLD.revision+1
BEGIN SELECT RAISE(ABORT,'immutable learnset identity or invalid revision'); END;
CREATE TRIGGER preset_owner_immutable BEFORE UPDATE ON moveset_presets
WHEN NEW.world_id IS NOT OLD.world_id OR NEW.variant_id IS NOT OLD.variant_id OR NEW.id IS NOT OLD.id OR NEW.revision<>OLD.revision+1
BEGIN SELECT RAISE(ABORT,'immutable preset identity or invalid revision'); END;

CREATE TRIGGER learnset_snapshot_seal BEFORE INSERT ON learnset_versions BEGIN
    SELECT RAISE(ABORT,'learnset head mismatch') WHERE NOT EXISTS(
        SELECT 1 FROM learnsets h WHERE h.scope=NEW.scope AND h.id=NEW.learnset_id AND h.revision=NEW.revision);
    SELECT RAISE(ABORT,'learnset snapshot mismatch') WHERE
        json_extract(NEW.after_json,'$.content.id') IS NOT NEW.learnset_id OR
        json_extract(NEW.after_json,'$.content.world_id') IS NOT nullif(NEW.scope,'') OR
        json_extract(NEW.after_json,'$.content.form_id') IS NOT (SELECT form_id FROM learnsets WHERE scope=NEW.scope AND id=NEW.learnset_id) OR
        json_extract(NEW.after_json,'$.content.variant_id') IS NOT NEW.variant_id OR
        json_extract(NEW.after_json,'$.content.revision') IS NOT NEW.revision OR
        json_extract(NEW.after_json,'$.content.source.learnset_id') IS NOT NEW.source_learnset_id OR
        json_extract(NEW.after_json,'$.content.source.version') IS NOT NEW.source_revision OR
        json_type(NEW.after_json,'$.content.records') IS NOT CASE WHEN NEW.records_defined=1 THEN 'array' ELSE 'null' END OR
        (SELECT count(*) FROM learnset_records WHERE scope=NEW.scope AND learnset_id=NEW.learnset_id AND revision=NEW.revision) <> coalesce(json_array_length(NEW.after_json,'$.content.records'),0) OR
        json_type(NEW.after_json,'$.content.mappings') IS NOT 'array' OR
        (SELECT count(*) FROM learnset_move_mappings WHERE scope=NEW.scope AND learnset_id=NEW.learnset_id AND revision=NEW.revision) <> json_array_length(NEW.after_json,'$.content.mappings');
    SELECT RAISE(ABORT,'learning record mismatch') WHERE EXISTS(
        SELECT 1 FROM learnset_records r JOIN move_versions m ON m.scope=r.scope AND m.move_id=r.move_id AND m.revision=r.move_version
        WHERE r.scope=NEW.scope AND r.learnset_id=NEW.learnset_id AND r.revision=NEW.revision AND (
        json_extract(NEW.after_json,'$.content.records['||(r.slot-1)||'].id') IS NOT r.record_id OR
        json_extract(NEW.after_json,'$.content.records['||(r.slot-1)||'].move_record') IS NOT json_extract(m.after_json,'$.content') OR
        json_extract(NEW.after_json,'$.content.records['||(r.slot-1)||'].method') IS NOT r.method OR
        json_extract(NEW.after_json,'$.content.records['||(r.slot-1)||'].context') IS NOT r.context OR
        json_extract(NEW.after_json,'$.content.records['||(r.slot-1)||'].condition') IS NOT r.condition OR
        json_extract(NEW.after_json,'$.content.records['||(r.slot-1)||'].threshold.state') IS NOT r.threshold_state));
    SELECT RAISE(ABORT,'learning threshold mismatch') WHERE EXISTS(SELECT 1 FROM (
        SELECT r.*,ltrim(CAST(r.coefficient AS TEXT),'-') AS digits,
        json_extract(NEW.after_json,'$.content.records['||(r.slot-1)||'].threshold.value') AS value
        FROM learnset_records r WHERE r.scope=NEW.scope AND r.learnset_id=NEW.learnset_id AND r.revision=NEW.revision)
        WHERE (threshold_state<>'known' AND value IS NOT NULL) OR
        (threshold_state='known' AND value IS NOT CASE WHEN scale=0 THEN CAST(coefficient AS TEXT)
        ELSE (CASE WHEN coefficient<0 THEN '-' ELSE '' END)||CASE WHEN length(digits)<=scale
        THEN '0.'||substr('000000',1,scale-length(digits))||digits
        ELSE substr(digits,1,length(digits)-scale)||'.'||substr(digits,length(digits)-scale+1) END END));
    SELECT RAISE(ABORT,'learning move mapping mismatch') WHERE EXISTS(
        SELECT 1 FROM learnset_move_mappings m JOIN move_versions local ON local.scope=m.scope AND local.move_id=m.local_move_id AND local.revision=m.local_move_version
        WHERE m.scope=NEW.scope AND m.learnset_id=NEW.learnset_id AND m.revision=NEW.revision AND (
        NEW.source_learnset_id IS NULL OR local.source_move_id IS NOT m.shared_move_id OR
        NOT EXISTS(SELECT 1 FROM learnset_records r WHERE r.scope='' AND r.learnset_id=NEW.source_learnset_id AND r.revision=NEW.source_revision AND r.move_id=m.shared_move_id AND r.move_version=m.shared_move_version) OR
        json_extract(NEW.after_json,'$.content.mappings['||(m.slot-1)||'].shared_move_id') IS NOT m.shared_move_id OR
        json_extract(NEW.after_json,'$.content.mappings['||(m.slot-1)||'].local_move.move_id') IS NOT m.local_move_id OR
        json_extract(NEW.after_json,'$.content.mappings['||(m.slot-1)||'].local_move.version') IS NOT m.local_move_version));
    SELECT RAISE(ABORT,'learnset predecessor mismatch') WHERE NEW.revision>1 AND NEW.before_json IS NOT
        (SELECT after_json FROM learnset_versions WHERE scope=NEW.scope AND learnset_id=NEW.learnset_id AND revision=NEW.revision-1);
END;
CREATE TRIGGER preset_snapshot_seal BEFORE INSERT ON moveset_preset_versions BEGIN
    SELECT RAISE(ABORT,'preset head mismatch') WHERE NOT EXISTS(
        SELECT 1 FROM moveset_presets h WHERE h.world_id=NEW.world_id AND h.variant_id=NEW.variant_id AND h.id=NEW.preset_id AND h.revision=NEW.revision AND h.slug=NEW.slug);
    SELECT RAISE(ABORT,'preset snapshot mismatch') WHERE
        json_extract(NEW.after_json,'$.content.id') IS NOT NEW.preset_id OR
        json_extract(NEW.after_json,'$.content.world_id') IS NOT NEW.world_id OR
        json_extract(NEW.after_json,'$.content.variant_id') IS NOT NEW.variant_id OR
        json_extract(NEW.after_json,'$.content.revision') IS NOT NEW.revision OR
        json_extract(NEW.after_json,'$.content.fields.slug') IS NOT NEW.slug OR
        json_extract(NEW.after_json,'$.content.fields.display_name') IS NOT NEW.display_name OR
        json_extract(NEW.after_json,'$.content.fields.description') IS NOT NEW.description OR
        json_extract(NEW.after_json,'$.content.archived_at') IS NOT NEW.archived_at OR
        json_extract(NEW.after_json,'$.content.learnset.learnset_id') IS NOT NEW.learnset_id OR
        json_extract(NEW.after_json,'$.content.learnset.version') IS NOT NEW.learnset_version OR
        json_extract(NEW.after_json,'$.content.ruleset.ruleset_id') IS NOT NEW.ruleset_id OR
        json_extract(NEW.after_json,'$.content.ruleset.version') IS NOT NEW.ruleset_version OR
        json_extract(NEW.after_json,'$.content.validator_version') IS NOT NEW.validator_version OR
        json_type(NEW.after_json,'$.content.entries') IS NOT 'array' OR
        (SELECT count(*) FROM moveset_preset_entries WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND preset_id=NEW.preset_id AND revision=NEW.revision) <> json_array_length(NEW.after_json,'$.content.entries');
    SELECT RAISE(ABORT,'preset selection mismatch') WHERE EXISTS(
        SELECT 1 FROM moveset_preset_entries e WHERE e.world_id=NEW.world_id AND e.variant_id=NEW.variant_id AND e.preset_id=NEW.preset_id AND e.revision=NEW.revision AND (
        e.learnset_id IS NOT NEW.learnset_id OR e.learnset_version IS NOT NEW.learnset_version OR
        json_extract(NEW.after_json,'$.content.entries['||(e.slot-1)||']') IS NOT e.record_id));
    SELECT RAISE(ABORT,'preset predecessor mismatch') WHERE NEW.revision>1 AND NEW.before_json IS NOT
        (SELECT after_json FROM moveset_preset_versions WHERE world_id=NEW.world_id AND variant_id=NEW.variant_id AND preset_id=NEW.preset_id AND revision=NEW.revision-1);
END;
