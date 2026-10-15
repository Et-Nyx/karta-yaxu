"""Independent chart workflows, through native keyboard-operated forms only.

Each test owns its server/database and uses cached fixtures. SQL only inspects
saved content; fresh-context navigation also probes URLs without permitted links.
The support mixin deliberately inherits no other workflow tests.
"""

import json
import unittest
from urllib.parse import parse_qs, urlsplit

from playwright.sync_api import expect

from test_authoring import AuthoringSupport


class ChartWorkflows(AuthoringSupport):
    seed_reference = True

    def setUp(self):
        super().setUp()
        self.assertEqual(self.sql("SELECT count(*) FROM charts"), [(0,)])
        self.admin("seed-fixtures")
        self.admin("invite-user", "--subject", "collaborator", "--role", "editor")
        self.assertEqual(self.sql("SELECT count(*) FROM charts"), [(3,)])
        self.sign_in("admin")

    def shared_id(self, slug):
        return self.sql("SELECT id FROM types WHERE slug = ?", (slug,))[0][0]

    def local_id(self, world, slug):
        return self.sql(
            "SELECT t.id FROM world_types t JOIN worlds w ON w.id = t.world_id "
            "WHERE w.slug = ? AND t.slug = ?", (world, slug),
        )[0][0]

    def open_chart(self, world=None):
        if world is None:
            self.follow("Charts")
            self.follow("Lab Effectiveness")
        else:
            self.follow("Worlds")
            self.follow(world)
            self.follow("View or create world chart")
        return self.page.url.removeprefix(self.origin), self.canonical_id("chart")

    def saved_chart(self, chart_id):
        rows = self.sql(
            "SELECT v.after_json FROM chart_versions v JOIN charts c "
            "ON c.id = v.chart_id AND c.revision = v.revision WHERE c.id = ?",
            (chart_id,),
        )
        self.assertEqual(len(rows), 1)
        return json.loads(rows[0][0])["content"]

    def write_counts(self):
        tables = ("charts", "chart_versions", "chart_roster", "chart_cells", "chart_mapping",
                  "audit_events", "world_revisions", "world_content_snapshots",
                  "form_content_snapshots", "variant_content_snapshots")
        return {table: self.sql(f"SELECT count(*) FROM {table}")[0][0] for table in tables}

    def typing_state(self):
        # Exact assignments and their immutable full snapshots must survive chart edits.
        tables = ("form_type_bindings", "variant_type_bindings",
                  "form_content_snapshots", "variant_content_snapshots")
        return {table: self.sql(f"SELECT * FROM {table} ORDER BY 1, 2") for table in tables}

    def comparison(self, heading):
        return self.page.locator("section.chart-comparison").filter(
            has=self.page.get_by_role("heading", name=heading, exact=True))

    def assert_pair(self, attacker, defender, text, panel=None):
        root = panel if panel is not None else self.page
        table = root.locator(".chart-table")
        pairs = root.locator(".chart-pairs")
        if self.viewport["width"] == 320:
            expect(table).to_be_hidden()
            expect(pairs).to_be_visible()
            row = pairs.locator(".chart-pair-row").filter(has=self.page.get_by_role(
                "heading", name=f"Attacker: {attacker}", exact=True))
            cell = row.get_by_text(f"Against {defender}", exact=True).locator(
                "xpath=following-sibling::dd[1]")
        else:
            expect(pairs).to_be_hidden()
            expect(table).to_be_visible()
            headings = table.locator("thead th").all_text_contents()
            self.assertIn(defender, headings)
            row = table.locator("tbody tr").filter(has=self.page.get_by_role(
                "rowheader", name=attacker, exact=True))
            cell = row.locator("td").nth(headings.index(defender) - 1)
        expect(cell).to_be_visible()
        expect(cell).to_have_text(text)
        self.assert_no_horizontal_overflow()

    def get_form(self, name, status=200):
        button = self.page.get_by_role("button", name=name, exact=True)
        expect(button.locator("xpath=ancestor::form")).to_have_attribute("method", "get")
        self.tab_to(button)
        with self.page.expect_navigation(wait_until="load") as navigation:
            self.page.keyboard.press("Enter")
        self.assertEqual(navigation.value.status, status)
        self.assertEqual(navigation.value.request.method, "GET")
        query = parse_qs(urlsplit(self.page.url).query)
        self.assertNotIn("csrf", query)
        self.assertNotIn("karta_session", query)
        self.assert_no_horizontal_overflow()

    def check(self, label):
        checkbox = self.page.get_by_label(label, exact=True)
        self.tab_to(checkbox)
        expect(checkbox).not_to_be_checked()
        self.page.keyboard.press("Space")
        expect(checkbox).to_be_checked()

    def confirm_chart(self):
        self.check("I reviewed the proposed matrix, exact pins, explicit mappings and selected source policy.")

    def confirm_neutral(self):
        self.check("I confirm filling all remaining Unknown cells with 1/1 Neutral.")

    def row_label(self, prefix, name, type_id):
        return f"{prefix} against {name} ({type_id})"

    def factor(self, name, type_id, numerator, denominator):
        self.enter(self.row_label("Numerator", name, type_id), numerator)
        self.enter(self.row_label("Denominator", name, type_id), denominator)

    def map_source(self, name, source_id, local_id):
        self.select(f"Map source {name} ({source_id}) to local Type", local_id)

    def chart_snapshot(self, revision):
        self.follow("Chart history")
        self.follow(f"Revision {revision}")
        self.tab_to(self.page.get_by_text("Full inert revision snapshot", exact=True))
        self.page.keyboard.press("Enter")
        snapshot = self.page.locator("details").filter(has=self.page.get_by_text(
            "Full inert revision snapshot", exact=True)).locator("pre")
        expect(snapshot).to_be_visible()
        result = json.loads(snapshot.inner_text())
        self.assertEqual(result["snapshot_version"], 1)
        return result["content"]

    def add_type(self, world, name, source=""):
        if self.page.url != self.origin + world:
            self.follow_path(world)
        self.follow("Add type")
        self.select("Source shared type and version", source)
        self.content(name, f"Local chart roster member: {name}.")
        self.submit("Add world type")
        return self.canonical_id("type")

    def test_fixture_states_row_validation_normalization_neutral_history_and_noop(self):
        pins = self.typing_state()
        self.open_chart()
        self.assert_pair("Fire", "Dragon", "1/2×")
        self.open_chart("ClassicLab")
        self.assert_pair("Fire", "Dragon", "1/2×")
        path, chart_id = self.open_chart("VariantLab")
        baseline = self.saved_chart(chart_id)
        self.assert_pair("Fire", "Dragon", "1× — Neutral")
        self.assert_pair("Dragon", "Fire", "2×")
        self.assert_pair("Flying", "Mist", "Unknown")
        self.assert_pair("Mist", "Fire", "0× — No effect")
        self.assert_pair("Mist", "Mist", "1× — Neutral")
        fire = self.local_id("variantlab", "fire")
        flying = self.local_id("variantlab", "flying")
        mist = self.local_id("variantlab", "mist")
        self.follow("Edit attacking row: Mist")
        self.factor("Fire", fire, "not-a-number", "0")
        self.factor("Flying", flying, "6", "4")
        reason = "Retain raw fractions, then explicitly fill Unknown"
        self.enter("Change reason (optional)", reason)
        counts = self.write_counts()
        self.submit("Save chart changes", status=422)
        expect(self.page.get_by_role("alert")).to_contain_text("Nothing was saved")
        for prefix, raw in (("Numerator", "not-a-number"), ("Denominator", "0")):
            field = self.page.get_by_label(self.row_label(prefix, "Fire", fire), exact=True)
            expect(field).to_have_value(raw)
            expect(field).to_have_attribute("aria-invalid", "true")
        expect(self.page.get_by_label(self.row_label("Numerator", "Flying", flying), exact=True)).to_have_value("6")
        expect(self.page.get_by_label(self.row_label("Denominator", "Flying", flying), exact=True)).to_have_value("4")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value(reason)
        self.assertEqual(self.write_counts(), counts)
        self.assertEqual(self.saved_chart(chart_id), baseline)
        self.factor("Fire", fire, "0", "5")
        self.factor("Mist", mist, "", "")
        self.submit("Save chart changes", destination=path)
        self.assert_pair("Mist", "Fire", "0× — No effect")
        self.assert_pair("Mist", "Flying", "3/2×")
        self.assert_pair("Mist", "Mist", "Unknown")
        edited = self.saved_chart(chart_id)
        self.assertEqual(edited["roster"], baseline["roster"])
        self.assertEqual(edited["source"], baseline["source"])
        self.assertEqual(edited["mapping"], baseline["mapping"])
        after = self.chart_snapshot(2)
        self.assertEqual(after, edited)
        expect(self.page.get_by_role("main")).to_contain_text("admin · edit")
        expect(self.page.get_by_role("main")).to_contain_text(reason)
        self.assert_pair("Mist", "Mist", "Unknown")
        self.follow("← Chart history")
        self.follow("Revision 1")
        self.assert_pair("Mist", "Mist", "1× — Neutral")
        self.follow("← Chart history")
        self.follow("← Current chart")
        self.follow("Fill remaining Unknown cells with neutral")
        counts = self.write_counts()
        self.submit("Fill remaining Unknown with neutral", status=422)
        expect(self.page.get_by_role("alert")).to_contain_text("Confirm filling")
        self.assertEqual(self.write_counts(), counts)
        self.confirm_neutral()
        self.submit("Fill remaining Unknown with neutral", destination=path)
        self.assert_pair("Mist", "Mist", "1× — Neutral")
        self.assert_pair("Flying", "Mist", "1× — Neutral")
        self.assert_pair("Mist", "Fire", "0× — No effect")
        self.assert_pair("Mist", "Flying", "3/2×")
        self.assertEqual(len(self.saved_chart(chart_id)["cells"]), 16)
        counts = self.write_counts()
        self.follow("Fill remaining Unknown cells with neutral")
        self.confirm_neutral()
        self.submit("Fill remaining Unknown with neutral", destination=path)
        self.follow("Edit attacking row: Mist")
        self.factor("Flying", flying, "6", "4")
        self.submit("Save chart changes", destination=path)
        self.assertEqual(self.write_counts(), counts)
        self.assertEqual(self.sql("SELECT revision FROM charts WHERE id = ?", (chart_id,)), [(3,)])
        self.assertEqual(self.saved_chart(chart_id)["roster"], baseline["roster"])
        self.assertEqual(self.typing_state(), pins)
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])

    def test_shared_and_local_native_creation_exact_roster_mapping_and_single_chart(self):
        fire, dragon = (self.shared_id(slug) for slug in ("fire", "dragon"))
        self.follow("Charts")
        self.follow("Create shared chart")
        self.content("Reviewed Source", "Keyboard-created synthetic source.", "Create exact shared roster")
        self.select("Type slot 1", f"{fire}@1")
        self.select("Type slot 2", f"{fire}@1")
        counts = self.write_counts()
        self.submit("Preview creation", status=422)
        expect(self.page.get_by_label("Type slot 2", exact=True)).to_have_attribute("aria-invalid", "true")
        expect(self.page.get_by_label("Type slot 2", exact=True)).to_have_value(f"{fire}@1")
        expect(self.page.get_by_label("Display name", exact=True)).to_have_value("Reviewed Source")
        self.assertEqual(self.write_counts(), counts)
        self.select("Type slot 2", f"{dragon}@1")
        self.submit("Preview creation", status=200)
        self.assert_pair("Fire", "Dragon", "Unknown", self.comparison("Proposed result"))
        self.assertEqual(self.write_counts(), counts)
        self.submit("Create reviewed chart", status=422)
        expect(self.page.get_by_role("alert")).to_contain_text("confirm creation")
        self.assertEqual(self.write_counts(), counts)
        self.submit("Preview creation", status=200)
        self.confirm_chart()
        self.submit("Create reviewed chart", destination="/charts/reviewed-source")
        source_id = self.canonical_id("chart")
        self.assertEqual([a["pin"] for a in self.saved_chart(source_id)["roster"]],
                         [{"type_id": fire, "version": 1}, {"type_id": dragon, "version": 1}])
        self.follow("Edit attacking row: Fire")
        self.factor("Dragon", dragon, "3", "1")
        self.submit("Save chart changes")

        world = self.create_world("Chart Workshop")
        world_id = self.canonical_id("world")
        first = self.add_type(world, "First Fire", f"{fire}@1")
        chosen = self.add_type(world, "Chosen Fire", f"{fire}@1")
        local_dragon = self.add_type(world, "Local Dragon", f"{dragon}@1")
        mist = self.add_type(world, "Mist")
        self.follow_path(world)
        self.follow("View or create world chart")
        # Reload the source before drafting. Similar names/source identities never
        # choose between the two compatible Fire types or populate roster slots.
        self.select("Source shared chart and exact version", f"{source_id}@2")
        self.get_form("Reload slots and source")
        expect(self.page.locator('form.chart-editor input[name="source_pin"]')).to_have_value(f"{source_id}@2")
        for index in range(1, 4):
            expect(self.page.get_by_label(f"Type slot {index}", exact=True)).to_have_value("")
        self.content("Workshop Chart", "Explicit ambiguous-source mapping.", "Choose the second Fire explicitly")
        for index, type_id in enumerate((mist, chosen, local_dragon), 1):
            self.select(f"Type slot {index}", f"{type_id}@1")
        for name, source in (("Fire", fire), ("Dragon", dragon)):
            expect(self.page.get_by_label(f"Map source {name} ({source}) to local Type", exact=True)).to_have_value("")
        self.map_source("Fire", fire, first)  # Compatible identity, absent from this roster.
        self.map_source("Dragon", dragon, local_dragon)
        self.select("Source cell policy", "apply")
        counts = self.write_counts()
        self.submit("Preview creation", status=422)
        expect(self.page.get_by_role("alert")).to_contain_text("Nothing was saved")
        expect(self.page.get_by_label(f"Map source Fire ({fire}) to local Type", exact=True)).to_have_value(first)
        expect(self.page.get_by_label("Source cell policy", exact=True)).to_have_value("apply")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Choose the second Fire explicitly")
        for index, type_id in enumerate((mist, chosen, local_dragon), 1):
            expect(self.page.get_by_label(f"Type slot {index}", exact=True)).to_have_value(f"{type_id}@1")
        self.assertEqual(self.write_counts(), counts)
        self.map_source("Fire", fire, chosen)
        self.submit("Preview creation", status=200)
        self.assert_pair("Chosen Fire", "Local Dragon", "3×", self.comparison("Proposed result"))
        self.assert_pair("Mist", "Chosen Fire", "Unknown", self.comparison("Proposed result"))
        self.assertEqual(self.write_counts(), counts)
        self.confirm_chart()
        path = world + "/charts/workshop-chart"
        self.submit("Create reviewed chart", destination=path)
        chart_id = self.canonical_id("chart")
        saved = self.saved_chart(chart_id)
        self.assertEqual([a["pin"] for a in saved["roster"]],
                         [{"type_id": value, "version": 1} for value in (mist, chosen, local_dragon)])
        self.assertEqual(saved["source"], {"chart_id": source_id, "version": 2})
        self.assertEqual({m["source_type_id"]: m["local_type_id"] for m in saved["mapping"]},
                         {fire: chosen, dragon: local_dragon})
        self.follow("← World")
        self.follow("View or create world chart")
        expect(self.page).to_have_url(self.origin + path)
        self.assertEqual(self.sql("SELECT id FROM charts WHERE world_id = ?", (world_id,)), [(chart_id,)])

        # A fresh-context duplicate-creation probe must still reject a genuine form.
        self.new_context(world + "/charts/new", authenticated_context=self.context)
        self.assertEqual(self.initial_status, 200)
        self.content("Second Chart", "A second local chart must be rejected.")
        counts = self.write_counts()
        self.submit("Preview creation", status=422)
        expect(self.page.get_by_role("alert")).to_contain_text("already has a chart")
        self.assertEqual(self.write_counts(), counts)
        self.assertEqual(self.saved_chart(chart_id), saved)
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])

    def test_source_update_explicit_get_preview_and_adoption_preserve_unmapped_and_typing(self):
        path, local_id = self.open_chart("VariantLab")
        baseline = self.saved_chart(local_id)
        pins = self.typing_state()
        _, classic_id = self.open_chart("ClassicLab")
        classic = self.saved_chart(classic_id)
        self.open_chart()
        source_id = self.canonical_id("chart")
        fire, flying, dragon = (self.shared_id(slug) for slug in ("fire", "flying", "dragon"))
        self.follow("Edit attacking row: Fire")
        self.factor("Fire", fire, "3", "2")
        self.factor("Flying", flying, "0", "1")
        self.factor("Dragon", dragon, "", "")
        self.enter("Change reason (optional)", "Publish Unknown and changed source factors")
        self.submit("Save chart changes")
        self.assertEqual(self.saved_chart(local_id), baseline)
        self.open_chart("VariantLab")
        self.assert_pair("Fire", "Dragon", "1× — Neutral")
        self.assert_pair("Fire", "Flying", "1× — Neutral")
        expect(self.saved_value("Source")).to_contain_text("version 1")
        self.get_form("Choose mappings and policy")
        expect(self.page.get_by_label("New shared chart version", exact=True)).to_have_value("2")
        expect(self.page.get_by_label("Source cell policy", exact=True)).to_have_value("keep")
        counts = self.write_counts()
        self.get_form("Preview selected mappings and policy")
        self.assert_pair("Fire", "Dragon", "1/2×", self.comparison("Old shared source"))
        self.assert_pair("Fire", "Dragon", "Unknown", self.comparison("New shared source"))
        self.assert_pair("Fire", "Dragon", "1× — Neutral", self.comparison("Proposed result"))
        self.assertEqual(self.write_counts(), counts)
        self.assertEqual(self.saved_chart(local_id), baseline)
        self.select("Source cell policy", "apply")
        self.map_source("Flying", flying, "")
        reason = "Apply mapped pairs, retain Mist and unmapped Flying"
        self.enter("Change reason (optional)", reason)
        self.get_form("Preview selected mappings and policy")
        current = self.comparison("Current local effective chart")
        proposed = self.comparison("Proposed result")
        self.assert_pair("Fire", "Dragon", "1× — Neutral", current)
        self.assert_pair("Fire", "Dragon", "Unknown", proposed)
        self.assert_pair("Fire", "Fire", "3/2×", proposed)
        self.assert_pair("Fire", "Flying", "1× — Neutral", proposed)
        self.assert_pair("Mist", "Fire", "0× — No effect", proposed)
        self.assert_pair("Mist", "Dragon", "Unknown", proposed)
        self.assert_pair("Fire", "Mist", "2×", proposed)
        self.assertEqual(self.write_counts(), counts)
        self.assertEqual(self.saved_chart(local_id), baseline)
        self.submit("Adopt reviewed source version", status=422)
        expect(self.page.get_by_role("alert")).to_contain_text("Preview and confirm")
        expect(self.page.get_by_label("Source cell policy", exact=True)).to_have_value("apply")
        expect(self.page.get_by_label(f"Map source Flying ({flying}) to local Type", exact=True)).to_have_value("")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value(reason)
        self.assertEqual(self.write_counts(), counts)
        self.get_form("Preview selected mappings and policy")
        self.confirm_chart()
        self.submit("Adopt reviewed source version", destination=path)
        self.assert_pair("Fire", "Dragon", "Unknown")
        self.assert_pair("Fire", "Flying", "1× — Neutral")
        self.assert_pair("Mist", "Fire", "0× — No effect")
        saved = self.saved_chart(local_id)
        self.assertEqual(saved["source"], {"chart_id": source_id, "version": 2})
        self.assertEqual(saved["roster"], baseline["roster"])
        unmapped = {self.local_id("variantlab", slug) for slug in ("mist", "flying")}
        untouched = lambda content: [c for c in content["cells"] if
                                    c["attacker_type_id"] in unmapped or c["defender_type_id"] in unmapped]
        self.assertEqual(untouched(saved), untouched(baseline))
        self.assertEqual(self.saved_chart(classic_id), classic)
        self.assertEqual(self.typing_state(), pins)
        self.assertEqual(self.chart_snapshot(2), saved)
        expect(self.page.get_by_role("main")).to_contain_text("admin · adopt")
        expect(self.page.get_by_role("main")).to_contain_text(reason)
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])

    def test_repeated_row_conflicts_explicit_review_revocation_and_scope_permissions(self):
        path, chart_id = self.open_chart("VariantLab")
        fire = self.local_id("variantlab", "fire")
        first_page = self.page
        self.follow("Edit attacking row: Mist")
        self.factor("Fire", fire, "4", "1")
        reason = "Explicitly reviewed winning row"
        self.enter("Change reason (optional)", reason)
        self.new_context()
        self.sign_in("admin")
        second_page = self.page
        self.open_chart("VariantLab")
        self.follow("Edit attacking row: Mist")
        self.factor("Fire", fire, "2", "1")
        self.submit("Save chart changes", destination=path)
        for expected_revision, saved_factor in ((2, "2×"), (3, "3×")):
            self.page = first_page
            counts = self.write_counts()
            saved = self.saved_chart(chart_id)
            self.submit("Save chart changes" if expected_revision == 2 else "Save reviewed changes", status=409)
            expect(self.page.get_by_role("alert")).to_contain_text("Nothing was saved")
            current = self.comparison("Current saved state")
            expect(current).to_contain_text(f"revision {expected_revision}")
            self.assert_pair("Mist", "Fire", saved_factor, current)
            expect(self.page.get_by_label(self.row_label("Numerator", "Fire", fire), exact=True)).to_have_value("4")
            expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value(reason)
            expect(self.page.locator('input[name="expected_revision"]')).to_have_value(str(expected_revision - 1))
            button = self.page.get_by_role("button", name="Save reviewed changes", exact=True)
            expect(button).to_have_attribute("name", "reviewed_revision")
            expect(button).to_have_attribute("value", str(expected_revision))
            self.assertEqual(self.write_counts(), counts)
            self.assertEqual(self.saved_chart(chart_id), saved)
            if expected_revision == 2:
                self.page = second_page
                self.follow("Edit attacking row: Mist")
                self.factor("Fire", fire, "3", "1")
                self.submit("Save chart changes", destination=path)
        self.submit("Save reviewed changes", destination=path)
        self.assert_pair("Mist", "Fire", "4×")
        self.assertEqual(self.sql("SELECT revision FROM charts WHERE id = ?", (chart_id,)), [(4,)])
        self.assertEqual(self.chart_snapshot(4), self.saved_chart(chart_id))
        expect(self.page.get_by_role("main")).to_contain_text(reason)

        owner_page = self.page
        collaborator_context, collaborator_page = self.new_context()
        self.sign_in("collaborator")
        self.open_chart("VariantLab")
        expect(self.link("Edit chart definition")).to_have_count(0)
        self.probe_forbidden(path + "/edit", collaborator_context)
        self.probe_forbidden("/charts/lab-effectiveness/edit", collaborator_context)
        self.page = owner_page
        self.follow("Worlds")
        self.follow("VariantLab")
        self.follow("Manage collaborators")
        self.enter("Target user ID", self.user_id("collaborator"))
        self.submit("Grant collaboration", destination="/worlds/variantlab/access")
        self.page = collaborator_page
        self.open_chart("VariantLab")
        self.follow("Edit attacking row: Mist")
        self.factor("Fire", fire, "5", "1")
        self.enter("Change reason (optional)", "Revoked row draft stays visible")
        self.page = owner_page
        self.submit("Revoke collaboration for collaborator", destination="/worlds/variantlab/access")
        counts = self.write_counts()
        saved = self.saved_chart(chart_id)
        self.page = collaborator_page
        self.submit("Save chart changes", status=403)
        expect(self.page.get_by_label(self.row_label("Numerator", "Fire", fire), exact=True)).to_have_value("5")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Revoked row draft stays visible")
        self.assertEqual(self.write_counts(), counts)
        self.assertEqual(self.saved_chart(chart_id), saved)
        self.follow("← Return to chart")
        expect(self.link("Edit attacking row: Mist")).to_have_count(0)

        # An administrator has library rights, not ownership of another editor's world.
        self.new_context()
        self.sign_in("alice")
        world = self.create_world("Alice Chart Scope")
        self.follow("View or create world chart")
        self.content("Alice Chart", "An empty independent chart is a valid draft.")
        self.submit("Preview creation", status=200)
        self.confirm_chart()
        self.submit("Create reviewed chart", destination=world + "/charts/alice-chart")
        admin_context = first_page.context
        self.probe_forbidden(world + "/charts/alice-chart/edit", admin_context)
        counts = self.write_counts()
        self.new_context(world + f"/charts/{chart_id}", authenticated_context=admin_context)
        self.assertEqual(self.initial_status, 404)
        self.assert_no_horizontal_overflow()
        self.assertEqual(self.write_counts(), counts)
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])


class DesktopChartTests(ChartWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 800}


class NarrowChartTests(ChartWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
