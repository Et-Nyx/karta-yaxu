"""ADR 0008/0009 workflows: exact typing, owned labs and complete dev bootstrap.

Only page.keyboard changes browser state. SQL inspects the disposable database,
except the explicitly marked selector fault injection; target saves always POST
the actual form. Cached provenance is read locally and never fetched.
"""

import json
from pathlib import Path
import unittest

from playwright.sync_api import expect

from test_authoring import AuthoringSupport


class TypesWorkflows(AuthoringSupport):
    seed_reference = True

    def setUp(self):
        super().setUp()
        for subject, role in (("librarian", "administrator"), ("collaborator", "editor")):
            self.admin("invite-user", "--subject", subject, "--role", role)

    def shared_id(self, slug):
        return self.sql("SELECT id FROM types WHERE slug = ?", (slug,))[0][0]

    def typing_panel(self):
        return self.page.locator("section").filter(has=self.page.get_by_role(
            "heading", name="Ordered world-local typing" if "/worlds/" in self.page.url
            else "Ordered typing", exact=True))

    def assert_typing(self, names):
        panel = self.typing_panel()
        if names is None:
            expect(panel).to_contain_text("Typing not modeled")
            expect(panel.locator("li")).to_have_count(0)
        elif not names:
            expect(panel).to_contain_text("Defined — intentionally typeless")
            expect(panel.locator("li")).to_have_count(0)
        else:
            expect(panel.locator("li > a:first-child")).to_have_text(names)
        self.assert_no_horizontal_overflow()

    def draft_typing(self, values, mode="defined", reason="Reviewed ordered typing"):
        self.select("Typing state", mode)
        for index, value in enumerate(values, 1):
            self.select(f"Type slot {index}", value)
        self.enter("Change reason (optional)", reason)

    def assert_draft(self, values, reason):
        expect(self.page.get_by_label("Typing state", exact=True)).to_have_value("defined")
        for index, value in enumerate(values, 1):
            expect(self.page.get_by_label(f"Type slot {index}", exact=True)).to_have_value(value)
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value(reason)

    def pins(self, content):
        assignments = content["typing"]["assignments"]
        return None if assignments is None else [entry["pin"] for entry in assignments]

    def pin(self, type_id, version=1):
        return {"type_id": type_id, "version": version}

    def saved_typing(self, kind, object_id):
        # Inspect authoritative complete snapshots, never the v1 projections.
        table, key = ("form_content_snapshots", "form_id") if kind == "form" else (
            "variant_content_snapshots", "variant_id")
        rows = self.sql(f"SELECT after_json FROM {table} WHERE {key} = ? ORDER BY revision DESC LIMIT 1",
                        (object_id,))
        self.assertEqual(len(rows), 1)
        return json.loads(rows[0][0])["content"]

    def add_type(self, world, name, source=""):
        if self.page.url != self.origin + world:
            self.follow_path(world)
        self.follow("Add type")
        self.assertRegex(self.page.locator('input[name="expected_world_revision"]').input_value(), r"^[1-9][0-9]*$")
        self.select("Source shared type and version", source)
        self.content(name, f"Independent local {name} metadata.")
        self.submit("Add world type")
        type_id = self.canonical_id("type")
        path = self.page.url.removeprefix(self.origin)
        self.assertTrue(path.startswith(world + "/types/"))
        return type_id, path

    def source_form(self, species="charizard", form="charizard"):
        self.follow("Creatures")
        self.follow_path(f"/creatures/{species}")
        self.follow_path(f"/creatures/{species}/forms/{form}")
        return self.canonical_id("form")

    def test_shared_type_creation_rename_history_and_editor_denial(self):
        self.sign_in("librarian")
        self.follow("Types")
        self.follow("Create shared type")
        self.content("Lumé Flux", "Original shared Type metadata.", "Create shared Type")
        self.submit("Create shared type", destination="/types/lume-flux")
        type_id = self.canonical_id("type")
        self.follow("Edit type")
        self.content("Lumé Current", "Revised shared Type metadata.", "Rename shared Type")
        self.submit("Save changes", destination="/types/lume-current")
        self.assertEqual(self.canonical_id("type"), type_id)
        after = self.snapshot("Type history", 2)
        self.assertEqual(after["fields"]["description"], "Revised shared Type metadata.")
        self.assertEqual(after["id"], type_id)
        expect(self.page).to_have_url(self.origin + "/types/lume-current/history/2")
        before = json.loads(self.page.locator("pre").first.inner_text())
        self.assertEqual(before["snapshot_version"], 1)
        self.assertEqual(before["content"]["fields"]["slug"], "lume-flux")
        expect(self.page.get_by_role("main")).to_contain_text("edit · librarian")
        self.follow("← Content history")
        self.follow("← Current content")
        self.follow("Edit type")
        self.enter("Description", "An editor cannot save this administrator draft.")
        stale_page = self.page

        # Change the authenticated actor using the real account UI in a
        # keyboard-opened tab, leaving a valid administrator form behind.
        self.tab_to(self.link("Account"))
        with self.context.expect_page() as new_tab:
            self.page.keyboard.press("Control+Enter")
        self.page = new_tab.value
        self.page.wait_for_load_state("load")
        self.submit("Sign out", destination="/")
        self.sign_in()
        editor_context = self.context
        self.follow("Types")
        expect(self.link("Create shared type")).to_have_count(0)
        self.follow("Lumé Current")
        expect(self.link("Edit type")).to_have_count(0)
        self.probe_forbidden("/types/new", editor_context)
        self.probe_forbidden("/types/lume-current/edit", editor_context)
        count = self.sql("SELECT count(*) FROM audit_events")
        self.page = stale_page
        self.submit("Save changes", status=403)
        expect(self.page.get_by_label("Description", exact=True)).to_have_value(
            "An editor cannot save this administrator draft.")
        self.assertEqual(self.sql("SELECT revision, description FROM types WHERE id = ?", (type_id,)),
                         [(2, "Revised shared Type metadata.")])
        self.assertEqual(self.sql("SELECT count(*) FROM audit_events"), count)

    def test_shared_form_three_ordered_types_dynamic_slots_empty_and_unmodeled(self):
        self.sign_in("librarian")
        form_id = self.source_form("emberling", "emberling")
        form_path = self.page.url.removeprefix(self.origin)
        self.assert_typing(None)
        self.follow("Edit typing")
        expect(self.page).to_have_url(self.origin + form_path + "/typing")
        expect(self.page.locator('select[name^="slot_"]')).to_have_count(3)
        # Expansion is a separate GET before drafting; options do not consume Tab stops.
        self.enter("Displayed slots", "32")
        self.tab_to(self.page.get_by_role("button", name="Show slots", exact=True))
        with self.page.expect_navigation(wait_until="load") as navigation:
            self.page.keyboard.press("Enter")
        self.assertEqual(navigation.value.status, 200)
        self.assertEqual(navigation.value.request.method, "GET")
        expect(self.page.locator('select[name^="slot_"]')).to_have_count(32)
        ids = [self.shared_id(slug) for slug in ("dragon", "fire", "flying")]
        self.select("Typing state", "defined")
        for index, type_id in zip((1, 16, 32), ids):
            self.select(f"Type slot {index}", f"{type_id}@1")
        self.enter("Change reason (optional)", "Three Types, ordered with harmless gaps")
        self.submit("Save typing", destination=form_path)
        self.assert_typing(["Dragon", "Fire", "Flying"])
        content = self.snapshot("Form history", 2)
        self.assertEqual(self.pins(content), [self.pin(value) for value in ids])
        before = json.loads(self.page.locator("pre").first.inner_text())
        # Both old v1 absence and new v2 null mean Unmodeled, never Defined-empty.
        self.assertIsNone(before["content"].get("typing", {"assignments": None})["assignments"])
        self.follow_path(f"/types/{ids[0]}/history/1")
        expect(self.page).to_have_url(self.origin + "/types/dragon/history/1")
        self.assertEqual(self.read_snapshot(1)["id"], ids[0])

        self.follow("Creatures")
        self.follow_path("/creatures/emberling")
        self.follow_path(form_path)
        self.follow("Edit shared form")
        self.enter("Description", "Metadata edit preserves three exact pins.")
        self.submit("Save changes", destination=form_path)
        self.assert_typing(["Dragon", "Fire", "Flying"])
        self.assertEqual(self.pins(self.saved_typing("form", form_id)), [self.pin(value) for value in ids])
        self.follow("Edit typing")
        self.draft_typing(["", "", "", ""], reason="Intentionally typeless")
        self.submit("Save typing", destination=form_path)
        self.assert_typing([])
        self.assertEqual(self.pins(self.saved_typing("form", form_id)), [])
        self.follow("Edit typing")
        self.draft_typing([f"{ids[0]}@1", "", ""], mode="unmodeled", reason="Stop modeling typing")
        self.submit("Save typing", destination=form_path)
        self.assert_typing(None)
        content = self.snapshot("Form history", 5)
        self.assertIsNone(self.pins(content))
        self.assertEqual(json.loads(self.page.locator("pre").first.inner_text())["content"]["typing"],
                         {"assignments": []})

    def test_world_type_versions_need_adoption_then_explicit_assignment(self):
        self.sign_in()
        owner_page = self.page
        world = self.create_world()
        world_id = self.canonical_id("world")
        fire = self.shared_id("fire")
        dragon = self.shared_id("dragon")
        local_fire, fire_path = self.add_type(world, "Local Fire", f"{fire}@1")
        local_dragon, _ = self.add_type(world, "Local Dragon", f"{dragon}@1")
        mist, mist_path = self.add_type(world, "Local Mist")
        expect(self.saved_value("Source")).to_have_text("World-exclusive — no shared source pin")
        self.follow_path(world)
        source = self.sql("SELECT id FROM forms WHERE slug = 'charizard'")[0][0]
        variant = self.create_variant(world, "Triad", f"{source}@1")
        variant_id = self.canonical_id("variant")
        self.assert_typing(None)  # Source pins never imply name-based type mapping.
        self.follow("Edit typing")
        ids = [mist, local_fire, local_dragon]
        self.draft_typing([f"{value}@1" for value in ids])
        self.submit("Save typing", destination=variant)
        self.assert_typing(["Local Mist", "Local Fire", "Local Dragon"])
        self.follow("Edit local values")
        self.enter("Description", "Local metadata preserves ordered typing.")
        self.submit("Save changes", destination=variant)
        baseline = self.saved_typing("variant", variant_id)
        self.assertEqual(self.pins(baseline), [self.pin(value) for value in ids])
        self.follow_path(world)
        membership = self.snapshot("World content history", 5)
        self.assertEqual(membership["type_ids"], sorted(ids))
        self.assertEqual(membership["variant_ids"], [variant_id])

        self.new_context()
        self.sign_in("librarian")
        self.follow("Types")
        self.follow("Fire")
        self.follow("Edit type")
        self.content("Fire Revised", "New shared Fire version.", "Publish shared Type two")
        self.submit("Save changes", destination="/types/fire-revised")
        self.assertEqual(self.canonical_id("type"), fire)
        # The reference form's assignment still names shared Fire at version 1,
        # even though its current Type has a new name and description.
        reference_form = self.source_form()
        self.assert_typing(["Fire", "Flying"])
        self.assertEqual(self.pins(self.saved_typing("form", reference_form))[0], self.pin(fire))
        self.follow(self.typing_panel().get_by_role("link", name="exact version 1", exact=True).first)
        expect(self.page).to_have_url(self.origin + "/types/fire-revised/history/1")
        historical_type = self.read_snapshot(1)
        self.assertEqual(historical_type["fields"]["display_name"], "Fire")

        self.page = owner_page
        self.follow("Worlds")
        self.follow("Alice World")
        self.follow_path(fire_path)
        expect(self.saved_value("Shared source ID and exact version")).to_contain_text("version 1")
        expect(self.page.get_by_role("main")).to_contain_text("currently has version 2")
        self.follow("Edit type")
        self.enter("Description", "Keep independent local Fire metadata.")
        self.submit("Save changes", destination=fire_path)
        expect(self.saved_value("Version")).to_have_text("2")
        self.assertEqual(self.saved_typing("variant", variant_id), baseline)
        self.follow("Review adoption of version 2")
        for heading, text in (("Old shared source", "PokeAPI placeholder Type."),
                              ("New shared source", "New shared Fire version.")):
            panel = self.page.locator("section").filter(has=self.page.get_by_role("heading", name=heading, exact=True))
            expect(panel).to_contain_text(text)
        expect(self.page.get_by_label("Description", exact=True)).to_have_value("Keep independent local Fire metadata.")
        self.submit("Adopt version 2 with reviewed values", destination=fire_path)
        expect(self.saved_value("Version")).to_have_text("3")
        expect(self.saved_value("Shared source ID and exact version")).to_contain_text("version 2")
        adopted = self.snapshot("Type history", 3)
        self.assertEqual(adopted["source"], self.pin(fire, 2))
        self.assertEqual(adopted["fields"]["description"], "Keep independent local Fire metadata.")
        self.assertEqual(self.saved_typing("variant", variant_id), baseline)
        self.assertEqual(self.sql("SELECT revision FROM worlds WHERE id = ?", (world_id,)), [(5,)])

        self.follow("Worlds")
        self.follow("Alice World")
        self.follow("Triad")
        self.assert_typing(["Local Mist", "Local Fire", "Local Dragon"])
        self.follow("Edit typing")
        expect(self.page.get_by_label("Type slot 2", exact=True)).to_have_value(f"{local_fire}@1")
        self.select("Type slot 2", f"{local_fire}@3")
        self.enter("Change reason (optional)", "Explicitly retarget local Type version")
        self.submit("Save typing", destination=variant)
        after = self.snapshot("Variant history", 4)
        self.assertEqual(self.pins(after), [self.pin(mist), self.pin(local_fire, 3), self.pin(local_dragon)])
        self.assertEqual(after["typing"]["assignments"][1]["fields"]["description"],
                         "Keep independent local Fire metadata.")
        old = json.loads(self.page.locator("pre").first.inner_text())["content"]
        self.assertEqual(old, baseline)
        # Exact links target immutable local Type snapshots, including the old
        # version after metadata/adoption advanced the current head.
        self.follow_path(f"/worlds/{world_id}/types/{local_fire}/history/1")
        expect(self.page).to_have_url(self.origin + fire_path + "/history/1")
        self.assertEqual(self.read_snapshot(1)["source"], self.pin(fire))
        self.new_context(mist_path + "/history/1", authenticated_context=self.context)
        self.assertEqual(self.initial_status, 200)
        self.assertIsNone(self.read_snapshot(1)["source"])

    def test_typing_validation_retains_order_reason_and_rejects_wrong_scope(self):
        self.sign_in()
        world = self.create_world()
        world_id = self.canonical_id("world")
        first, _ = self.add_type(world, "First")
        second, _ = self.add_type(world, "Second")
        self.follow_path(world)
        variant = self.create_variant(world, "Validation")
        variant_id = self.canonical_id("variant")
        self.follow("Edit typing")
        values = [f"{second}@1", f"{first}@1", f"{second}@1"]
        reason = "Retain this duplicate draft order"
        self.draft_typing(values, reason=reason)
        counts = self.content_counts()
        self.submit("Save typing", status=422)
        expect(self.page.get_by_role("alert")).to_contain_text("already selected")
        expect(self.page.get_by_label("Type slot 3", exact=True)).to_have_attribute("aria-invalid", "true")
        self.assert_draft(values, reason)
        self.assertEqual(self.content_counts(), counts)
        self.select("Type slot 3", "")
        self.submit("Save typing", destination=variant)
        self.assert_typing(["Second", "First"])
        baseline = self.saved_typing("variant", variant_id)

        # Forge only a selector catalog entry in this disposable DB, not the
        # target variant, bindings, snapshots, CSRF, or browser DOM. A shared ID
        # has no same-world immutable version. Remove the shadow row immediately
        # after rendering, then send its retained value using the real keyboard.
        shared = self.shared_id("fire")
        self.sql("INSERT INTO world_types(id, world_id, slug, display_name, description, revision) VALUES (?, ?, 'forged-shared', 'Forged shared selector', 'Selector fault injection only.', 1)",
                 (shared, world_id))
        try:
            self.follow("Edit typing")
        finally:
            self.sql("DELETE FROM world_types WHERE id = ?", (shared,))
        values = [f"{second}@1", f"{shared}@1", f"{first}@1"]
        reason = "Retain wrong-scope selections without saving"
        self.draft_typing(values, reason=reason)
        counts = self.content_counts()
        self.submit("Save typing", status=422)
        expect(self.page.get_by_role("alert")).to_contain_text("Nothing was saved")
        self.assert_draft(values, reason)
        self.assertEqual(self.content_counts(), counts)
        self.assertEqual(self.saved_typing("variant", variant_id), baseline)
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])
        self.follow("← Return to content")
        self.assert_typing(["Second", "First"])

    def test_typing_conflict_review_and_revoked_collaborator_type_draft(self):
        self.sign_in()
        first_page = self.page
        world = self.create_world()
        local, local_path = self.add_type(world, "Contested Type")
        other, _ = self.add_type(world, "Concurrent Type")
        self.follow_path(world)
        variant = self.create_variant(world, "Contested Typing")
        variant_id = self.canonical_id("variant")
        self.follow("Edit typing")
        values = [f"{local}@1", "", ""]
        reason = "Explicit reviewed typing winner"
        self.draft_typing(values, reason=reason)

        self.new_context()
        self.sign_in()
        second_page = self.page
        self.follow("Worlds")
        self.follow("Alice World")
        self.follow("Contested Typing")
        self.follow("Edit typing")
        self.draft_typing([f"{other}@1", "", ""], reason="Concurrent ordered selection")
        self.submit("Save typing", destination=variant)
        self.page = first_page
        counts = self.content_counts()
        self.submit("Save typing", status=409)
        expect(self.page.locator(".saved-state")).to_contain_text("revision 2")
        expect(self.page.locator(".saved-state")).to_contain_text("Concurrent Type")
        expect(self.page.locator(".saved-state .typing-list li")).to_have_count(1)
        expect(self.page.locator(".saved-state .typing-list li")).to_contain_text("exact version 1")
        self.assert_draft(values, reason)
        expect(self.page.locator('input[name="expected_revision"]')).to_have_value("1")
        expect(self.page.get_by_role("button", name="Save reviewed changes", exact=True)).to_have_attribute("value", "2")
        self.assertEqual(self.content_counts(), counts)

        self.page = second_page
        self.follow("Edit typing")
        self.select("Type slot 1", "")
        self.submit("Save typing", destination=variant)
        self.page = first_page
        counts = self.content_counts()
        self.submit("Save reviewed changes", status=409)
        expect(self.page.locator(".saved-state")).to_contain_text("revision 3")
        expect(self.page.locator(".saved-state")).to_contain_text("Defined — intentionally typeless")
        self.assert_draft(values, reason)
        expect(self.page.locator('input[name="expected_revision"]')).to_have_value("2")
        expect(self.page.get_by_role("button", name="Save reviewed changes", exact=True)).to_have_attribute("name", "reviewed_revision")
        expect(self.page.get_by_role("button", name="Save reviewed changes", exact=True)).to_have_attribute("value", "3")
        self.assertEqual(self.content_counts(), counts)
        self.submit("Save reviewed changes", destination=variant)
        self.assertEqual(self.pins(self.saved_typing("variant", variant_id)), [self.pin(local)])
        self.assert_typing(["Contested Type"])
        after = self.snapshot("Variant history", 4)
        self.assertEqual(self.pins(after), [self.pin(local)])
        expect(self.page.get_by_role("main")).to_contain_text(reason)

        collaborator_context, collaborator_page = self.new_context()
        self.sign_in("collaborator")
        collaborator_id = self.user_id("collaborator")
        self.follow("Worlds")
        self.follow("Alice World")
        expect(self.link("Add type")).to_have_count(0)
        self.probe_forbidden(local_path + "/edit", collaborator_context)
        self.page = first_page
        self.follow("Worlds")
        self.follow("Alice World")
        self.follow("Manage collaborators")
        self.enter("Target user ID", collaborator_id)
        self.submit("Grant collaboration", destination=world + "/access")
        self.page = collaborator_page
        self.follow("Worlds")
        self.follow("Alice World")
        self.follow("Contested Type")
        self.follow("Edit type")
        self.enter("Description", "Collaborator Type metadata.")
        self.submit("Save changes", destination=local_path)
        self.follow("Edit type")
        self.enter("Description", "Revoked stale Type draft.")
        self.enter("Change reason (optional)", "Retained after revocation")
        self.page = first_page
        self.submit("Revoke collaboration for collaborator", destination=world + "/access")
        counts = self.content_counts()
        self.page = collaborator_page
        self.submit("Save changes", status=403)
        expect(self.page.get_by_label("Description", exact=True)).to_have_value("Revoked stale Type draft.")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Retained after revocation")
        self.assertEqual(self.sql("SELECT revision, description FROM world_types WHERE id = ?", (local,)),
                         [(2, "Collaborator Type metadata.")])
        self.assertEqual(self.content_counts(), counts)
        self.probe_forbidden(world + "/access", collaborator_context)

    def content_counts(self):
        tables = ("users", "invites", "species", "forms", "worlds", "variants", "types", "world_types",
                  "species_versions", "form_versions", "world_revisions", "variant_revisions",
                  "type_versions", "world_type_versions", "world_access_revisions", "world_collaborators",
                  "form_content_snapshots", "variant_content_snapshots", "world_content_snapshots",
                  "form_type_bindings", "variant_type_bindings", "audit_events", "reference_provenance")
        return {table: self.sql(f"SELECT count(*) FROM {table}")[0][0] for table in tables}

    def test_complete_seed_bootstraps_admin_for_first_sign_in_and_keeps_owner_edits(self):
        self.assertEqual(self.sql("SELECT count(*) FROM users"), [(0,)])
        self.admin("seed-fixtures")
        owner = self.user_id("admin")
        self.assertEqual(self.sql("SELECT role, is_disabled FROM users"), [("administrator", 0)])
        self.assertEqual(self.sql("SELECT count(*) FROM sessions"), [(0,)])
        self.assertEqual(self.sql("SELECT DISTINCT owner_user_id FROM worlds"), [(owner,)])
        self.assertEqual(self.sql("SELECT count(*) FROM species"), [(4,)])
        self.assertEqual(self.sql("SELECT count(*) FROM forms"), [(5,)])
        self.assertEqual(self.sql("SELECT count(*) FROM variants"), [(5,)])
        counts = self.content_counts()
        self.admin("seed-fixtures")
        self.assertEqual(self.content_counts(), counts)

        self.sign_in("admin")
        self.assertEqual(self.user_id("admin"), owner)
        self.follow("Types")
        for name in ("Fire", "Flying", "Dragon"):
            expect(self.link(name)).to_be_visible()
        expect(self.link("Create shared type")).to_be_visible()
        self.follow("Worlds")
        expect(self.link("VariantLab")).to_be_visible()
        self.follow("ClassicLab")
        self.follow("Manage collaborators")
        expect(self.saved_value("Immutable owner user ID")).to_have_text(owner)
        self.follow("← Return to world")
        self.follow("Edit world")
        self.enter("Description", "Keep the bootstrap owner's authored lab description.")
        self.submit("Save changes", destination="/worlds/classiclab")
        counts = self.content_counts()
        sessions = self.sql("SELECT * FROM sessions")
        self.admin("seed-fixtures")
        self.assertEqual(self.content_counts(), counts)
        self.assertEqual(self.sql("SELECT * FROM sessions"), sessions)
        self.follow("Worlds")
        self.follow("ClassicLab")
        expect(self.page.get_by_role("main")).to_contain_text(
            "Keep the bootstrap owner's authored lab description.")
        self.follow_path("/worlds/classiclab/variants/charizard")
        self.assert_typing(["Fire", "Flying"])
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])

    def test_owned_world_fixture_labs_are_explicit_idempotent_and_cached(self):
        before = self.content_counts()
        result = self.admin("seed-world-fixtures", "--owner-user-id", "0" * 32, check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.content_counts(), before)
        self.assertEqual(self.sql("SELECT count(*) FROM users"), [(0,)])
        self.sign_in()
        owner = self.user_id("alice")  # The real account is created by login, not invitation.
        users = self.sql("SELECT id, provider_subject FROM users ORDER BY id")
        invites = self.sql("SELECT * FROM invites ORDER BY provider_subject")
        self.admin("seed-world-fixtures", "--owner-user-id", owner)
        self.assertEqual(self.sql("SELECT id, provider_subject FROM users ORDER BY id"), users)
        self.assertEqual(self.sql("SELECT * FROM invites ORDER BY provider_subject"), invites)
        self.assertEqual(self.sql("SELECT DISTINCT owner_user_id FROM worlds"), [(owner,)])
        self.follow("Types")
        for name in ("Fire", "Flying", "Dragon"):
            expect(self.link(name)).to_be_visible()
        self.source_form()
        self.assert_typing(["Fire", "Flying"])
        self.follow("← Parent species")
        species = self.snapshot("Species history", 1)
        self.assertEqual(species["fields"]["display_name"], "Charizard")
        self.follow("← Content history")
        self.follow("← Current content")
        self.follow_path("/creatures/charizard/forms/mega-charizard-x")
        self.assert_typing(["Fire", "Dragon"])
        expect(self.page.get_by_role("main")).to_contain_text("Battle-only")
        self.follow("Worlds")
        self.follow("ClassicLab")
        classic = self.page.url.removeprefix(self.origin)
        self.follow("Manage collaborators")
        expect(self.saved_value("Immutable owner user ID")).to_have_text(owner)
        expect(self.page.get_by_role("heading", name="Private access history", exact=True)).to_be_visible()
        initial = self.page.locator("article.history-entry")
        expect(initial).to_have_count(1)
        expect(initial).to_contain_text("bundled-fixture-loader")
        self.tab_to(initial.get_by_text("Inspect private JSON snapshots", exact=True))
        self.page.keyboard.press("Enter")
        private = json.loads(initial.locator("pre").inner_text())
        self.assertEqual(private["snapshot_version"], 1)
        self.assertEqual(private["content"]["owner_user_id"], owner)
        self.follow("← Return to world")
        self.follow_path(classic + "/variants/charizard")
        self.assert_typing(["Fire", "Flying"])
        base_source = self.saved_value("Canonical source form ID and pinned version").inner_text()
        self.follow_path(classic)
        self.follow_path(classic + "/variants/mega-charizard-x")
        self.assert_typing(["Fire", "Dragon"])

        self.follow("Worlds")
        self.follow("VariantLab")
        lab = self.page.url.removeprefix(self.origin)
        # Independent lab entries keep being checked if one entry has a keyboard
        # regression. Subtest failures still fail this workflow and get a screenshot.
        for name, names in (("Charizard Triad", ["Fire", "Flying", "Dragon"]),
                            ("Charizard Mist", ["Mist", "Fire"]), ("Mistling", ["Mist"])):
            with self.subTest(fixture_variant=name):
                self.follow(name)
                self.assert_typing(names)
                if name == "Mistling":
                    expect(self.saved_value("Source")).to_have_text("World-exclusive · no shared source pin")
                    self.follow("Edit local values")
                    self.enter("Description", "Retain the owner's exclusive variant edit.")
                    self.submit("Save changes", destination=lab + "/variants/mistling")
                    self.assert_typing(["Mist"])
                else:
                    self.assertEqual(self.saved_value("Canonical source form ID and pinned version").inner_text(),
                                     base_source)
            self.follow("Worlds")
            self.follow("VariantLab")
        self.follow("Mist")
        expect(self.saved_value("Source")).to_have_text("World-exclusive — no shared source pin")
        self.follow("Edit type")
        self.enter("Description", "Retain the owner's local Mist metadata edit.")
        self.submit("Save changes", destination=lab + "/types/mist")
        self.follow_path(lab)
        self.follow("Edit world")
        self.enter("Description", "Retain the owner's lab metadata edit.")
        self.submit("Save changes", destination=lab)

        # Compare endpoint-specific provenance against cached files, not a remote
        # response. Source identity, retrieval UTC and transform version all matter.
        cache = Path(__file__).resolve().parents[2] / "fixtures" / "pokeapi"
        expected = []
        for path in sorted(cache.glob("*.json")):
            source = json.loads(path.read_text())["source"]
            expected.append(tuple(source[key] for key in (
                "endpoint_kind", "external_id", "source_url", "retrieved_at", "transformation_version")))
        provenance = self.sql("SELECT endpoint_kind, external_id, source_url, retrieved_at, transformation_version FROM reference_provenance")
        self.assertEqual(sorted(provenance), sorted(expected))
        self.assertEqual(len(provenance), 6)
        counts = self.content_counts()
        self.admin("seed-world-fixtures", "--owner-user-id", owner)
        self.admin("seed-reference-fixtures")
        self.assertEqual(self.content_counts(), counts)
        self.assertEqual(self.sql("SELECT id, provider_subject FROM users ORDER BY id"), users)
        self.follow("Worlds")
        self.follow("VariantLab")
        expect(self.page.get_by_role("main")).to_contain_text("Retain the owner's lab metadata edit.")
        self.follow("Mist")
        expect(self.page.get_by_role("main")).to_contain_text("Retain the owner's local Mist metadata edit.")
        expect(self.saved_value("Version")).to_have_text("2")
        self.follow_path(lab)
        self.follow("Mistling")
        expect(self.page.get_by_role("main")).to_contain_text("Retain the owner's exclusive variant edit.")
        self.assert_typing(["Mist"])
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])


class DesktopTypesTests(TypesWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 800}


class NarrowTypesTests(TypesWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
