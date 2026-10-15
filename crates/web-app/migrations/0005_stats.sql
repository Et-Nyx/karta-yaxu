-- Independent definitions and one ordered roster per world. Empty scope is shared.
CREATE TABLE stat_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    scope TEXT NOT NULL,
    world_id TEXT REFERENCES worlds(id),
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    revision INTEGER NOT NULL CHECK(revision > 0),
    CHECK(scope=coalesce(world_id,'')),
    UNIQUE(scope,id), UNIQUE(scope,slug),
    FOREIGN KEY(scope,id,revision) REFERENCES stat_versions(scope,stat_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE stat_versions (
    scope TEXT NOT NULL,
    stat_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision>0),
    source_scope TEXT NOT NULL DEFAULT '' CHECK(source_scope=''),
    source_stat_id TEXT,
    source_revision INTEGER,
    slug TEXT NOT NULL CHECK(length(slug) BETWEEN 1 AND 80),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 100),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 2000),
    archived_at INTEGER,
    whole_only INTEGER NOT NULL CHECK(whole_only IN (0,1)),
    minimum_coefficient INTEGER,
    minimum_scale INTEGER,
    maximum_coefficient INTEGER,
    maximum_scale INTEGER,
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    previous_revision INTEGER,
    before_json TEXT CHECK(before_json IS NULL OR json_valid(before_json)),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 1),
    PRIMARY KEY(scope,stat_id,revision),
    CHECK((source_stat_id IS NULL)=(source_revision IS NULL)),
    CHECK(source_revision IS NULL OR source_revision>0),
    CHECK(scope<>'' OR source_stat_id IS NULL),
    CHECK((minimum_coefficient IS NULL)=(minimum_scale IS NULL)),
    CHECK((maximum_coefficient IS NULL)=(maximum_scale IS NULL)),
    CHECK(minimum_scale IS NULL OR (minimum_scale BETWEEN 0 AND 6 AND (minimum_scale=0 OR minimum_coefficient%10<>0))),
    CHECK(maximum_scale IS NULL OR (maximum_scale BETWEEN 0 AND 6 AND (maximum_scale=0 OR maximum_coefficient%10<>0))),
    CHECK((revision=1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision>1 AND previous_revision IS NOT NULL AND previous_revision=revision-1 AND before_json IS NOT NULL)),
    FOREIGN KEY(scope,stat_id) REFERENCES stat_definitions(scope,id),
    FOREIGN KEY(scope,stat_id,previous_revision) REFERENCES stat_versions(scope,stat_id,revision),
    FOREIGN KEY(source_scope,source_stat_id,source_revision) REFERENCES stat_versions(scope,stat_id,revision)
) STRICT;
CREATE TABLE stat_rosters (
    world_id TEXT PRIMARY KEY NOT NULL REFERENCES worlds(id),
    id TEXT NOT NULL UNIQUE,
    revision INTEGER NOT NULL CHECK(revision>0),
    UNIQUE(world_id,id),
    FOREIGN KEY(world_id,id,revision) REFERENCES stat_roster_versions(world_id,roster_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE stat_roster_versions (
    world_id TEXT NOT NULL,
    roster_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision>0),
    audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id),
    previous_revision INTEGER,
    before_json TEXT CHECK(before_json IS NULL OR json_valid(before_json)),
    after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 1),
    PRIMARY KEY(world_id,roster_id,revision),
    CHECK((revision=1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision>1 AND previous_revision IS NOT NULL AND previous_revision=revision-1 AND before_json IS NOT NULL)),
    FOREIGN KEY(world_id,roster_id) REFERENCES stat_rosters(world_id,id),
    FOREIGN KEY(world_id,roster_id,previous_revision) REFERENCES stat_roster_versions(world_id,roster_id,revision)
) STRICT;
CREATE TABLE stat_roster_entries (
    world_id TEXT NOT NULL,
    roster_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    slot INTEGER NOT NULL CHECK(slot>0),
    scope TEXT NOT NULL CHECK(scope=world_id AND scope<>''),
    stat_id TEXT NOT NULL,
    stat_version INTEGER NOT NULL CHECK(stat_version>0),
    PRIMARY KEY(world_id,roster_id,revision,slot),
    UNIQUE(world_id,roster_id,revision,stat_id),
    FOREIGN KEY(scope,stat_id,stat_version) REFERENCES stat_versions(scope,stat_id,revision),
    FOREIGN KEY(world_id,roster_id,revision) REFERENCES stat_roster_versions(world_id,roster_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TRIGGER stat_identity BEFORE UPDATE OF id,scope,world_id ON stat_definitions
WHEN NEW.id<>OLD.id OR NEW.scope<>OLD.scope OR NEW.world_id IS NOT OLD.world_id
BEGIN SELECT RAISE(ABORT,'immutable stat identity'); END;
CREATE TRIGGER stat_roster_identity BEFORE UPDATE OF id,world_id ON stat_rosters
WHEN NEW.id<>OLD.id OR NEW.world_id<>OLD.world_id
BEGIN SELECT RAISE(ABORT,'immutable stat roster identity'); END;
CREATE TRIGGER stat_head_advance BEFORE UPDATE OF revision ON stat_definitions
WHEN NEW.revision<>OLD.revision+1
BEGIN SELECT RAISE(ABORT,'stat head must advance exactly once'); END;
CREATE TRIGGER stat_roster_head_advance BEFORE UPDATE OF revision ON stat_rosters
WHEN NEW.revision<>OLD.revision+1
BEGIN SELECT RAISE(ABORT,'stat roster head must advance exactly once'); END;
CREATE TRIGGER stat_versions_update BEFORE UPDATE ON stat_versions BEGIN SELECT RAISE(ABORT,'immutable stat version'); END;
CREATE TRIGGER stat_versions_delete BEFORE DELETE ON stat_versions BEGIN SELECT RAISE(ABORT,'retained stat version'); END;
CREATE TRIGGER stat_roster_versions_update BEFORE UPDATE ON stat_roster_versions BEGIN SELECT RAISE(ABORT,'immutable stat roster version'); END;
CREATE TRIGGER stat_roster_versions_delete BEFORE DELETE ON stat_roster_versions BEGIN SELECT RAISE(ABORT,'retained stat roster version'); END;
CREATE TRIGGER stat_roster_entries_update BEFORE UPDATE ON stat_roster_entries BEGIN SELECT RAISE(ABORT,'immutable stat roster entry'); END;
CREATE TRIGGER stat_roster_entries_delete BEFORE DELETE ON stat_roster_entries BEGIN SELECT RAISE(ABORT,'retained stat roster entry'); END;
CREATE TRIGGER stat_roster_entries_insert BEFORE INSERT ON stat_roster_entries
WHEN EXISTS(SELECT 1 FROM stat_roster_versions WHERE world_id=NEW.world_id AND roster_id=NEW.roster_id AND revision=NEW.revision)
BEGIN SELECT RAISE(ABORT,'sealed stat roster'); END;
CREATE TRIGGER stat_version_seal BEFORE INSERT ON stat_versions BEGIN
    SELECT RAISE(ABORT,'stat head seal mismatch') WHERE NOT EXISTS(
      SELECT 1 FROM stat_definitions WHERE scope=NEW.scope AND id=NEW.stat_id AND revision=NEW.revision AND slug=NEW.slug);
    SELECT RAISE(ABORT,'stat snapshot mismatch') WHERE
        json_extract(NEW.after_json,'$.content.id') IS NOT NEW.stat_id OR
        json_extract(NEW.after_json,'$.content.world_id') IS NOT nullif(NEW.scope,'') OR
        json_extract(NEW.after_json,'$.content.revision') IS NOT NEW.revision OR
        json_extract(NEW.after_json,'$.content.fields.slug') IS NOT NEW.slug OR
        json_extract(NEW.after_json,'$.content.fields.display_name') IS NOT NEW.display_name OR
        json_extract(NEW.after_json,'$.content.fields.description') IS NOT NEW.description OR
        json_extract(NEW.after_json,'$.content.archived_at') IS NOT NEW.archived_at OR
        json_extract(NEW.after_json,'$.content.source.stat_id') IS NOT NEW.source_stat_id OR
        json_extract(NEW.after_json,'$.content.source.version') IS NOT NEW.source_revision OR
        json_extract(NEW.after_json,'$.content.constraints.whole_only') IS NOT NEW.whole_only;
    -- Render typed coefficients as canonical decimal text. String operations avoid
    -- both SQLite's saturating text-to-integer cast and abs(i64::MIN) overflow.
    SELECT RAISE(ABORT,'stat bound mismatch') WHERE EXISTS(
      SELECT 1 FROM (
        SELECT *,ltrim(CAST(coefficient AS TEXT),'-') AS digits FROM (
          SELECT NEW.minimum_coefficient AS coefficient,NEW.minimum_scale AS scale,
            json_type(NEW.after_json,'$.content.constraints.minimum') AS kind,
            json_extract(NEW.after_json,'$.content.constraints.minimum') AS value
          UNION ALL
          SELECT NEW.maximum_coefficient,NEW.maximum_scale,
            json_type(NEW.after_json,'$.content.constraints.maximum'),
            json_extract(NEW.after_json,'$.content.constraints.maximum')
        )
      ) WHERE
        (coefficient IS NULL AND kind IS NOT 'null') OR
        (coefficient IS NOT NULL AND (kind IS NOT 'text' OR value IS NOT
          CASE WHEN scale=0 THEN CAST(coefficient AS TEXT)
          ELSE (CASE WHEN coefficient<0 THEN '-' ELSE '' END) ||
            CASE WHEN length(digits)<=scale
              THEN '0.' || substr('000000',1,scale-length(digits)) || digits
              ELSE substr(digits,1,length(digits)-scale) || '.' || substr(digits,length(digits)-scale+1)
            END
          END)));
    SELECT RAISE(ABORT,'stat predecessor mismatch') WHERE NEW.revision>1 AND NEW.before_json IS NOT
      (SELECT after_json FROM stat_versions WHERE scope=NEW.scope AND stat_id=NEW.stat_id AND revision=NEW.revision-1);
END;
CREATE TRIGGER stat_roster_version_seal BEFORE INSERT ON stat_roster_versions BEGIN
    SELECT RAISE(ABORT,'stat roster head seal mismatch') WHERE NOT EXISTS(
      SELECT 1 FROM stat_rosters WHERE world_id=NEW.world_id AND id=NEW.roster_id AND revision=NEW.revision);
    SELECT RAISE(ABORT,'stat roster snapshot mismatch') WHERE
      json_extract(NEW.after_json,'$.content.id') IS NOT NEW.roster_id OR
      json_extract(NEW.after_json,'$.content.world_id') IS NOT NEW.world_id OR
      json_extract(NEW.after_json,'$.content.revision') IS NOT NEW.revision OR
      json_type(NEW.after_json,'$.content.entries') IS NOT 'array' OR
      (SELECT count(*) FROM stat_roster_entries WHERE world_id=NEW.world_id AND roster_id=NEW.roster_id AND revision=NEW.revision) <> json_array_length(NEW.after_json,'$.content.entries') OR
      EXISTS(SELECT 1 FROM stat_roster_entries e JOIN stat_versions s ON s.scope=e.scope AND s.stat_id=e.stat_id AND s.revision=e.stat_version
        WHERE e.world_id=NEW.world_id AND e.roster_id=NEW.roster_id AND e.revision=NEW.revision AND (
          json_extract(NEW.after_json,'$.content.entries['||(e.slot-1)||'].id') IS NOT e.stat_id OR
          json_extract(NEW.after_json,'$.content.entries['||(e.slot-1)||'].revision') IS NOT e.stat_version OR
          json_extract(NEW.after_json,'$.content.entries['||(e.slot-1)||']') IS NOT json_extract(s.after_json,'$.content')));
    SELECT RAISE(ABORT,'stat roster predecessor mismatch') WHERE NEW.revision>1 AND NEW.before_json IS NOT
      (SELECT after_json FROM stat_roster_versions WHERE world_id=NEW.world_id AND roster_id=NEW.roster_id AND revision=NEW.revision-1);
END;
