"""Named Card Workshop authoring, entirely through native keyboard controls.

Each test provisions its own original definitions through forms, so it also runs
before the optional operator workshop seed is installed. No opaque coordinates
are copied into authoring forms. SQL only checks the test-owned saved state.
"""
import hashlib
import json
from pathlib import Path
import unittest

from playwright.sync_api import expect
from test_authoring import AuthoringSupport
from test_cards import CardWorkflows


class WorkshopWorkflows(AuthoringSupport):
    open = CardWorkflows.open
    send = CardWorkflows.send

    def setUp(self):
        super().setUp()
        self.admin("invite-user", "--subject", "workshop-designer", "--role", "administrator")
        self.sign_in("workshop-designer")

    def make_card(self, name, world="", source=""):
        self.open(world + "/cards/new")
        self.enter("Display name", name)
        self.enter("Description (inert plain text)", "Original inert workshop example")
        if source:
            self.enter("Picker name contains", source)
            self.send("Search picker", status=200)
            self.send("Choose " + source, status=200)
        self.send()
        return self.page.url.removeprefix(self.origin)

    def make_field(self, name, kind, world="", source=""):
        self.open(world + "/card-fields/new")
        self.enter("Display name", name)
        self.enter("Description (inert plain text)", "Original reusable field")
        self.select("Field kind", kind)
        if source:
            self.enter("Picker name contains", source)
            self.send("Search picker", status=200)
            self.send("Choose " + source, status=200)
        self.send()
        return self.page.url.removeprefix(self.origin)

    def choose_field(self, card, name):
        self.open(card)
        self.follow("Edit assignment")
        self.enter("Picker name contains", name)
        self.send("Search picker", status=200)
        self.send("Choose " + name, status=200)

    def test_named_number_invalid_draft_search_boolean_false_and_exact_history(self):
        self.make_field("Roll Bonus", "number")
        self.make_field("Optional Marker", "boolean")
        card = self.make_card("Trail Kit")
        self.choose_field(card, "Roll Bonus")
        self.select("Field value state", "known")
        self.enter("Loaded field value (number, text or stable choice ID)", "not a number −雨")
        self.enter("Change reason (optional)", "Preserve this unsaved reason")
        self.enter("Picker name contains", "Optional")
        self.send("Search picker", status=200)
        expect(self.page.get_by_label("Loaded field value (number, text or stable choice ID)", exact=True)).to_have_value("not a number −雨")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Preserve this unsaved reason")
        expect(self.page.get_by_role("main")).to_contain_text("Selected field: Roll Bonus")
        self.send(status=422)
        self.enter("Loaded field value (number, text or stable choice ID)", "0")
        self.send()
        self.choose_field(card, "Optional Marker")
        self.select("Field value state", "known")
        self.select("Known boolean", "false")
        self.send()
        expect(self.page.get_by_role("main")).to_contain_text("Number 0")
        expect(self.page.get_by_role("main")).to_contain_text("Boolean false")
        self.follow("History")
        self.follow("Revision 2")
        expect(self.page.get_by_role("main")).to_contain_text("Number 0")
        expect(self.page.get_by_role("main")).not_to_contain_text("Boolean false")

    def test_dirty_text_search_switch_and_explicit_discard_load(self):
        self.make_field("Usage Notes", "text")
        self.make_field("Other Notes", "text")
        card = self.make_card("Field Wrap")
        for name, text in (("Usage Notes", "Saved first paragraph"), ("Other Notes", "Saved second paragraph")):
            self.choose_field(card, name)
            self.select("Field value state", "known")
            self.enter("Loaded field value (number, text or stable choice ID)", text)
            self.send()
        self.follow("Load assignment Usage Notes")
        prose = "🐉" * 1990 + "\nDraft"
        self.enter("Loaded field value (number, text or stable choice ID)", prose)
        self.enter("Picker name contains", "Other Notes")
        self.send("Search picker", status=200)
        self.send("Choose Other Notes", status=200)
        expect(self.page.get_by_label("Loaded field value (number, text or stable choice ID)", exact=True)).to_have_value(prose)
        self.send("Load selected assignment", status=422)
        expect(self.page.get_by_label("Loaded field value (number, text or stable choice ID)", exact=True)).to_have_value(prose)
        self.assertEqual(self.page.locator('textarea[name="text"]').count(), 1)
        self.assertFalse(self.page.locator('input[type="hidden"]').evaluate_all(
            "(elements, prose) => elements.some(element => element.value === prose)", prose))
        self.send("Discard text and load selected assignment", status=200)
        expect(self.page.get_by_label("Loaded field value (number, text or stable choice ID)", exact=True)).to_have_value("Saved second paragraph")
        self.enter("Loaded field value (number, text or stable choice ID)", "")
        self.send()
        expect(self.page.get_by_role("main")).to_contain_text("Text (empty)")

    def test_named_choices_and_references_keep_exact_historical_names(self):
        field = self.make_field("Use Context", "choice")
        self.follow("Edit option")
        self.enter("Option label", "During preparation")
        self.send()
        self.make_field("Associated Card", "card_reference")
        target = self.make_card("Focused Effort")
        card = self.make_card("Steady Ascent")
        self.choose_field(card, "Use Context")
        self.select("Field value state", "known")
        self.select("Find by name", "choice")
        self.enter("Picker name contains", "preparation")
        self.send("Search picker", status=200)
        self.send("Choose During preparation", status=200)
        self.send()
        self.choose_field(card, "Associated Card")
        self.select("Field value state", "known")
        self.select("Find by name", "reference")
        self.enter("Picker name contains", "Focused Effort")
        self.send("Search picker", status=200)
        self.send("Choose Focused Effort", status=200)
        self.send()
        self.open(target)
        self.follow("Edit name")
        self.enter("Display name", "Renamed current target")
        self.send()
        self.open(field)
        self.follow("Load option During preparation")
        self.enter("Option label", "Changed current choice")
        self.send()
        self.open(card)
        expect(self.page.get_by_role("main")).to_contain_text("During preparation")
        expect(self.page.get_by_role("main")).to_contain_text("Focused Effort")
        expect(self.page.get_by_role("main")).not_to_contain_text("Renamed current target")
        expect(self.page.get_by_role("main")).not_to_contain_text("Changed current choice")
        self.follow("Referenced Focused Effort · exact revision 1")
        expect(self.page.get_by_role("heading", name="Focused Effort", exact=True)).to_be_visible()

    def test_named_local_source_mapping_and_confirmed_whole_replacement(self):
        shared = self.make_field("Shared Roll Bonus", "number")
        source = self.make_card("Shared Trail Kit")
        self.choose_field(source, "Shared Roll Bonus")
        self.select("Field value state", "known")
        self.enter("Loaded field value (number, text or stable choice ID)", "3")
        self.send()
        world = self.create_world("Named Workshop")
        self.make_field("Local Roll Bonus", "number", world, "Shared Roll Bonus")
        local = self.make_card("Local Trail Kit", world, "Shared Trail Kit")
        self.follow("Edit copy")
        self.send("Choose Shared Roll Bonus", status=200)
        self.send("Choose Local Roll Bonus", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("Shared Roll Bonus → Local Roll Bonus")
        self.enter("Picker name contains", "Nothing matches")
        self.send("Search picker", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("Shared Roll Bonus → Local Roll Bonus")
        self.send("Preview changes", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("Number 3")
        expect(self.page.get_by_role("main")).to_contain_text("Removed local assignments (0 total")
        self.send("Confirm reviewed replacement or adoption")
        expect(self.page).to_have_url(self.origin + local)
        expect(self.page.get_by_role("main")).to_contain_text("Local Roll Bonus")
        self.open(shared)
        self.follow("Edit name")
        self.enter("Display name", "Shared Roll Bonus revised")
        self.send()
        self.open(world + "/card-fields")
        self.enter("Name contains (literal text)", "Local Roll Bonus")
        button = self.page.get_by_role("button", name="Search catalog", exact=True)
        self.tab_to(button)
        with self.page.expect_navigation(wait_until="load"):
            self.page.keyboard.press("Enter")
        self.follow("View field")
        self.follow("Edit adopt")
        # Named version buttons include the exact revision in nearby context;
        # the new current name makes this choice unambiguous without copying IDs.
        self.send("Choose Shared Roll Bonus revised", status=200)
        self.send("Preview changes", status=200)
        self.send("Confirm reviewed replacement or adoption")
        expect(self.page.get_by_role("heading", name="Local Roll Bonus", exact=True)).to_be_visible()
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])

    def test_explicit_seed_named_edit_rerun_inspection_and_frozen_download(self):
        owner = self.user_id("workshop-designer")
        self.admin("seed-card-workshop", "--owner-user-id", owner)
        self.open("/worlds/card-workshop/cards")
        self.enter("Name contains (literal text)", "Trail Kit")
        button = self.page.get_by_role("button", name="Search catalog", exact=True)
        self.tab_to(button)
        with self.page.expect_navigation(wait_until="load"):
            self.page.keyboard.press("Enter")
        self.follow("View card")
        self.follow("Load assignment Roll Bonus")
        self.select("Field value state", "known")
        self.enter("Loaded field value (number, text or stable choice ID)", "4")
        self.send()
        expect(self.page.get_by_role("main")).to_contain_text("Number 4")
        self.admin("seed-card-workshop", "--owner-user-id", owner)
        self.follow("History")
        self.follow("Revision 2")
        expect(self.page.get_by_role("main")).to_contain_text("Number 4")
        self.open("/worlds/card-workshop/inspection/cards")
        expect(self.page.get_by_role("main")).to_contain_text("Coverage: Complete")
        expect(self.page.get_by_role("main")).to_contain_text("Known number: 4")
        self.open("/worlds/card-workshop/export")
        self.send("Preview package", status=200)
        digest = self.page.locator('input[name="reviewed_digest"]').input_value()
        checkbox = self.page.get_by_role("checkbox", name="I reviewed this package and its draft diagnostics.", exact=True)
        self.tab_to(checkbox)
        self.page.keyboard.press("Space")
        button = self.page.get_by_role("button", name="Confirm and download", exact=True)
        self.tab_to(button)
        with self.page.expect_download() as pending:
            self.page.keyboard.press("Enter")
        downloaded = pending.value
        self.assertIsNone(downloaded.failure())
        artifact = Path(self.work.name) / "workshop.json"
        downloaded.save_as(artifact)
        package = json.loads(artifact.read_bytes())
        self.assertEqual(package["manifest"]["schema_version"], 2)
        canonical = json.dumps(package["content"], ensure_ascii=False, separators=(",", ":")).encode()
        self.assertEqual(hashlib.sha256(b"karta-design-package-content-sha256-v2\n" + canonical).hexdigest(), digest)
        local_kit = next(c for c in package["content"]["cards"]
                         if c["identity"]["world_id"] and c["fields"]["display_name"] == "Trail Kit" and c["revision"] == 2)
        self.assertEqual(local_kit["revision"], 2)
        self.assertIn({"identity": local_kit["identity"], "version": 2}, package["content"]["card_roots"])
        bonus = next(f for f in local_kit["component"]["fields"] if f["definition"]["fields"]["display_name"] == "Roll Bonus")
        self.assertEqual(bonus["value"], {"state": "number", "value": "4"})
        self.assert_no_horizontal_overflow()


class DesktopCardWorkshopTests(WorkshopWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 800}


class NarrowCardWorkshopTests(WorkshopWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
