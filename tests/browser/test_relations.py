"""Original records: native keyboard authoring and frozen exact relationships."""
import hashlib
import json
from pathlib import Path
import re
import unittest
from playwright.sync_api import expect
from test_authoring import AuthoringSupport
from test_cards import CardWorkflows


class RecordWorkflows(AuthoringSupport):
    browser_timeout = 30000
    open = CardWorkflows.open
    send = CardWorkflows.send

    def setUp(self):
        super().setUp()
        self.admin("invite-user", "--subject", "record-designer", "--role", "administrator")
        self.sign_in("record-designer")
        self.owner = self.user_id("record-designer")
        self.admin("seed-card-relations", "--owner-user-id", self.owner)

    def card_records(self, slug):
        return f"/worlds/card-relations-workshop/cards/standalone/{slug}/records"

    def search(self, kind, query=""):
        self.select("Find by name", kind)
        self.enter("Picker name contains", query)
        self.send("Search picker", status=200)

    def choose(self, name):
        button = self.page.get_by_role("button", name=re.compile(r"^Choose " + re.escape(name) + r"(?: ·|$)"))
        self.send(button.inner_text(), status=200)

    def test_named_amount_edit_invalid_search_exact_history_and_edited_rerun(self):
        path = self.card_records("second-wind")
        self.open(path)
        self.follow("Edit cell")
        self.search("column", "Amount")
        self.choose("Amount")
        self.send("Load selected cell", status=200)
        self.select("Cell state", "known")
        self.enter("Loaded numeric value (zero is known)", "invalid 🐉")
        self.search("card", "Focus")
        expect(self.page.get_by_label("Loaded numeric value (zero is known)", exact=True)).to_have_value("invalid 🐉")
        self.send(status=422)
        self.enter("Loaded numeric value (zero is known)", "0")
        self.enter("Change reason (optional)", "Retain stable resource row")
        self.send()
        expect(self.page.get_by_role("main")).to_contain_text("Number 0")
        self.admin("seed-card-relations", "--owner-user-id", self.owner)
        self.follow("History")
        expect(self.page.get_by_role("main")).to_contain_text("Retain stable resource row")
        self.follow("Revision 1")
        expect(self.page.get_by_role("main")).to_contain_text("Amount: Number 2")

    def test_dirty_unicode_condition_switch_requires_discard_and_preserves_known_empty(self):
        self.open(self.card_records("second-wind"))
        self.follow("Edit cell")
        self.search("column", "Condition")
        self.choose("Condition")
        self.send("Load selected cell", status=200)
        self.select("Cell state", "known")
        prose = "🐉" * 1995 + "\nline"
        self.enter("Loaded cell text (empty is known, inert)", prose)
        self.send()
        self.follow("Edit cell")
        self.search("column", "Condition")
        self.choose("Condition")
        self.send("Load selected cell", status=200)
        self.enter("Loaded cell text (empty is known, inert)", "")
        self.search("entry", "Entry 2 ")
        self.choose(self.page.get_by_role("button", name=re.compile(r"^Choose Entry 2 ·")).inner_text().removeprefix("Choose "))
        self.send("Load selected cell", status=422)
        self.send("Discard text and load selected cell", status=200)
        self.select("Cell state", "known")
        self.enter("Loaded cell text (empty is known, inert)", "")
        self.assertEqual(self.page.locator('textarea[name="text"]').count(), 1)
        self.send()
        expect(self.page.get_by_role("main")).to_contain_text("Text (empty)")

    def test_named_stat_and_field_targets_preserve_old_revisions_and_mixed_usage(self):
        self.open(self.card_records("patient"))
        self.follow("Edit cell")
        self.select("Cell state", "known")
        self.search("stat", "Resolve")
        self.choose("Resolve")
        self.send()
        expect(self.page.get_by_role("main")).to_contain_text("Known reference: Resolve")
        self.follow("Edit cell")
        self.search("entry", "Entry 2 ")
        self.send(self.page.get_by_role("button", name=re.compile(r"^Choose Entry 2 ·")).inner_text(), status=200)
        self.send("Load selected cell", status=200)
        self.select("Cell state", "known")
        self.search("target_field", "Quantity")
        self.choose("Quantity")
        self.send()
        expect(self.page.get_by_role("main")).to_contain_text("Known reference: Quantity")
        # Public capture pages follow exact references without exposing access data.
        context, _ = self.new_context("/worlds/card-relations-workshop/inspection/records?target=layout")
        self.assertEqual(self.initial_status, 200)
        expect(self.page.get_by_role("main")).to_contain_text("Resource Amount")
        self.follow(self.page.get_by_role("link", name="Where used · exact revision", exact=True).first)
        expect(self.page.get_by_role("main")).to_contain_text("Exact collection layout")
        self.follow("Representative root paths")
        expect(self.page.get_by_role("main")).to_contain_text("Representative root path")
        self.assertEqual(context.cookies(), [])
        self.assertNotIn(self.owner, self.page.content())

    def test_layout_creation_named_field_column_collection_and_row_occurrence(self):
        world = "/worlds/card-relations-workshop"
        self.open(world + "/record-layouts/new")
        self.enter("Display name", "Designer supply record")
        self.enter("Description (inert plain text)", "Original flat authoring data")
        self.send()
        self.follow("Edit column")
        self.select("Column kind", "field")
        self.enter("Column label", "Quantity per occurrence")
        self.search("field", "Quantity")
        self.choose("Quantity")
        self.send()
        self.open(world + "/record-collections/new")
        self.enter("Display name", "Designer supplies")
        self.enter("Description (inert plain text)", "Reusable named collection")
        self.search("layout", "Designer supply record")
        self.choose("Designer supply record")
        self.send()
        self.open(self.card_records("field-satchel"))
        self.follow("Edit collection")
        self.select("Collection operation", "add")
        self.select("Collection value state", "entries")
        self.search("collection", "Designer supplies")
        self.choose("Designer supplies")
        self.send()
        self.follow("Edit row")
        self.select("Entry operation", "add")
        self.search("attached", "Designer supplies")
        self.choose("Designer supplies")
        self.send()
        expect(self.page.get_by_role("main")).to_contain_text("Absent cell")
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])

    def test_explicit_source_column_mappings_confirm_and_schema_three_download(self):
        path = self.card_records("second-wind")
        self.open(path)
        self.follow("Edit copy")
        self.search("source_attached", "Resource Costs")
        self.choose("Resource Costs")
        self.search("collection", "Resource Costs")
        self.choose("Resource Costs")
        for name in ("Resource", "Amount", "Condition"):
            self.select("Source column", self.page.get_by_label("Source column", exact=True).locator("option").filter(has_text=re.compile("^" + name + "$" )).get_attribute("value"))
            self.select("Destination column", self.page.get_by_label("Destination column", exact=True).locator("option").filter(has_text=re.compile("^" + name + "$" )).get_attribute("value"))
            self.send("Add collection or column mapping", status=200)
        self.send("Preview changes", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("Unmapped source cells: 0")
        self.send("Confirm reviewed changes")
        shared = self.page.get_by_role("link", name="Ember · Card revision 1", exact=True)
        self.assertTrue(shared.get_attribute("href").startswith("/cards/type/"), "Source copy preserves exact shared targets rather than silently mapping them locally")
        self.open("/worlds/card-relations-workshop/export")
        self.send("Preview package", status=200)
        digest = self.page.locator('input[name="reviewed_digest"]').input_value()
        checkbox = self.page.get_by_role("checkbox", name="I reviewed this package and its draft diagnostics.", exact=True)
        self.tab_to(checkbox)
        self.page.keyboard.press("Space")
        self.tab_to(self.page.get_by_role("button", name="Confirm and download", exact=True))
        with self.page.expect_download() as pending:
            self.page.keyboard.press("Enter")
        path = Path(self.work.name) / "records.json"
        pending.value.save_as(path)
        package = json.loads(path.read_bytes())
        self.assertEqual(package["manifest"]["schema_version"], 3)
        canonical = json.dumps(package["content"], ensure_ascii=False, separators=(",", ":")).encode()
        self.assertEqual(hashlib.sha256(b"karta-design-package-content-sha256-v3\n" + canonical).hexdigest(), digest)
        self.assertTrue(package["content"]["layouts"])
        self.assertEqual(len(package["content"]["relations"]), len(package["content"]["cards"]["cards"]))
        self.assert_no_horizontal_overflow()


class DesktopRecordTests(RecordWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 900}


class NarrowRecordTests(RecordWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
