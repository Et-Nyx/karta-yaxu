"""Independent learnsets and presets: native keyboard workflows at both widths.

The guarded ignored Rust helper creates 28 acquisitions through real services so
these tests can exercise pagination without depending on evolving setup fixtures.
"""
from pathlib import Path
import subprocess
import unittest
from urllib.parse import urlencode

from playwright.sync_api import expect
from test_authoring import AuthoringSupport


class LearningWorkflows(AuthoringSupport):
    def setUp(self):
        super().setUp()
        self.admin("invite-user", "--subject", "librarian", "--role", "administrator")
        self.sign_in("librarian")
        root = Path(__file__).resolve().parents[2]
        subprocess.run(
            ["cargo", "test", "--locked", "-p", "web-app", "--lib",
             "web::learning_tests::seed_learning_browser", "--", "--exact", "--ignored"],
            cwd=root, env=dict(self.env, CARGO_BUILD_JOBS="1",
                               KARTA_LEARNING_BROWSER_DATABASE=str(self.database)),
            check=True, capture_output=True, timeout=300,
        )
        self.form = self.sql("SELECT default_form_id FROM species WHERE slug='learning-creature'")[0][0]
        self.world = self.sql("SELECT id FROM worlds WHERE slug='learning-world'")[0][0]
        self.variant = self.sql("SELECT id FROM variants WHERE world_id=?", (self.world,))[0][0]
        self.shared_move = self.sql("SELECT id FROM moves WHERE world_id IS NULL")[0][0]
        self.local_move = self.sql("SELECT id FROM moves WHERE world_id=?", (self.world,))[0][0]
        self.shared_learnset = self.sql("SELECT id FROM learnsets WHERE form_id=?", (self.form,))[0][0]
        self.local_learnset = self.sql("SELECT id FROM learnsets WHERE variant_id=?", (self.variant,))[0][0]
        self.local_path = f"/worlds/{self.world}/variants/{self.variant}/learnset"

    def open_learning(self, path):
        self.new_context(path, authenticated_context=self.context)
        # The 28-record stress context verifies immutable dependency histories on
        # reports and writes. Keep this separate from the ordinary 5s UI timeout.
        self.page.set_default_timeout(30000)
        self.assertEqual(self.initial_status, 200)
        self.assert_no_horizontal_overflow()

    def confirm(self, status=303):
        checkbox = self.page.get_by_label(
            "I reviewed the exact pins, records, mappings and proposed changes.", exact=True)
        self.tab_to(checkbox)
        self.page.keyboard.press("Space")
        expect(checkbox).to_be_checked()
        self.assert_transport_budget("save")
        self.submit("Confirm learning changes", status=status)

    def preview(self):
        self.assert_transport_budget("preview")
        self.submit("Preview learning changes", status=200)

    def assert_transport_budget(self, intent):
        fields = self.page.locator("form.stat-editor").evaluate(
            "form => Array.from(new FormData(form).entries())")
        self.assertLess(len(urlencode([tuple(pair) for pair in fields] + [("intent", intent)])), 32768)

    def test_bounded_record_edit_dirty_navigation_and_readable_history(self):
        self.open_learning(f"/forms/{self.form}/learnset")
        expect(self.page.get_by_role("main")).to_contain_text("28 acquisition records")
        self.follow("Edit learnset")
        self.enter("Threshold exact value", "1e2")
        self.enter("Change reason (optional)", "Retain this raw page")
        self.submit("Next draft page", status=422)
        expect(self.page.get_by_label("Threshold exact value", exact=True)).to_have_value("1e2")
        expect(self.page.get_by_role("main")).to_contain_text("Save the current page before navigation")
        self.follow("Discard draft and reload")
        self.submit("Next draft page")
        expect(self.page.get_by_role("main")).to_contain_text("Page 2")
        # Only the displayed three records have edit links; no hidden record drafts.
        buttons = self.page.get_by_role("button", name="Edit acquisition ")
        self.assertEqual(buttons.count(), 3)
        record = buttons.first.inner_text().removeprefix("Edit acquisition ")
        self.submit(f"Edit acquisition {record}")
        self.select("Text field to load", "condition")
        self.submit("Load selected text field")
        expect(self.page.locator("form textarea")).to_have_count(1)
        self.enter("Condition (Unknown if blank)", "🦀" * 2000)
        self.select("Threshold state", "known")
        self.enter("Threshold exact value", "1e2")
        self.enter("Change reason (optional)", "🦉" * 500)
        self.submit("Preview learning changes", status=422)
        expect(self.page.get_by_label("Threshold exact value", exact=True)).to_have_value("1e2")
        expect(self.page.get_by_label("Condition (Unknown if blank)", exact=True)).to_have_value("🦀" * 2000)
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("🦉" * 500)
        self.enter("Threshold exact value", "-2.500000")
        self.preview()
        self.confirm()
        self.follow("Learnset history")
        self.follow("Revision 2")
        self.follow("Next page")
        expect(self.page.get_by_role("main")).to_contain_text(f"Acquisition {record}")
        expect(self.page.get_by_role("main")).to_contain_text("Known -2.5")
        expect(self.page.locator('input[type="hidden"][name^="record_"]')).to_have_count(0)

    def test_explicit_source_mapping_changed_confirmation_and_duplicate_copy(self):
        self.open_learning(self.local_path)
        self.follow("Review learnset source adoption")
        self.enter("Shared learnset and exact version", f"{self.shared_learnset}@1")
        self.select("Move mapping operation", "set")
        self.enter("Shared Move identity to map", self.shared_move)
        self.enter("Local Move and exact version", f"{self.local_move}@1")
        self.preview()
        for name in ("Old shared source", "Current local learnset", "New shared source", "Proposed learnset"):
            expect(self.page.get_by_role("heading", name=name, exact=True)).to_be_visible()
        self.select("Adoption policy", "replace")
        checkbox = self.page.get_by_label(
            "I reviewed the exact pins, records, mappings and proposed changes.", exact=True)
        self.tab_to(checkbox)
        self.page.keyboard.press("Space")
        self.submit("Confirm learning changes", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("Nothing was saved. Review this fresh preview")
        expect(checkbox).not_to_be_checked()
        self.confirm()
        self.assertEqual(self.sql("SELECT revision FROM learnsets WHERE id=?", (self.local_learnset,)), [(2,)])
        self.assertEqual(self.sql(
            "SELECT count(*) FROM learnset_records WHERE learnset_id=? AND revision=2",
            (self.local_learnset,),
        ), [(28,)])
        expect(self.page.get_by_role("main")).to_contain_text("Threshold: Unknown")
        expect(self.page.get_by_role("main")).to_contain_text("Threshold: Not applicable")
        expect(self.page.get_by_role("main")).to_contain_text("Independent shared learnset source")
        self.follow("Learnset history")
        self.follow("Revision 2")
        self.follow("Next page")
        expect(self.page.get_by_role("main")).to_contain_text("Page 2")

    def test_legacy_validation_policy_requires_explicit_reviewed_upgrade(self):
        # Complete setup deliberately retains policy-1 immutable baselines.
        self.admin("seed-fixtures")
        path = "/worlds/variantlab/variants/stats-sample-variant/movesets/repeat-exploration"
        self.open_learning(path)
        expect(self.page.get_by_role("main")).to_contain_text("Validation report · policy 1")
        expect(self.page.get_by_role("main")).to_contain_text("The ruleset has no moveset maximum.")
        self.follow("Edit moveset preset")
        expect(self.page.get_by_label("Validation policy", exact=True)).to_have_value("saved")
        self.enter("Description", "An unrelated edit preserves the saved validation policy.")
        self.preview()
        self.confirm()
        expect(self.page.get_by_role("main")).to_contain_text("Validation report · policy 1")
        self.follow("Edit moveset preset")
        self.select("Validation policy", "current")
        self.preview()
        expect(self.page.get_by_role("main")).to_contain_text("Validation report · policy 2")
        # Changing the choice invalidates the confirmation, even though all
        # content dependency pins and selected acquisition IDs are unchanged.
        self.select("Validation policy", "saved")
        self.confirm(status=200)
        expect(self.page.get_by_role("main")).to_contain_text("Nothing was saved. Review this fresh preview")
        expect(self.page.get_by_label("I reviewed the exact pins, records, mappings and proposed changes.", exact=True)).not_to_be_checked()
        self.select("Validation policy", "current")
        self.preview()
        self.confirm()
        expect(self.page.get_by_role("main")).to_contain_text("Validation report · policy 2")
        expect(self.page.get_by_role("main")).not_to_contain_text("The ruleset has no moveset maximum.")
        self.follow("Preset history")
        self.follow("Revision 1")
        expect(self.page.get_by_role("main")).to_contain_text("Validation report · policy 1")
        expect(self.page.get_by_role("main")).to_contain_text("The ruleset has no moveset maximum.")
        self.assertEqual(self.sql(
            "SELECT revision,validator_version FROM moveset_preset_versions WHERE preset_id=? ORDER BY revision",
            ("46a00000000040008000000000000004",),
        ), [(1, 1), (2, 1), (3, 2)])
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])

    def test_named_preset_preview_repeats_and_historical_validation(self):
        self.open_learning(self.local_path)
        self.follow("Moveset presets")
        self.follow("Create moveset preset")
        self.content("First repertoire", "Descriptive condition, not executed")
        self.select("Entry operation", "add")
        record = self.page.locator("#choices-record option").first.get_attribute("value")
        self.enter("Acquisition record to add", record)
        self.preview()
        expect(self.page.get_by_role("main")).to_contain_text("New preset proposal")
        for name in ("Contradictions", "Incomplete inputs", "Checks not evaluated", "Exact validation inputs"):
            expect(self.page.get_by_role("heading", name=name, exact=True)).to_be_visible()
        self.assertEqual(self.sql("SELECT count(*) FROM moveset_presets"), [(0,)])
        self.confirm()
        preset_path = self.page.url.removeprefix(self.origin)
        self.follow("Edit moveset preset")
        self.select("Entry operation", "add")
        self.enter("Acquisition record to add", record)
        self.preview()
        expect(self.page.get_by_role("main")).to_contain_text("repeats are forbidden")
        self.confirm()
        self.follow("Variant learnset")
        self.follow("Edit learnset")
        self.select("Acquisition operation", "unmodeled")
        self.preview()
        self.confirm()
        self.follow("Moveset presets")
        self.follow("First repertoire")
        expect(self.page.get_by_role("main")).to_contain_text("repeats are forbidden")
        self.follow("Preset history")
        self.follow("Revision 2")
        expect(self.page.get_by_role("main")).to_contain_text(f"{self.local_learnset}@1")
        expect(self.page.get_by_role("main")).to_contain_text("not combat readiness")
        self.follow("Current saved state")
        self.assertEqual(self.page.url, self.origin + preset_path)


class DesktopLearningTests(LearningWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 900}


class NarrowLearningTests(LearningWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
