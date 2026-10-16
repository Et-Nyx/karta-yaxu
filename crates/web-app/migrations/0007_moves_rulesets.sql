-- Independent scoped move data and singleton world configuration. Versions seal last.
CREATE TABLE move_attribute_definitions (
 id TEXT PRIMARY KEY NOT NULL, scope TEXT NOT NULL, world_id TEXT REFERENCES worlds(id),
 slug TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 CHECK(scope=coalesce(world_id,'')), UNIQUE(scope,id), UNIQUE(scope,slug),
 FOREIGN KEY(scope,id,revision) REFERENCES move_attribute_versions(scope,attribute_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE move_attribute_versions (
 scope TEXT NOT NULL, attribute_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 source_scope TEXT NOT NULL DEFAULT '' CHECK(source_scope=''), source_attribute_id TEXT, source_revision INTEGER,
 slug TEXT NOT NULL, display_name TEXT NOT NULL, description TEXT NOT NULL, unit TEXT NOT NULL, archived_at INTEGER,
 whole_only INTEGER NOT NULL CHECK(whole_only IN (0,1)),
 minimum_coefficient INTEGER, minimum_scale INTEGER, maximum_coefficient INTEGER, maximum_scale INTEGER,
 audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id), previous_revision INTEGER,
 before_json TEXT CHECK(before_json IS NULL OR json_valid(before_json)),
 after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 1),
 PRIMARY KEY(scope,attribute_id,revision),
 CHECK((source_attribute_id IS NULL)=(source_revision IS NULL)), CHECK(scope<>'' OR source_attribute_id IS NULL),
 CHECK((minimum_coefficient IS NULL)=(minimum_scale IS NULL)), CHECK((maximum_coefficient IS NULL)=(maximum_scale IS NULL)),
 CHECK(minimum_scale IS NULL OR (minimum_scale BETWEEN 0 AND 6 AND (minimum_scale=0 OR minimum_coefficient%10<>0))),
 CHECK(maximum_scale IS NULL OR (maximum_scale BETWEEN 0 AND 6 AND (maximum_scale=0 OR maximum_coefficient%10<>0))),
 CHECK((revision=1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision>1 AND previous_revision IS revision-1 AND before_json IS NOT NULL)),
 FOREIGN KEY(scope,attribute_id) REFERENCES move_attribute_definitions(scope,id),
 FOREIGN KEY(scope,attribute_id,previous_revision) REFERENCES move_attribute_versions(scope,attribute_id,revision),
 FOREIGN KEY(source_scope,source_attribute_id,source_revision) REFERENCES move_attribute_versions(scope,attribute_id,revision)
) STRICT;
CREATE TABLE moves (
 id TEXT PRIMARY KEY NOT NULL, scope TEXT NOT NULL, world_id TEXT REFERENCES worlds(id), slug TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision>0), CHECK(scope=coalesce(world_id,'')), UNIQUE(scope,id), UNIQUE(scope,slug),
 FOREIGN KEY(scope,id,revision) REFERENCES move_versions(scope,move_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE move_versions (
 scope TEXT NOT NULL, move_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 source_scope TEXT NOT NULL DEFAULT '' CHECK(source_scope=''), source_move_id TEXT, source_revision INTEGER,
 slug TEXT NOT NULL, display_name TEXT NOT NULL, description TEXT NOT NULL, archived_at INTEGER,
 attributes_defined INTEGER NOT NULL CHECK(attributes_defined IN (0,1)),
 audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id), previous_revision INTEGER,
 before_json TEXT CHECK(before_json IS NULL OR json_valid(before_json)),
 after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 1),
 PRIMARY KEY(scope,move_id,revision),
 CHECK((source_move_id IS NULL)=(source_revision IS NULL)), CHECK(scope<>'' OR source_move_id IS NULL),
 CHECK((revision=1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision>1 AND previous_revision IS revision-1 AND before_json IS NOT NULL)),
 FOREIGN KEY(scope,move_id) REFERENCES moves(scope,id),
 FOREIGN KEY(scope,move_id,previous_revision) REFERENCES move_versions(scope,move_id,revision),
 FOREIGN KEY(source_scope,source_move_id,source_revision) REFERENCES move_versions(scope,move_id,revision)
) STRICT;
CREATE TABLE move_attribute_bindings (
 scope TEXT NOT NULL, move_id TEXT NOT NULL, revision INTEGER NOT NULL, slot INTEGER NOT NULL CHECK(slot>0),
 attribute_id TEXT NOT NULL, attribute_version INTEGER NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('known','unknown','not_applicable')), coefficient INTEGER, scale INTEGER,
 PRIMARY KEY(scope,move_id,revision,slot), UNIQUE(scope,move_id,revision,attribute_id),
 CHECK((state='known' AND coefficient IS NOT NULL AND scale IS NOT NULL) OR (state<>'known' AND coefficient IS NULL AND scale IS NULL)),
 CHECK(scale IS NULL OR (scale BETWEEN 0 AND 6 AND (scale=0 OR coefficient%10<>0))),
 FOREIGN KEY(scope,attribute_id,attribute_version) REFERENCES move_attribute_versions(scope,attribute_id,revision),
 FOREIGN KEY(scope,move_id,revision) REFERENCES move_versions(scope,move_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE move_attribute_mappings (
 scope TEXT NOT NULL CHECK(scope<>''), move_id TEXT NOT NULL, revision INTEGER NOT NULL, slot INTEGER NOT NULL CHECK(slot>0),
 shared_scope TEXT NOT NULL DEFAULT '' CHECK(shared_scope=''), shared_attribute_id TEXT NOT NULL,
 local_attribute_id TEXT NOT NULL, local_attribute_version INTEGER NOT NULL,
 PRIMARY KEY(scope,move_id,revision,slot), UNIQUE(scope,move_id,revision,shared_attribute_id), UNIQUE(scope,move_id,revision,local_attribute_id),
 FOREIGN KEY(shared_scope,shared_attribute_id) REFERENCES move_attribute_definitions(scope,id),
 FOREIGN KEY(scope,local_attribute_id,local_attribute_version) REFERENCES move_attribute_versions(scope,attribute_id,revision),
 FOREIGN KEY(scope,move_id,revision) REFERENCES move_versions(scope,move_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE world_rulesets (
 world_id TEXT PRIMARY KEY NOT NULL REFERENCES worlds(id), id TEXT NOT NULL UNIQUE, revision INTEGER NOT NULL CHECK(revision>0),
 UNIQUE(world_id,id),
 FOREIGN KEY(world_id,id,revision) REFERENCES world_ruleset_versions(world_id,ruleset_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE world_ruleset_versions (
 world_id TEXT NOT NULL, ruleset_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
 slug TEXT NOT NULL, display_name TEXT NOT NULL, description TEXT NOT NULL,
 expectations_defined INTEGER NOT NULL CHECK(expectations_defined IN (0,1)),
 roster_id TEXT, roster_version INTEGER, chart_id TEXT, chart_version INTEGER,
 max_moves INTEGER CHECK(max_moves IS NULL OR max_moves>0),
 repeat_policy TEXT NOT NULL CHECK(repeat_policy IN ('unspecified','allow','forbid')), notes TEXT NOT NULL,
 audit_id TEXT NOT NULL UNIQUE REFERENCES audit_events(id), previous_revision INTEGER,
 before_json TEXT CHECK(before_json IS NULL OR json_valid(before_json)),
 after_json TEXT NOT NULL CHECK(json_valid(after_json) AND json_extract(after_json,'$.snapshot_version') IS 1),
 PRIMARY KEY(world_id,ruleset_id,revision),
 CHECK((roster_id IS NULL)=(roster_version IS NULL)), CHECK((chart_id IS NULL)=(chart_version IS NULL)),
 CHECK((revision=1 AND previous_revision IS NULL AND before_json IS NULL) OR (revision>1 AND previous_revision IS revision-1 AND before_json IS NOT NULL)),
 FOREIGN KEY(world_id,ruleset_id) REFERENCES world_rulesets(world_id,id),
 FOREIGN KEY(world_id,ruleset_id,previous_revision) REFERENCES world_ruleset_versions(world_id,ruleset_id,revision),
 FOREIGN KEY(world_id,roster_id,roster_version) REFERENCES stat_roster_versions(world_id,roster_id,revision),
 FOREIGN KEY(world_id,chart_id,chart_version) REFERENCES chart_versions(scope,chart_id,revision)
) STRICT;
CREATE TABLE ruleset_attribute_expectations (
 world_id TEXT NOT NULL, ruleset_id TEXT NOT NULL, revision INTEGER NOT NULL, slot INTEGER NOT NULL CHECK(slot>0),
 attribute_id TEXT NOT NULL, attribute_version INTEGER NOT NULL,
 required INTEGER NOT NULL CHECK(required IN (0,1)), allow_not_applicable INTEGER NOT NULL CHECK(allow_not_applicable IN (0,1)),
 whole_only INTEGER NOT NULL CHECK(whole_only IN (0,1)),
 minimum_coefficient INTEGER, minimum_scale INTEGER, maximum_coefficient INTEGER, maximum_scale INTEGER,
 PRIMARY KEY(world_id,ruleset_id,revision,slot), UNIQUE(world_id,ruleset_id,revision,attribute_id),
 CHECK((minimum_coefficient IS NULL)=(minimum_scale IS NULL)), CHECK((maximum_coefficient IS NULL)=(maximum_scale IS NULL)),
 CHECK(minimum_scale IS NULL OR (minimum_scale BETWEEN 0 AND 6 AND (minimum_scale=0 OR minimum_coefficient%10<>0))),
 CHECK(maximum_scale IS NULL OR (maximum_scale BETWEEN 0 AND 6 AND (maximum_scale=0 OR maximum_coefficient%10<>0))),
 FOREIGN KEY(world_id,attribute_id,attribute_version) REFERENCES move_attribute_versions(scope,attribute_id,revision),
 FOREIGN KEY(world_id,ruleset_id,revision) REFERENCES world_ruleset_versions(world_id,ruleset_id,revision) DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE TRIGGER move_attribute_identity BEFORE UPDATE OF id,scope,world_id ON move_attribute_definitions
 WHEN NEW.id<>OLD.id OR NEW.scope<>OLD.scope OR NEW.world_id IS NOT OLD.world_id BEGIN SELECT RAISE(ABORT,'immutable attribute identity'); END;
CREATE TRIGGER move_identity BEFORE UPDATE OF id,scope,world_id ON moves
 WHEN NEW.id<>OLD.id OR NEW.scope<>OLD.scope OR NEW.world_id IS NOT OLD.world_id BEGIN SELECT RAISE(ABORT,'immutable move identity'); END;
CREATE TRIGGER ruleset_identity BEFORE UPDATE OF id,world_id ON world_rulesets
 WHEN NEW.id<>OLD.id OR NEW.world_id<>OLD.world_id BEGIN SELECT RAISE(ABORT,'immutable ruleset identity'); END;
CREATE TRIGGER move_attribute_advance BEFORE UPDATE OF revision ON move_attribute_definitions WHEN NEW.revision<>OLD.revision+1 BEGIN SELECT RAISE(ABORT,'attribute revision must advance once'); END;
CREATE TRIGGER move_advance BEFORE UPDATE OF revision ON moves WHEN NEW.revision<>OLD.revision+1 BEGIN SELECT RAISE(ABORT,'move revision must advance once'); END;
CREATE TRIGGER ruleset_advance BEFORE UPDATE OF revision ON world_rulesets WHEN NEW.revision<>OLD.revision+1 BEGIN SELECT RAISE(ABORT,'ruleset revision must advance once'); END;
CREATE TRIGGER move_attribute_versions_update BEFORE UPDATE ON move_attribute_versions BEGIN SELECT RAISE(ABORT,'immutable attribute version'); END;
CREATE TRIGGER move_attribute_versions_delete BEFORE DELETE ON move_attribute_versions BEGIN SELECT RAISE(ABORT,'retained attribute version'); END;
CREATE TRIGGER move_versions_update BEFORE UPDATE ON move_versions BEGIN SELECT RAISE(ABORT,'immutable move version'); END;
CREATE TRIGGER move_versions_delete BEFORE DELETE ON move_versions BEGIN SELECT RAISE(ABORT,'retained move version'); END;
CREATE TRIGGER world_ruleset_versions_update BEFORE UPDATE ON world_ruleset_versions BEGIN SELECT RAISE(ABORT,'immutable ruleset version'); END;
CREATE TRIGGER world_ruleset_versions_delete BEFORE DELETE ON world_ruleset_versions BEGIN SELECT RAISE(ABORT,'retained ruleset version'); END;
CREATE TRIGGER move_attribute_bindings_update BEFORE UPDATE ON move_attribute_bindings BEGIN SELECT RAISE(ABORT,'immutable move binding'); END;
CREATE TRIGGER move_attribute_bindings_delete BEFORE DELETE ON move_attribute_bindings BEGIN SELECT RAISE(ABORT,'retained move binding'); END;
CREATE TRIGGER move_attribute_bindings_insert BEFORE INSERT ON move_attribute_bindings WHEN EXISTS(SELECT 1 FROM move_versions WHERE scope=NEW.scope AND move_id=NEW.move_id AND revision=NEW.revision) BEGIN SELECT RAISE(ABORT,'sealed move'); END;
CREATE TRIGGER move_attribute_mappings_update BEFORE UPDATE ON move_attribute_mappings BEGIN SELECT RAISE(ABORT,'immutable move mapping'); END;
CREATE TRIGGER move_attribute_mappings_delete BEFORE DELETE ON move_attribute_mappings BEGIN SELECT RAISE(ABORT,'retained move mapping'); END;
CREATE TRIGGER move_attribute_mappings_insert BEFORE INSERT ON move_attribute_mappings WHEN EXISTS(SELECT 1 FROM move_versions WHERE scope=NEW.scope AND move_id=NEW.move_id AND revision=NEW.revision) BEGIN SELECT RAISE(ABORT,'sealed move'); END;
CREATE TRIGGER ruleset_attribute_expectations_update BEFORE UPDATE ON ruleset_attribute_expectations BEGIN SELECT RAISE(ABORT,'immutable expectation'); END;
CREATE TRIGGER ruleset_attribute_expectations_delete BEFORE DELETE ON ruleset_attribute_expectations BEGIN SELECT RAISE(ABORT,'retained expectation'); END;
CREATE TRIGGER ruleset_attribute_expectations_insert BEFORE INSERT ON ruleset_attribute_expectations WHEN EXISTS(SELECT 1 FROM world_ruleset_versions WHERE world_id=NEW.world_id AND ruleset_id=NEW.ruleset_id AND revision=NEW.revision) BEGIN SELECT RAISE(ABORT,'sealed ruleset'); END;

CREATE TRIGGER move_attribute_version_seal BEFORE INSERT ON move_attribute_versions BEGIN
 SELECT RAISE(ABORT,'attribute head mismatch') WHERE NOT EXISTS(SELECT 1 FROM move_attribute_definitions WHERE scope=NEW.scope AND id=NEW.attribute_id AND revision=NEW.revision AND slug=NEW.slug);
 SELECT RAISE(ABORT,'attribute snapshot mismatch') WHERE
 json_extract(NEW.after_json,'$.content.id') IS NOT NEW.attribute_id OR json_extract(NEW.after_json,'$.content.world_id') IS NOT nullif(NEW.scope,'') OR
 json_extract(NEW.after_json,'$.content.revision') IS NOT NEW.revision OR json_extract(NEW.after_json,'$.content.fields.slug') IS NOT NEW.slug OR
 json_extract(NEW.after_json,'$.content.fields.display_name') IS NOT NEW.display_name OR json_extract(NEW.after_json,'$.content.fields.description') IS NOT NEW.description OR
 json_extract(NEW.after_json,'$.content.unit') IS NOT NEW.unit OR json_extract(NEW.after_json,'$.content.archived_at') IS NOT NEW.archived_at OR
 json_extract(NEW.after_json,'$.content.source.attribute_id') IS NOT NEW.source_attribute_id OR json_extract(NEW.after_json,'$.content.source.version') IS NOT NEW.source_revision OR
 json_extract(NEW.after_json,'$.content.constraints.whole_only') IS NOT NEW.whole_only;
 SELECT RAISE(ABORT,'attribute bound mismatch') WHERE EXISTS(SELECT 1 FROM (
 SELECT *,ltrim(CAST(coefficient AS TEXT),'-') AS digits FROM (
 SELECT NEW.minimum_coefficient AS coefficient,NEW.minimum_scale AS scale,json_type(NEW.after_json,'$.content.constraints.minimum') AS kind,json_extract(NEW.after_json,'$.content.constraints.minimum') AS value
 UNION ALL SELECT NEW.maximum_coefficient,NEW.maximum_scale,json_type(NEW.after_json,'$.content.constraints.maximum'),json_extract(NEW.after_json,'$.content.constraints.maximum')))
 WHERE (coefficient IS NULL AND kind IS NOT 'null') OR (coefficient IS NOT NULL AND (kind IS NOT 'text' OR value IS NOT
 CASE WHEN scale=0 THEN CAST(coefficient AS TEXT) ELSE (CASE WHEN coefficient<0 THEN '-' ELSE '' END)||CASE WHEN length(digits)<=scale THEN '0.'||substr('000000',1,scale-length(digits))||digits ELSE substr(digits,1,length(digits)-scale)||'.'||substr(digits,length(digits)-scale+1) END END)));
 SELECT RAISE(ABORT,'attribute predecessor mismatch') WHERE NEW.revision>1 AND NEW.before_json IS NOT (SELECT after_json FROM move_attribute_versions WHERE scope=NEW.scope AND attribute_id=NEW.attribute_id AND revision=NEW.revision-1);
END;
CREATE TRIGGER move_version_seal BEFORE INSERT ON move_versions BEGIN
 SELECT RAISE(ABORT,'move head mismatch') WHERE NOT EXISTS(SELECT 1 FROM moves WHERE scope=NEW.scope AND id=NEW.move_id AND revision=NEW.revision AND slug=NEW.slug);
 SELECT RAISE(ABORT,'move snapshot mismatch') WHERE
 json_extract(NEW.after_json,'$.content.id') IS NOT NEW.move_id OR json_extract(NEW.after_json,'$.content.world_id') IS NOT nullif(NEW.scope,'') OR
 json_extract(NEW.after_json,'$.content.revision') IS NOT NEW.revision OR json_extract(NEW.after_json,'$.content.fields.slug') IS NOT NEW.slug OR
 json_extract(NEW.after_json,'$.content.fields.display_name') IS NOT NEW.display_name OR json_extract(NEW.after_json,'$.content.fields.description') IS NOT NEW.description OR
 json_extract(NEW.after_json,'$.content.archived_at') IS NOT NEW.archived_at OR json_extract(NEW.after_json,'$.content.source.move_id') IS NOT NEW.source_move_id OR
 json_extract(NEW.after_json,'$.content.source.version') IS NOT NEW.source_revision OR
 json_type(NEW.after_json,'$.content.attributes') IS NOT CASE WHEN NEW.attributes_defined=1 THEN 'array' ELSE 'null' END OR
 (SELECT count(*) FROM move_attribute_bindings WHERE scope=NEW.scope AND move_id=NEW.move_id AND revision=NEW.revision) <> coalesce(json_array_length(NEW.after_json,'$.content.attributes'),0) OR
 json_type(NEW.after_json,'$.content.mappings') IS NOT 'array' OR
 (SELECT count(*) FROM move_attribute_mappings WHERE scope=NEW.scope AND move_id=NEW.move_id AND revision=NEW.revision) <> json_array_length(NEW.after_json,'$.content.mappings');
 SELECT RAISE(ABORT,'move dependency mismatch') WHERE EXISTS(SELECT 1 FROM move_attribute_bindings b JOIN move_attribute_versions a ON a.scope=b.scope AND a.attribute_id=b.attribute_id AND a.revision=b.attribute_version
 WHERE b.scope=NEW.scope AND b.move_id=NEW.move_id AND b.revision=NEW.revision AND (
 json_extract(NEW.after_json,'$.content.attributes['||(b.slot-1)||'].definition') IS NOT json_extract(a.after_json,'$.content') OR
 json_extract(NEW.after_json,'$.content.attributes['||(b.slot-1)||'].value.state') IS NOT b.state));
 SELECT RAISE(ABORT,'move value mismatch') WHERE EXISTS(SELECT 1 FROM (
 SELECT b.*,ltrim(CAST(b.coefficient AS TEXT),'-') AS digits,json_extract(NEW.after_json,'$.content.attributes['||(b.slot-1)||'].value.value') AS value FROM move_attribute_bindings b
 WHERE b.scope=NEW.scope AND b.move_id=NEW.move_id AND b.revision=NEW.revision) WHERE
 (state<>'known' AND value IS NOT NULL) OR (state='known' AND value IS NOT CASE WHEN scale=0 THEN CAST(coefficient AS TEXT) ELSE (CASE WHEN coefficient<0 THEN '-' ELSE '' END)||CASE WHEN length(digits)<=scale THEN '0.'||substr('000000',1,scale-length(digits))||digits ELSE substr(digits,1,length(digits)-scale)||'.'||substr(digits,length(digits)-scale+1) END END));
 SELECT RAISE(ABORT,'move mapping mismatch') WHERE EXISTS(SELECT 1 FROM move_attribute_mappings m JOIN move_attribute_versions a ON a.scope=m.scope AND a.attribute_id=m.local_attribute_id AND a.revision=m.local_attribute_version
 WHERE m.scope=NEW.scope AND m.move_id=NEW.move_id AND m.revision=NEW.revision AND (
 NEW.source_move_id IS NULL OR a.source_attribute_id IS NOT m.shared_attribute_id OR
 json_extract(NEW.after_json,'$.content.mappings['||(m.slot-1)||'].shared_attribute_id') IS NOT m.shared_attribute_id OR
 json_extract(NEW.after_json,'$.content.mappings['||(m.slot-1)||'].local_attribute.attribute_id') IS NOT m.local_attribute_id OR
 json_extract(NEW.after_json,'$.content.mappings['||(m.slot-1)||'].local_attribute.version') IS NOT m.local_attribute_version));
 SELECT RAISE(ABORT,'move predecessor mismatch') WHERE NEW.revision>1 AND NEW.before_json IS NOT (SELECT after_json FROM move_versions WHERE scope=NEW.scope AND move_id=NEW.move_id AND revision=NEW.revision-1);
END;
CREATE TRIGGER world_ruleset_version_seal BEFORE INSERT ON world_ruleset_versions BEGIN
 SELECT RAISE(ABORT,'ruleset head mismatch') WHERE NOT EXISTS(SELECT 1 FROM world_rulesets WHERE world_id=NEW.world_id AND id=NEW.ruleset_id AND revision=NEW.revision);
 SELECT RAISE(ABORT,'ruleset snapshot mismatch') WHERE
 json_extract(NEW.after_json,'$.content.id') IS NOT NEW.ruleset_id OR json_extract(NEW.after_json,'$.content.world_id') IS NOT NEW.world_id OR json_extract(NEW.after_json,'$.content.revision') IS NOT NEW.revision OR
 json_extract(NEW.after_json,'$.content.fields.slug') IS NOT NEW.slug OR json_extract(NEW.after_json,'$.content.fields.display_name') IS NOT NEW.display_name OR json_extract(NEW.after_json,'$.content.fields.description') IS NOT NEW.description OR
 json_extract(NEW.after_json,'$.content.stat_roster.roster_id') IS NOT NEW.roster_id OR json_extract(NEW.after_json,'$.content.stat_roster.version') IS NOT NEW.roster_version OR
 json_extract(NEW.after_json,'$.content.chart.chart_id') IS NOT NEW.chart_id OR json_extract(NEW.after_json,'$.content.chart.version') IS NOT NEW.chart_version OR
 json_extract(NEW.after_json,'$.content.max_moves') IS NOT NEW.max_moves OR json_extract(NEW.after_json,'$.content.repeat_policy') IS NOT NEW.repeat_policy OR json_extract(NEW.after_json,'$.content.notes') IS NOT NEW.notes OR
 json_type(NEW.after_json,'$.content.expectations') IS NOT CASE WHEN NEW.expectations_defined=1 THEN 'array' ELSE 'null' END OR
 (SELECT count(*) FROM ruleset_attribute_expectations WHERE world_id=NEW.world_id AND ruleset_id=NEW.ruleset_id AND revision=NEW.revision) <> coalesce(json_array_length(NEW.after_json,'$.content.expectations'),0);
 SELECT RAISE(ABORT,'ruleset expectation mismatch') WHERE EXISTS(SELECT 1 FROM ruleset_attribute_expectations e JOIN move_attribute_versions a ON a.scope=e.world_id AND a.attribute_id=e.attribute_id AND a.revision=e.attribute_version
 WHERE e.world_id=NEW.world_id AND e.ruleset_id=NEW.ruleset_id AND e.revision=NEW.revision AND (
 json_extract(NEW.after_json,'$.content.expectations['||(e.slot-1)||'].definition') IS NOT json_extract(a.after_json,'$.content') OR
 json_extract(NEW.after_json,'$.content.expectations['||(e.slot-1)||'].required') IS NOT e.required OR
 json_extract(NEW.after_json,'$.content.expectations['||(e.slot-1)||'].allow_not_applicable') IS NOT e.allow_not_applicable OR
 json_extract(NEW.after_json,'$.content.expectations['||(e.slot-1)||'].constraints.whole_only') IS NOT e.whole_only));
 SELECT RAISE(ABORT,'ruleset bound mismatch') WHERE EXISTS(SELECT 1 FROM (
 SELECT *,ltrim(CAST(coefficient AS TEXT),'-') AS digits FROM (
 SELECT e.minimum_coefficient AS coefficient,e.minimum_scale AS scale,json_type(NEW.after_json,'$.content.expectations['||(e.slot-1)||'].constraints.minimum') AS kind,json_extract(NEW.after_json,'$.content.expectations['||(e.slot-1)||'].constraints.minimum') AS value FROM ruleset_attribute_expectations e WHERE e.world_id=NEW.world_id AND e.ruleset_id=NEW.ruleset_id AND e.revision=NEW.revision
 UNION ALL SELECT e.maximum_coefficient,e.maximum_scale,json_type(NEW.after_json,'$.content.expectations['||(e.slot-1)||'].constraints.maximum'),json_extract(NEW.after_json,'$.content.expectations['||(e.slot-1)||'].constraints.maximum') FROM ruleset_attribute_expectations e WHERE e.world_id=NEW.world_id AND e.ruleset_id=NEW.ruleset_id AND e.revision=NEW.revision))
 WHERE (coefficient IS NULL AND kind IS NOT 'null') OR (coefficient IS NOT NULL AND (kind IS NOT 'text' OR value IS NOT CASE WHEN scale=0 THEN CAST(coefficient AS TEXT) ELSE (CASE WHEN coefficient<0 THEN '-' ELSE '' END)||CASE WHEN length(digits)<=scale THEN '0.'||substr('000000',1,scale-length(digits))||digits ELSE substr(digits,1,length(digits)-scale)||'.'||substr(digits,length(digits)-scale+1) END END)));
 SELECT RAISE(ABORT,'ruleset predecessor mismatch') WHERE NEW.revision>1 AND NEW.before_json IS NOT (SELECT after_json FROM world_ruleset_versions WHERE world_id=NEW.world_id AND ruleset_id=NEW.ruleset_id AND revision=NEW.revision-1);
END;
