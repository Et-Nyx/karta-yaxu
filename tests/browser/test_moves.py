"""Native move/attribute/ruleset workflows, keyboard only at desktop and 320px.

AuthoringSupport owns temporary databases, Chromium contexts and server cleanup.
Only the old creature seed and native forms provision content; SQL is read-only.
"""
import unittest

from playwright.sync_api import expect
from test_authoring import AuthoringSupport


class MoveWorkflows(AuthoringSupport):
    def setUp(self):
        super().setUp()
        self.admin("invite-user", "--subject", "librarian", "--role", "administrator")
        self.sign_in("librarian")

    def open(self, path):
        self.new_context(path, authenticated_context=self.page.context)
        self.assertEqual(self.initial_status, 200)
        self.assert_no_horizontal_overflow()

    def check(self, label):
        control = self.page.get_by_label(label, exact=True)
        self.tab_to(control)
        self.page.keyboard.press("Space")
        expect(control).to_be_checked()

    def confirm(self):
        self.check("I reviewed the exact pins, proposed values, removals and version changes.")
        self.submit("Confirm changes")

    def save(self):
        self.submit("Preview changes", status=200)
        self.confirm()

    def attribute(self, name, world=None, source=""):
        self.open((world or "") + "/move-attributes")
        self.follow("Create move attribute")
        self.content(name, f"Synthetic numeric metadata for {name}.")
        self.enter("Descriptive unit", "units")
        if world:
            self.source_pin(source)
        self.save()
        path = self.page.url.removeprefix(self.origin)
        identity = self.sql("SELECT id FROM move_attribute_definitions WHERE slug = ?", (name.lower().replace(" ", "-"),))[0][0]
        return path, identity

    def movement(self, name, world=None, source="", attribute="", state="unknown", value=""):
        self.open((world or "") + "/moves")
        self.follow("Create move")
        self.content(name, f"Synthetic move {name}.")
        if world:
            self.source_pin(source)
        if attribute:
            self.select("Attribute collection state", "defined")
            self.select("Collection operation", "add")
            self.select("Attribute and exact version", attribute)
            self.select("Numeric value state", state)
            if value:
                self.enter("Known numeric value", value)
        self.save()
        path = self.page.url.removeprefix(self.origin)
        identity = self.sql("SELECT id FROM moves WHERE slug = ?", (name.lower().replace(" ", "-"),))[0][0]
        return path, identity

    def source_pin(self, pin):
        choices = self.page.get_by_label("Shared source and exact version", exact=True)
        values = choices.locator("option").evaluate_all("options => options.map(option => option.value)")
        if pin in values:
            self.select("Shared source and exact version", pin)
        else:
            self.enter("Other exact shared source pin", pin)

    def test_attribute_raw_decimals_exact_source_adoption_and_readable_history(self):
        path, shared = self.attribute("Shared Balance")
        self.follow("Edit")
        self.enter("Minimum (inclusive)", "-1.2345678")
        self.enter("Maximum (inclusive)", "1e2")
        self.enter("Change reason (optional)", "Keep raw malformed numeric constraints")
        self.submit("Preview changes", status=422)
        expect(self.page.get_by_label("Minimum (inclusive)", exact=True)).to_have_value("-1.2345678")
        expect(self.page.get_by_label("Maximum (inclusive)", exact=True)).to_have_value("1e2")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Keep raw malformed numeric constraints")
        self.enter("Minimum (inclusive)", "-2.500000")
        self.enter("Maximum (inclusive)", "5")
        self.save()
        self.follow("History")
        self.follow("Revision 1")
        expect(self.page.get_by_role("main")).to_contain_text("unbounded")
        world = self.create_world("Attribute Workshop")
        local_path, local = self.attribute("Local Balance", world, f"{shared}@1")
        self.follow("Review source adoption")
        expect(self.page.get_by_label("Shared source version", exact=True)).to_have_value("2")
        self.submit("Preview changes", status=200)
        for heading in ("Old shared source", "Current local definition", "New shared source", "Proposed state"):
            expect(self.page.get_by_role("heading", name=heading, exact=True)).to_be_visible()
        self.confirm()
        expect(self.page).to_have_url(self.origin + local_path)
        expect(self.page.get_by_role("main")).to_contain_text(f"{shared}@2")
        expect(self.page.get_by_role("main")).to_contain_text("unbounded")
        self.assertEqual(self.sql("SELECT revision FROM move_attribute_definitions WHERE id=?", (local,)), [(2,)])
        self.open(path)
        expect(self.page.get_by_role("main")).to_contain_text("Minimum (inclusive): -2.5")

    def test_move_states_explicit_mapping_whole_replacement_and_changed_preview(self):
        _, shared = self.attribute("Shared Numeric")
        _, source = self.movement("Shared Technique", attribute=f"{shared}@1", state="not_applicable")
        world = self.create_world("Mapping Workshop")
        _, local = self.attribute("Local Numeric", world, f"{shared}@1")
        _, unrelated = self.attribute("Independent Numeric", world)
        local_path, target = self.movement("Local Technique", world, f"{source}@1", f"{unrelated}@1", "known", "0")
        expect(self.page.get_by_role("main")).to_contain_text("Known 0")
        self.follow("Review source adoption")
        # Default keep-local at the pinned source version does not replace zero.
        self.enter("Shared source version", "1")
        self.submit("Preview changes", status=200)
        proposed = self.page.locator("section").filter(has=self.page.get_by_role("heading", name="Proposed state", exact=True))
        expect(proposed).to_contain_text("Known 0")
        self.confirm()
        self.follow("Review source adoption")
        self.select("Mapping operation", "set")
        self.enter("Shared attribute identity", shared)
        self.select("Local attribute and exact version", f"{local}@1")
        self.check("Replace the whole attribute collection from source")
        self.submit("Preview changes", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("All removed local assignments")
        expect(proposed).to_contain_text("NotApplicable")
        self.enter("Change reason (optional)", "Changed after preview")
        self.confirm_changed()
        self.confirm()
        expect(self.page).to_have_url(self.origin + local_path)
        expect(self.page.get_by_role("main")).to_contain_text("Local Numeric = NotApplicable")
        self.follow("Edit")
        self.select("Collection operation", "value")
        self.enter("Existing entry position", "1")
        self.select("Numeric value state", "unknown")
        self.save()
        expect(self.page.get_by_role("main")).to_contain_text("Local Numeric = Unknown")
        self.follow("History")
        self.follow("Revision 2")
        expect(self.page.get_by_role("main")).to_contain_text("NotApplicable")
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])

    def confirm_changed(self):
        self.check("I reviewed the exact pins, proposed values, removals and version changes.")
        self.submit("Confirm changes", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("Review this fresh preview")
        expect(self.page.get_by_label("I reviewed the exact pins, proposed values, removals and version changes.", exact=True)).not_to_be_checked()

    def test_ruleset_unconfigured_optional_policy_exact_validation_and_removal(self):
        world = self.create_world("Rules Workshop")
        _, attribute = self.attribute("Variable Numeric", world)
        move_path, move = self.movement("Unknown Technique", world, attribute=f"{attribute}@1")
        expect(self.page.get_by_role("main")).to_contain_text("World ruleset is unconfigured")
        self.follow("World ruleset")
        expect(self.page.get_by_role("main")).to_contain_text("Unconfigured")
        self.follow("Configure ruleset")
        self.content("Explicit Rules", "Independent authoring configuration.")
        self.select("Expectation collection state", "defined")
        self.select("Collection operation", "add")
        self.select("Attribute and exact version", f"{attribute}@1")
        self.check("Required Known value")
        self.enter("Minimum (inclusive)", "-1.25")
        self.save()
        expect(self.page.get_by_role("main")).to_contain_text("Unspecified — no default")
        self.follow("Edit")
        self.select("Text field to load", "notes")
        self.enter("Description", "Unsaved signed Unicode −雨 draft")
        self.enter("Change reason (optional)", "Retain this signed − prose reason")
        self.submit("Load selected text field", status=422)
        expect(self.page.get_by_label("Description", exact=True)).to_have_value("Unsaved signed Unicode −雨 draft")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Retain this signed − prose reason")
        self.submit("Discard this page draft and load latest", status=200)
        self.select("Text field to load", "notes")
        self.submit("Load selected text field", status=200)
        expect(self.page.locator('textarea[name="description"], textarea[name="notes"], input[name="description"], input[name="notes"]')).to_have_count(0)
        expect(self.page.locator('textarea[name="text"]')).to_have_count(1)
        self.enter("Inert interaction and environment notes", "Rain and formula prose remains inert. Signed −雨 notes.")
        self.save()
        self.open(move_path)
        expect(self.page.get_by_role("main")).to_contain_text("Authoring validation — NOT combat readiness")
        expect(self.page.get_by_role("main")).to_contain_text("required value is Unknown")
        expect(self.page.get_by_role("main")).to_contain_text(f"move {move}@1")
        self.follow("World ruleset")
        self.follow("Edit")
        self.select("Collection operation", "remove")
        self.enter("Existing entry position", "1")
        self.enter("Maximum moves (optional)", "7")
        self.select("Repeat policy", "forbid")
        self.submit("Preview changes", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("All removed entries")
        self.confirm()
        expect(self.page.get_by_role("main")).to_contain_text("Defined-empty")
        self.follow("History")
        self.follow("Revision 2")
        expect(self.page.get_by_role("main")).to_contain_text("Required Known")
        expect(self.page.get_by_role("main")).to_contain_text("Rain and formula prose remains inert")
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])


class DesktopMoveTests(MoveWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 800}


class NarrowMoveTests(MoveWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
