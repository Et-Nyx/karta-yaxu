"""Exact native Stat workflows; all page interaction is keyboard-only.

Each test owns its temporary database/server. Content is created through native
forms, independently of the complete fixture loader's evolving stat examples.
"""
from pathlib import Path
import subprocess
import unittest
from urllib.parse import urlencode

from playwright.sync_api import expect
from test_authoring import AuthoringSupport


class StatWorkflows(AuthoringSupport):
    def setUp(self):
        super().setUp()
        self.admin("invite-user", "--subject", "librarian", "--role", "administrator")
        self.sign_in("librarian")

    def check(self, label):
        field = self.page.get_by_label(label, exact=True)
        self.tab_to(field)
        self.page.keyboard.press("Space")
        expect(field).to_be_checked()

    def confirm(self):
        self.check("I reviewed the exact pins, proposed values, removals and version changes.")
        self.submit("Confirm stat changes")

    def stat_id(self, name, world=None):
        if world is None:
            return self.sql("SELECT d.id FROM stat_definitions d JOIN stat_versions v ON v.stat_id=d.id AND v.revision=d.revision WHERE d.world_id IS NULL AND v.display_name = ?", (name,))[0][0]
        return self.sql("SELECT d.id FROM stat_definitions d JOIN stat_versions v ON v.stat_id=d.id AND v.revision=d.revision WHERE d.world_id = ? AND v.display_name = ?", (world, name))[0][0]

    def create_stat(self, name="Balance", world=None, source=""):
        if world:
            if self.page.url != self.origin + world:
                self.follow_path(world)
            self.follow("World stat definitions")
            self.follow("Create world stat")
            self.select("Source shared stat and exact version", source)
        else:
            self.follow("Stats")
            self.follow("Create shared stat")
        self.content(name, f"Exact synthetic stat {name}.")
        self.submit("Save stat changes")
        return self.page.url.removeprefix(self.origin)

    def roster_add(self, world, stat):
        self.follow_path(world)
        self.follow("World stat roster")
        self.follow("Edit stat roster")
        self.select("Roster operation", "add")
        self.select("Stat to add or select version", f"{stat}@1")
        self.submit("Preview stat changes", status=200)
        self.confirm()

    def form_stats(self):
        self.follow("Creatures")
        self.follow("Emberling")
        self.follow("exact base stats")
        return self.page.url.removeprefix(self.origin)

    def value_label(self, name, stat):
        return f"Base value for {name} ({stat})"

    def submit_from_end(self, name, status=200):
        button = self.page.get_by_role("button", name=name, exact=True)
        # A page can have 25 assignment/pin/removal groups. Reverse traversal is
        # the native bounded route to navigation controls after those groups.
        for _ in range(100):
            if button.evaluate("element => element === document.activeElement"):
                break
            self.page.keyboard.press("Shift+Tab")
        self.submit(name, status=status)

    def provision_large_form(self):
        root = Path(__file__).resolve().parents[2]
        env = dict(self.env, KARTA_STATS_BROWSER_DATABASE=str(self.database))
        subprocess.run(
            ["cargo", "test", "--locked", "-p", "web-app", "--lib",
             "web::stats_tests::seed_large_browser_stats", "--", "--exact", "--ignored"],
            cwd=root, env=env, check=True, capture_output=True, timeout=300,
        )

    def open_large_form(self):
        self.follow("Creatures")
        self.follow("Large stat creature")
        self.follow("exact base stats")

    def test_large_page_transport_conflict_review_and_readable_later_history(self):
        self.provision_large_form()
        # Complete immutable 301-entry states are validated on writes and history
        # reads. Keep this stress fixture's wait bounded independently of the
        # ordinary five-second small-content keyboard workflows.
        self.page.set_default_timeout(30000)
        self.open_large_form()
        first = self.page
        ids = self.sql("SELECT stat_id FROM stat_versions WHERE scope='' ORDER BY display_name")
        first_id, later_id = ids[0][0], ids[33][0]
        form = self.sql("SELECT default_form_id FROM species WHERE slug='large-stat-creature'")[0][0]
        self.new_context(authenticated_context=first.context)
        self.page.set_default_timeout(30000)
        self.open_large_form()
        self.submit_from_end("Next draft page")
        self.enter(self.value_label("Stat 033", later_id), "34.5")
        self.submit_from_end("Save stat changes", status=303)
        self.page = first
        self.enter(self.value_label("Stat 000", first_id), "-1.25")
        self.submit_from_end("Save stat changes", status=409)
        expect(self.page.get_by_label(self.value_label("Stat 000", first_id), exact=True)).to_have_value("-1.25")
        expect(self.page.locator(f'input[name="value_{later_id}"]')).to_have_count(0)
        self.submit_from_end("Save reviewed changes", status=303)
        self.assertEqual(self.sql(
            "SELECT coefficient,scale FROM form_stat_bindings WHERE form_id=? AND revision=4 AND stat_id=?",
            (form, later_id),
        ), [(345, 1)])
        self.follow("Form history")
        self.follow("Revision 3")
        changes = self.page.locator("section").filter(has=self.page.get_by_role("heading", name="Field changes", exact=True))
        expect(changes).to_contain_text("Stat 033 = 34.5")
        self.follow("Read all historical base stats by page")
        self.follow("Next page")
        expect(self.page.get_by_role("main")).to_contain_text("Stat 033 = 34.5")
        self.open_large_form()
        for page in range(1, 14):
            controls = self.page.locator("form.stat-editor input").evaluate_all(
                "elements => elements.filter(e => e.type !== 'checkbox' || e.checked).map(e => [e.name, e.value])"
            )
            self.assertEqual(sum(name.startswith("value_") for name, _ in controls), 1 if page == 13 else 25)
            self.assertLess(len(urlencode([tuple(pair) for pair in controls])), 12000)
            expect(self.page.locator('input[type="hidden"][name^="value_"]')).to_have_count(0)
            if page < 13:
                self.submit_from_end("Next draft page")
        last = ids[300][0]
        self.enter(self.value_label("Stat 300", last), "not-a-number")
        self.enter("Change reason (optional)", "Retain this page before navigation")
        self.submit_from_end("Previous draft page", status=422)
        expect(self.page.get_by_label(self.value_label("Stat 300", last), exact=True)).to_have_value("not-a-number")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Retain this page before navigation")
        expect(self.page.get_by_role("main")).to_contain_text("Save the current page before navigation")

    def test_changed_roster_position_requires_a_fresh_confirmation(self):
        world = self.create_world("Reviewed roster")
        world_id = self.canonical_id("world")
        self.create_stat("First", world)
        first = self.stat_id("First", world_id)
        self.create_stat("Second", world)
        second = self.stat_id("Second", world_id)
        self.roster_add(world, first)
        self.roster_add(world, second)
        self.follow("Edit stat roster")
        self.select("Roster operation", "remove")
        self.enter("Existing entry position", "1")
        self.submit("Preview stat changes", status=200)
        revision = self.sql("SELECT revision FROM stat_rosters WHERE world_id=?", (world_id,))[0][0]
        self.enter("Existing entry position", "2")
        self.check("I reviewed the exact pins, proposed values, removals and version changes.")
        self.submit("Confirm stat changes", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("Nothing was saved. Review this fresh preview")
        expect(self.page.get_by_label("I reviewed the exact pins, proposed values, removals and version changes.", exact=True)).not_to_be_checked()
        self.assertEqual(self.sql("SELECT revision FROM stat_rosters WHERE world_id=?", (world_id,)), [(revision,)])
        self.confirm()
        self.assertEqual(self.sql(
            "SELECT e.stat_id FROM stat_roster_entries e JOIN stat_rosters r ON r.world_id=e.world_id AND r.id=e.roster_id AND r.revision=e.revision WHERE e.world_id=?",
            (world_id,),
        ), [(first,)])

    def test_definition_raw_decimals_exact_history_and_native_source_adoption(self):
        path = self.create_stat()
        shared = self.stat_id("Balance")
        self.follow("Edit stat definition")
        self.enter("Minimum (inclusive)", "-1.2345678")
        self.enter("Maximum (inclusive)", "1e2")
        self.enter("Change reason (optional)", "Keep raw invalid constraints")
        self.submit("Save stat changes", status=422)
        expect(self.page.get_by_label("Minimum (inclusive)", exact=True)).to_have_value("-1.2345678")
        expect(self.page.get_by_label("Maximum (inclusive)", exact=True)).to_have_value("1e2")
        self.enter("Minimum (inclusive)", "-2.500000")
        self.enter("Maximum (inclusive)", "5")
        self.submit("Save stat changes", destination=path)
        expect(self.page.get_by_role("main")).to_contain_text("Minimum (inclusive): -2.5")
        self.follow("Stat history")
        self.follow("Revision 1")
        expect(self.page.get_by_role("main")).to_contain_text("unbounded")
        world = self.create_world("Stat Adoption Lab")
        world_id = self.canonical_id("world")
        local_path = self.create_stat("Local Balance", world, f"{shared}@2")
        local = self.stat_id("Local Balance", world_id)
        self.follow("Stats")
        self.follow("Balance")
        self.follow("Edit stat definition")
        self.enter("Description", "New source text stays separate.")
        self.submit("Save stat changes")
        self.follow("Worlds")
        self.follow("Stat Adoption Lab")
        self.follow("World stat definitions")
        self.follow("Local Balance")
        self.follow("Review stat source adoption")
        expect(self.page.get_by_label("New shared stat version", exact=True)).to_have_value("3")
        expect(self.page.get_by_label("Display name", exact=True)).to_have_value("Local Balance")
        self.submit("Preview stat changes", status=200)
        for heading in ("Old shared source", "Current local definition", "New shared source", "Proposed definition"):
            expect(self.page.get_by_role("heading", name=heading, exact=True)).to_be_visible()
        self.confirm()
        expect(self.page).to_have_url(self.origin + local_path)
        self.assertEqual(self.sql("SELECT v.source_revision, d.revision FROM stat_definitions d JOIN stat_versions v ON v.stat_id=d.id AND v.revision=d.revision WHERE d.id = ?", (local,)), [(3, 2)])

    def test_roster_unconfigured_empty_reconcile_constraint_fix_and_removed_preview(self):
        world = self.create_world("Roster Workshop")
        world_id = self.canonical_id("world")
        stat_path = self.create_stat("Balance", world)
        stat = self.stat_id("Balance", world_id)
        self.follow_path(world)
        self.follow("World stat roster")
        expect(self.page.get_by_role("main")).to_contain_text("Unconfigured")
        self.follow("Edit stat roster")
        self.submit("Preview stat changes", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("Defined-empty")
        self.confirm()
        self.roster_add(world, stat)
        self.follow_path(world)
        variant = self.create_variant(world, "Exact Variant")
        self.follow("View or edit local base stats")
        expect(self.page.get_by_role("main")).to_contain_text("Unmodeled")
        self.follow("Review roster reconciliation")
        self.enter(self.value_label("Balance", stat), "-1.25")
        self.submit("Preview stat changes", status=200)
        self.confirm()
        expect(self.page.get_by_role("main")).to_contain_text("Balance = -1.25")
        self.follow("Return to variant")
        self.follow_path(world)
        self.follow("World stat definitions")
        self.follow("Balance")
        self.follow("Edit stat definition")
        self.check("Whole numbers only")
        self.submit("Save stat changes", destination=stat_path)
        self.follow_path(world)
        self.follow("World stat roster")
        self.follow("Edit stat roster")
        self.select("Roster operation", "version")
        self.enter("Existing entry position", "1")
        self.select("Stat to add or select version", f"{stat}@2")
        self.submit("Preview stat changes", status=200)
        self.confirm()
        self.follow_path(world)
        self.follow("Exact Variant")
        self.follow("View or edit local base stats")
        expect(self.page.get_by_role("main")).to_contain_text("older exact roster context")
        self.follow("Review roster reconciliation")
        self.enter("Change reason (optional)", "Do not clamp carried decimals")
        self.submit("Preview stat changes", status=422)
        expect(self.page.get_by_label(self.value_label("Balance", stat), exact=True)).to_have_value("-1.25")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Do not clamp carried decimals")
        self.enter(self.value_label("Balance", stat), "")
        self.submit("Preview stat changes", status=200)
        self.confirm()
        expect(self.page.get_by_role("main")).to_contain_text("Balance = Missing")
        self.follow("Return to variant")
        self.follow_path(world)
        self.follow("World stat roster")
        self.follow("Edit stat roster")
        self.select("Roster operation", "remove")
        self.enter("Existing entry position", "1")
        self.submit("Preview stat changes", status=200)
        panel = self.page.locator("section").filter(has=self.page.get_by_role("heading", name="Removed entries", exact=True))
        expect(panel).to_contain_text("Balance")
        self.confirm()
        self.assertEqual(self.sql("SELECT count(*) FROM stat_roster_entries WHERE revision = 4"), [(0,)])

    def test_form_values_source_mapping_keep_local_and_clear_missing_only_selected(self):
        self.create_stat("Reusable")
        shared = self.stat_id("Reusable")
        form_path = self.form_stats()
        self.select("Base stat state", "defined")
        self.select("Add shared stat and exact version", f"{shared}@1")
        self.submit("Save stat changes", destination=form_path)
        self.enter(self.value_label("Reusable", shared), "-0.125000")
        self.submit("Save stat changes")
        expect(self.page.get_by_role("main")).to_contain_text("Reusable = -0.125")
        form = self.sql("SELECT default_form_id FROM species WHERE slug = 'emberling'")[0][0]
        world = self.create_world("Source Values Lab")
        world_id = self.canonical_id("world")
        self.create_stat("Local Reusable", world, f"{shared}@1")
        local = self.stat_id("Local Reusable", world_id)
        self.roster_add(world, local)
        self.follow_path(world)
        self.create_variant(world, "Mapped Variant", f"{form}@3")
        self.follow("View or edit local base stats")
        self.follow("Review roster reconciliation")
        self.enter(self.value_label("Local Reusable", local), "0")
        self.submit("Preview stat changes", status=200)
        self.confirm()
        self.follow("Review source base values")
        self.enter("Source form version", "3")
        self.submit("Preview stat changes", status=200)
        proposed = self.page.locator("section").filter(has=self.page.get_by_role("heading", name="Proposed base stats", exact=True))
        expect(proposed).to_contain_text("Local Reusable = 0")
        self.confirm()
        self.follow("Review source base values")
        label = f"Copy source into Local Reusable ({local})"
        self.select(label, shared)
        self.submit("Preview stat changes", status=200)
        proposed = self.page.locator("section").filter(has=self.page.get_by_role("heading", name="Proposed base stats", exact=True))
        expect(proposed).to_contain_text("Local Reusable = -0.125")
        self.confirm()
        self.form_stats()
        self.enter(self.value_label("Reusable", shared), "")
        self.submit("Save stat changes")
        self.follow("Worlds")
        self.follow("Source Values Lab")
        self.follow("Mapped Variant")
        self.follow("View or edit local base stats")
        self.follow("Review source base values")
        self.select(label, shared)
        self.enter("Source form version", "4")
        self.submit("Preview stat changes", status=200)
        proposed = self.page.locator("section").filter(has=self.page.get_by_role("heading", name="Proposed base stats", exact=True))
        expect(proposed).to_contain_text("Local Reusable = -0.125")
        self.check(f"Clear to Missing when source is Missing for Local Reusable ({local})")
        self.submit("Preview stat changes", status=200)
        expect(proposed).to_contain_text("Local Reusable = Missing")
        self.confirm()
        expect(self.page.get_by_role("main")).to_contain_text("Local Reusable = Missing")
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])

    def test_conflict_review_raw_value_draft_and_admin_has_no_world_bypass(self):
        path = self.create_stat()
        self.follow("Edit stat definition")
        self.enter("Minimum (inclusive)", "-1.25")
        self.enter("Change reason (optional)", "Retained concurrent stat draft")
        first = self.page
        self.new_context()
        self.sign_in("librarian")
        self.follow("Stats")
        self.follow("Balance")
        self.follow("Edit stat definition")
        self.enter("Description", "Winning concurrent edit")
        self.submit("Save stat changes")
        self.page = first
        self.submit("Save stat changes", status=409)
        expect(self.page.get_by_label("Minimum (inclusive)", exact=True)).to_have_value("-1.25")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Retained concurrent stat draft")
        expect(self.page.get_by_role("main")).to_contain_text("Winning concurrent edit")
        self.submit("Save reviewed changes", destination=path)
        admin_context = first.context
        self.new_context()
        self.sign_in("alice")
        world = self.create_world("Editor Stat Scope")
        self.create_stat("Private authoring scope", world)
        local = self.page.url.removeprefix(self.origin)
        self.probe_forbidden(local + "/edit", admin_context)
        self.new_context(local, authenticated_context=admin_context)
        expect(self.link("Edit stat definition")).to_have_count(0)
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])


class DesktopStatTests(StatWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 800}


class NarrowStatTests(StatWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
