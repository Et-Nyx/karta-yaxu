"""Native authoring workflows; every page interaction uses page.keyboard.

SQL reads inspect only the owned database. Content, versions and access changes
are made through real forms. Initial navigation in a fresh context is also used
to probe protected URLs that intentionally have no link for unauthorized users.
"""

import json
import unittest

from playwright.sync_api import expect

from support import BrowserFixture


class AuthoringWorkflows(BrowserFixture):
    def setUp(self):
        super().setUp()
        for subject, role in (("librarian", "administrator"),
                              ("collaborator", "editor"), ("outsider", "editor")):
            self.admin("invite-user", "--subject", subject, "--role", role)

    def link(self, name):
        return self.page.get_by_role("link", name=name, exact=True)

    def follow(self, locator, status=200):
        if isinstance(locator, str):
            locator = self.link(locator)
        self.tab_to(locator)
        # Wait for the local stylesheet as well as the document before checking
        # reflow. DOMContentLoaded alone can precede stylesheet application.
        with self.page.expect_navigation(wait_until="load") as navigation:
            self.page.keyboard.press("Enter")
        self.assertEqual(navigation.value.status, status)
        self.assert_no_horizontal_overflow()

    def follow_path(self, path, status=200):
        self.follow(self.page.locator(f'a[href="{path}"]').first, status)

    def enter(self, label, value):
        field = self.page.get_by_label(label, exact=True)
        self.tab_to(field)
        self.page.keyboard.press("Control+A")
        self.page.keyboard.insert_text(value)
        expect(field).to_have_value(value)

    def content(self, slug, name, description, reason="Browser authoring review"):
        for label, value in (("Slug", slug), ("Display name", name),
                             ("Description", description), ("Change reason (optional)", reason)):
            self.enter(label, value)

    def submit(self, name, status=303, destination=None):
        button = self.page.get_by_role("button", name=name, exact=True)
        action = button.locator("xpath=ancestor::form").get_attribute("action")
        self.tab_to(button)
        with self.page.expect_navigation(wait_until="load"):
            with self.page.expect_response(
                lambda response: response.url == self.origin + action
                and response.request.method == "POST"
            ) as submission:
                self.page.keyboard.press("Enter")
        self.assertEqual(submission.value.status, status)
        if destination is not None:
            expect(self.page).to_have_url(self.origin + destination)
        self.assert_no_horizontal_overflow()

    def create_world(self, slug="alice-world", name="Alice World"):
        self.follow("Worlds")
        self.follow("Create world")
        self.content(slug, name, f"Independent definition for {name}.")
        self.submit("Create world")
        expect(self.page.get_by_role("heading", name=name, exact=True)).to_be_visible()
        return self.page.url.removeprefix(self.origin)

    def select_source(self, value):
        select = self.page.get_by_label("Source form and version", exact=True)
        values = select.locator("option").evaluate_all("options => options.map(option => option.value)")
        self.assertIn(value, values)
        self.tab_to(select)
        self.page.keyboard.press("Home")
        for _ in range(values.index(value)):
            self.page.keyboard.press("ArrowDown")
        self.page.keyboard.press("Tab")
        expect(select).to_have_value(value)

    def create_variant(self, world, slug, name, source=None, description=None):
        self.follow("Add variant")
        self.select_source(source or "")
        self.content(slug, name, description or f"Local values for {name}.")
        self.submit("Create variant")
        path = self.page.url.removeprefix(self.origin)
        self.assertTrue(path.startswith(world + "/variants/"))
        expect(self.page.get_by_role("heading", name=name, exact=True)).to_be_visible()
        return path

    def user_id(self, subject):
        rows = self.sql("SELECT id FROM users WHERE provider_subject = ?", (subject,))
        self.assertEqual(len(rows), 1)
        return rows[0][0]

    def probe_forbidden(self, path, context):
        current = self.page
        self.new_context(path, authenticated_context=context)
        self.assertEqual(self.initial_status, 403)
        expect(self.page.get_by_role("heading", name="Content could not be loaded", exact=True)).to_be_visible()
        expect(self.page.get_by_role("main")).to_contain_text("not permitted")
        self.assert_no_horizontal_overflow()
        self.page = current

    def saved_value(self, label):
        # Inspect a semantic definition list, never change the DOM.
        return self.page.get_by_text(label, exact=True).locator("xpath=following-sibling::dd[1]")

    def snapshot(self, history_link, revision):
        self.follow(history_link)
        self.follow(f"Revision {revision}")
        after = self.page.locator("section").filter(
            has=self.page.get_by_role("heading", name="After", exact=True)
        ).locator("pre")
        result = json.loads(after.inner_text())
        self.assertEqual(result["snapshot_version"], 1)
        self.assert_no_horizontal_overflow()
        return result["content"]

    def test_multiple_variants_and_world_scoped_permissions(self):
        self.sign_in()
        alice_page = self.page
        alice_world = self.create_world()
        world_id = alice_world.rsplit("/", 1)[1]
        form_id = self.sql("SELECT default_form_id FROM species WHERE slug = 'emberling'")[0][0]
        first = self.create_variant(alice_world, "local-one", "Local One", f"{form_id}@1")
        self.follow_path(alice_world)
        second = self.create_variant(alice_world, "local-two", "Local Two", f"{form_id}@1")
        self.assertNotEqual(first, second)
        self.follow_path(alice_world)
        self.create_variant(alice_world, "exclusive", "Exclusive")
        expect(self.saved_value("Source")).to_have_text("World-exclusive · no shared source pin")
        self.assertEqual(self.sql(
            "SELECT source_form_id, source_form_version FROM variants WHERE world_id = ? ORDER BY slug",
            (world_id,),
        ), [(None, None), (form_id, 1), (form_id, 1)])

        self.follow("Creatures")
        self.follow("Emberling")
        species = self.page.url.removeprefix(self.origin)
        expect(self.link("Edit species")).to_have_count(0)
        self.probe_forbidden(species + "/edit", self.context)
        self.probe_forbidden(f"/forms/{form_id}/edit", self.context)

        collaborator_context, collaborator_page = self.new_context()
        self.sign_in("collaborator")
        collaborator_world = self.create_world("collaborator-world", "Collaborator World")
        # Local slugs are scoped to a world, not globally unique.
        self.create_variant(collaborator_world, "local-one", "Other Local One")
        self.follow("Worlds")
        self.follow("Alice World")
        expect(self.link("Edit world")).to_have_count(0)
        expect(self.link("Add variant")).to_have_count(0)
        expect(self.link("Manage collaborators")).to_have_count(0)
        self.probe_forbidden(alice_world + "/edit", collaborator_context)
        self.probe_forbidden(alice_world + "/variants/new", collaborator_context)
        self.page = alice_page
        self.probe_forbidden(collaborator_world + "/edit", self.context)
        self.assertEqual(self.sql("SELECT owner_user_id FROM worlds WHERE id = ?", (world_id,)),
                         [(self.user_id("alice"),)])
        self.assertEqual(self.sql("SELECT owner_user_id FROM worlds WHERE slug = 'collaborator-world'"),
                         [(self.user_id("collaborator"),)])

    def test_shared_library_versions_require_explicit_local_adoption(self):
        self.sign_in("librarian")
        librarian_page = self.page
        self.follow("Creatures")
        self.follow("Create species")
        self.content("lantern-moth", "Lantern Moth", "Shared species description.", "New shared species")
        self.submit("Create species")
        species = self.page.url.removeprefix(self.origin)
        default_id = self.sql("SELECT default_form_id FROM species WHERE slug = 'lantern-moth'")[0][0]
        self.follow_path(f"/forms/{default_id}")
        expect(self.page.get_by_text("Shared form · Default", exact=True)).to_be_visible()
        expect(self.saved_value("Form version")).to_have_text("1")
        self.follow("← Parent species")
        self.follow("Add shared form")
        expect(self.page.get_by_label("Pinned species version", exact=True)).to_have_value("1")
        self.content("dusk-form", "Dusk Form", "Old source description.", "Additional shared form")
        self.submit("Create shared form")
        form = self.page.url.removeprefix(self.origin)
        form_id = form.rsplit("/", 1)[1]
        self.assertEqual(self.sql("SELECT species_id, species_version, is_default FROM forms WHERE id = ?",
                                  (form_id,)), [(species.rsplit("/", 1)[1], 1, 0)])

        alice_context, alice_page = self.new_context()
        self.sign_in()
        world = self.create_world()
        # The shared-form link copies source fields, then local values can diverge.
        source_link = world + f"/variants/new?form_id={form_id}&form_version=1"
        self.follow_path(source_link)
        expect(self.page.get_by_label("Source form and version", exact=True)).to_have_value(f"{form_id}@1")
        expect(self.page.get_by_label("Display name", exact=True)).to_have_value("Dusk Form")
        self.content("local-dusk", "Local Dusk", "Keep this local description.", "Local divergence")
        self.submit("Create variant")
        variant = self.page.url.removeprefix(self.origin)
        self.follow_path(world)
        other = self.create_variant(world, "other-dusk", "Other Dusk", f"{form_id}@1")

        self.page = librarian_page
        self.follow("Edit shared form")
        self.content("dusk-form", "Dusk Form Updated", "New source description.", "Publish form version two")
        self.submit("Save changes", destination=form)
        expect(self.saved_value("Form version")).to_have_text("2")
        self.probe_forbidden(world + "/edit", self.context)
        self.probe_forbidden(world + "/access", self.context)
        form_snapshot = self.snapshot("Form history", 2)
        self.assertEqual(form_snapshot["fields"]["description"], "New source description.")
        expect(self.page.get_by_role("main")).to_contain_text("librarian")
        expect(self.page.locator("pre").first).to_contain_text("Old source description.")

        self.page = alice_page
        self.follow_path(world)
        self.follow_path(variant)
        expect(self.saved_value("Canonical source form ID and pinned version")).to_contain_text("version 1")
        expect(self.page.get_by_role("main")).to_contain_text("Keep this local description.")
        expect(self.page.get_by_role("main")).to_contain_text("library currently has version 2")
        self.assertEqual(self.sql("SELECT source_form_version FROM variants WHERE source_form_id = ?", (form_id,)),
                         [(1,), (1,)])
        self.follow("Review adoption of version 2")
        for heading, description in (("Old source", "Old source description."),
                                     ("New source", "New source description.")):
            panel = self.page.locator("section").filter(has=self.page.get_by_role("heading", name=heading, exact=True))
            expect(panel).to_contain_text(description)
        expect(self.page.get_by_label("Slug", exact=True)).to_have_value("local-dusk")
        expect(self.page.get_by_label("Display name", exact=True)).to_have_value("Local Dusk")
        expect(self.page.get_by_label("Description", exact=True)).to_have_value("Keep this local description.")
        expect(self.page.locator('input[name="target_version"]')).to_have_value("2")
        expect(self.page.locator('input[name="expected_revision"]')).to_have_value("1")
        self.submit("Adopt version 2 with reviewed values", destination=variant)
        expect(self.saved_value("Canonical source form ID and pinned version")).to_contain_text("version 2")
        self.assertEqual(self.sql(
            "SELECT source_form_version, slug, display_name, description FROM variants WHERE id = ?",
            (variant.rsplit("/", 1)[1],),
        ), [(2, "local-dusk", "Local Dusk", "Keep this local description.")])
        self.assertEqual(self.sql("SELECT source_form_version FROM variants WHERE id = ?", (other.rsplit("/", 1)[1],)),
                         [(1,)])
        adopted = self.snapshot("Variant history", 2)
        self.assertEqual(adopted["source"]["version"], 2)
        self.assertEqual(adopted["fields"]["description"], "Keep this local description.")
        expect(self.page.get_by_role("main")).to_contain_text("adopt · alice")
        self.assertEqual(self.sql("SELECT count(*) FROM world_collaborators"), [(0,)])

    def test_validation_preserves_draft_and_history_snapshots_are_keyboard_reachable(self):
        self.sign_in()
        self.follow("Worlds")
        self.follow("Create world")
        description = "Draft <script>inert text</script> & notes.\n\nSecond paragraph."
        reason = "Correct the retained draft"
        self.content("BAD SLUG", "Retained World", description, reason)
        audit_count = self.sql("SELECT count(*) FROM audit_events")
        self.submit("Create world", status=422, destination="/worlds/new")
        expect(self.page.get_by_role("alert")).to_contain_text("entered values have been kept")
        for label, value in (("Slug", "BAD SLUG"), ("Display name", "Retained World"),
                             ("Description", description), ("Change reason (optional)", reason)):
            expect(self.page.get_by_label(label, exact=True)).to_have_value(value)
        expect(self.page.get_by_label("Slug", exact=True)).to_have_attribute("aria-invalid", "true")
        self.assertEqual(self.sql("SELECT count(*) FROM worlds"), [(0,)])
        self.assertEqual(self.sql("SELECT count(*) FROM audit_events"), audit_count)
        self.enter("Slug", "retained-world")
        self.submit("Create world")
        world = self.page.url.removeprefix(self.origin)
        expect(self.page.locator("main script")).to_have_count(0)
        expect(self.page.get_by_role("main")).to_contain_text(description)
        self.follow("Edit world")
        expect(self.page.get_by_label("Description", exact=True)).to_have_value(description)
        self.enter("Description", "Reviewed second version.")
        self.enter("Change reason (optional)", "Keyboard revision reason")
        self.submit("Save changes", destination=world)
        after = self.snapshot("World content history", 2)
        self.assertEqual(after["fields"]["description"], "Reviewed second version.")
        before = json.loads(self.page.locator("pre").first.inner_text())
        self.assertEqual(before["content"]["fields"]["description"], description)
        expect(self.page.get_by_role("main")).to_contain_text("Keyboard revision reason")
        expect(self.page.get_by_role("main")).to_contain_text("edit · alice")
        expect(self.page.locator("main script")).to_have_count(0)
        self.assertNotIn("owner_user_id", after)
        self.assertNotIn("access_revision", after)
        self.follow("← Content history")
        self.follow("Revision 1")
        expect(self.page.get_by_role("main")).to_contain_text("No previous snapshot (initial creation).")
        self.assertEqual(json.loads(self.page.locator("pre").inner_text())["content"]["fields"]["description"], description)

    def test_unicode_limits_allow_non_bmp_characters_at_domain_maximum(self):
        self.sign_in()
        self.follow("Worlds")
        self.follow("Create world")
        name = "🌿" * 100
        description = "🌿" * 2000
        reason = "🌿" * 500
        self.content("unicode-limits", name, description, reason)
        self.submit("Create world")
        self.assertEqual(self.sql(
            "SELECT display_name, description FROM worlds WHERE slug = 'unicode-limits'"
        ), [(name, description)])
        self.follow("World content history")
        self.follow("Revision 1")
        expect(self.page.get_by_role("heading", name="Field changes", exact=True)).to_be_visible()
        snapshot = json.loads(self.page.locator("pre").inner_text())
        self.assertEqual(snapshot["content"]["fields"]["display_name"], name)
        expect(self.page.get_by_role("main")).to_contain_text(reason)

    def test_competing_edits_require_review_and_repeated_conflicts_never_overwrite(self):
        self.sign_in()
        first_page = self.page
        world = self.create_world()
        variant = self.create_variant(world, "contested", "Contested")
        variant_id = variant.rsplit("/", 1)[1]
        self.follow("Edit local values")
        self.enter("Description", "First editor retained draft.")
        self.enter("Change reason (optional)", "Explicitly reviewed winner")
        self.new_context()
        self.sign_in()
        second_page = self.page
        self.follow("Worlds")
        self.follow("Alice World")
        self.follow("Contested")
        self.follow("Edit local values")
        self.enter("Description", "Second editor saved version two.")
        self.submit("Save changes", destination=variant)

        self.page = first_page
        self.submit("Save changes", status=409, destination=variant + "/edit")
        expect(self.page.get_by_role("alert")).to_contain_text("Nothing was saved")
        expect(self.page.locator(".saved-state")).to_contain_text("Second editor saved version two.")
        expect(self.page.get_by_label("Description", exact=True)).to_have_value("First editor retained draft.")
        expect(self.page.locator('input[name="expected_revision"]')).to_have_value("1")
        expect(self.page.get_by_role("button", name="Save reviewed changes", exact=True)).to_have_attribute("value", "2")
        self.assertEqual(self.sql("SELECT revision, description FROM variants WHERE id = ?", (variant_id,)),
                         [(2, "Second editor saved version two.")])
        audit_count = self.sql("SELECT count(*) FROM audit_events")

        self.page = second_page
        self.follow("Edit local values")
        self.enter("Description", "Second editor saved version three.")
        self.submit("Save changes", destination=variant)
        self.page = first_page
        self.submit("Save reviewed changes", status=409, destination=variant + "/edit")
        expect(self.page.locator(".saved-state")).to_contain_text("revision 3")
        expect(self.page.locator(".saved-state")).to_contain_text("Second editor saved version three.")
        expect(self.page.get_by_label("Description", exact=True)).to_have_value("First editor retained draft.")
        expect(self.page.locator('input[name="expected_revision"]')).to_have_value("2")
        expect(self.page.get_by_role("button", name="Save reviewed changes", exact=True)).to_have_attribute("value", "3")
        self.assertEqual(self.sql("SELECT revision, description FROM variants WHERE id = ?", (variant_id,)),
                         [(3, "Second editor saved version three.")])
        self.assertEqual(self.sql("SELECT count(*) FROM audit_events"), [(audit_count[0][0] + 1,)])
        self.submit("Save reviewed changes", destination=variant)
        self.assertEqual(self.sql("SELECT revision, description FROM variants WHERE id = ?", (variant_id,)),
                         [(4, "First editor retained draft.")])
        self.assertEqual(self.sql("SELECT count(*) FROM variant_revisions WHERE variant_id = ?", (variant_id,)), [(4,)])
        after = self.snapshot("Variant history", 4)
        self.assertEqual(after["fields"]["description"], "First editor retained draft.")
        before = json.loads(self.page.locator("pre").first.inner_text())
        self.assertEqual(before["content"]["fields"]["description"], "Second editor saved version three.")
        expect(self.page.get_by_role("main")).to_contain_text("Explicitly reviewed winner")

    def test_owner_grants_and_revokes_without_collaborator_private_access_or_bypass(self):
        self.sign_in()
        owner_page = self.page
        world = self.create_world()
        world_id = world.rsplit("/", 1)[1]
        collaborator_context, collaborator_page = self.new_context()
        self.sign_in("collaborator")
        collaborator_id = self.user_id("collaborator")
        self.follow("Worlds")
        self.follow("Alice World")
        expect(self.link("Edit world")).to_have_count(0)
        self.probe_forbidden(world + "/access", collaborator_context)

        self.page = owner_page
        self.follow("Manage collaborators")
        self.enter("Target user ID", collaborator_id)
        self.enter("Change reason (optional)", "Owner invites collaborator")
        self.submit("Grant collaboration", destination=world + "/access")
        expect(self.page.get_by_role("main")).to_contain_text("Access revision 2 · grant")
        expect(self.page.get_by_role("main")).to_contain_text("Owner invites collaborator")
        self.assertEqual(self.sql("SELECT user_id FROM world_collaborators WHERE world_id = ?", (world_id,)),
                         [(collaborator_id,)])
        self.assertEqual(self.sql(
            "SELECT a.actor_user_id FROM world_access_revisions r JOIN audit_events a ON a.id = r.audit_id WHERE r.world_id = ? AND r.revision = 2",
            (world_id,),
        ), [(self.user_id("alice"),)])

        self.page = collaborator_page
        self.follow("Worlds")
        self.follow("Alice World")
        expect(self.link("Manage collaborators")).to_have_count(0)
        self.probe_forbidden(world + "/access", collaborator_context)
        self.follow("Edit world")
        self.enter("Description", "Collaborator contribution.")
        self.enter("Change reason (optional)", "Collaborator attribution")
        self.submit("Save changes", destination=world)
        self.follow("Edit world")
        self.enter("Description", "Revoked stale draft must not be saved.")

        self.page = owner_page
        self.enter("Revocation reason (optional)", "Owner removes collaboration")
        self.submit("Revoke collaboration for collaborator", destination=world + "/access")
        expect(self.page.get_by_role("main")).to_contain_text("No collaborators. Only the owner")
        expect(self.page.get_by_role("main")).to_contain_text("Access revision 3 · revoke")
        audit_count = self.sql("SELECT count(*) FROM audit_events")
        self.page = collaborator_page
        self.submit("Save changes", status=403, destination=world + "/edit")
        expect(self.page.get_by_label("Description", exact=True)).to_have_value("Revoked stale draft must not be saved.")
        self.assertEqual(self.sql("SELECT revision, description FROM worlds WHERE id = ?", (world_id,)),
                         [(2, "Collaborator contribution.")])
        self.assertEqual(self.sql("SELECT count(*) FROM audit_events"), audit_count)
        self.follow("← Return to content")
        expect(self.link("Edit world")).to_have_count(0)
        after = self.snapshot("World content history", 2)
        self.assertEqual(after["fields"]["description"], "Collaborator contribution.")
        expect(self.page.get_by_role("main")).to_contain_text("edit · collaborator")
        self.assertNotIn("collaborator_ids", after)

        # Keep a genuine owner form open while the shared browser context signs
        # in as a collaborator in a keyboard-opened tab. No forged actor/CSRF or
        # form action is used: the server must authorize the actual current user.
        self.page = owner_page
        self.enter("Target user ID", collaborator_id)
        self.enter("Change reason (optional)", "Unauthorized self grant")
        self.tab_to(self.link("Account"))
        with self.context.expect_page() as new_tab:
            self.page.keyboard.press("Control+Enter")
        self.page = new_tab.value
        self.page.set_default_timeout(5000)
        self.page.wait_for_load_state("load")
        expect(self.page).to_have_url(self.origin + "/account")
        self.submit("Sign out", destination="/")
        self.sign_in("collaborator")
        self.page = owner_page
        self.submit("Grant collaboration", status=403, destination=world + "/access/grant")
        expect(self.page.get_by_role("main")).to_contain_text("Private access state could not be loaded")
        expect(self.page.get_by_role("heading", name="Private access history", exact=True)).to_have_count(0)
        expect(self.page.get_by_role("heading", name="Current saved access — revision 3", exact=True)).to_have_count(0)
        expect(self.page.get_by_label("Target user ID", exact=True)).to_have_value(collaborator_id)
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Unauthorized self grant")
        self.assertEqual(self.sql("SELECT count(*) FROM world_collaborators WHERE world_id = ?", (world_id,)), [(0,)])
        self.assertEqual(self.sql("SELECT count(*) FROM audit_events"), audit_count)

    def test_stale_access_changes_require_explicit_review_of_current_collaborators(self):
        self.sign_in()
        owner_page = self.page
        world = self.create_world()
        world_id = world.rsplit("/", 1)[1]
        for subject in ("collaborator", "outsider"):
            self.new_context()
            self.sign_in(subject)
        collaborator_id = self.user_id("collaborator")
        outsider_id = self.user_id("outsider")
        self.page = owner_page
        self.follow("Manage collaborators")
        self.enter("Target user ID", outsider_id)
        self.enter("Change reason (optional)", "Review outsider grant")

        self.new_context()
        self.sign_in()
        second_owner = self.page
        self.follow("Worlds")
        self.follow("Alice World")
        self.follow("Manage collaborators")
        self.enter("Target user ID", collaborator_id)
        self.submit("Grant collaboration", destination=world + "/access")
        self.page = owner_page
        self.submit("Grant collaboration", status=409, destination=world + "/access/grant")
        expect(self.page.get_by_role("main")).to_contain_text("Current saved access — revision 2")
        expect(self.page.get_by_label("Target user ID", exact=True)).to_have_value(outsider_id)
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Review outsider grant")
        expect(self.page.locator('input[name="expected_access_revision"]')).to_have_value("1")
        button = self.page.get_by_role("button", name="Save reviewed changes", exact=True)
        expect(button).to_have_attribute("name", "reviewed_access_revision")
        expect(button).to_have_attribute("value", "2")
        self.assertEqual(self.sql("SELECT user_id FROM world_collaborators WHERE world_id = ?", (world_id,)), [(collaborator_id,)])

        self.page = second_owner
        self.submit("Revoke collaboration for collaborator", destination=world + "/access")
        self.page = owner_page
        self.submit("Save reviewed changes", status=409, destination=world + "/access/grant")
        expect(self.page.get_by_role("main")).to_contain_text("Current saved access — revision 3")
        expect(self.page.get_by_role("main")).to_contain_text("No collaborators. Only the owner")
        expect(self.page.locator('input[name="expected_access_revision"]')).to_have_value("2")
        expect(self.page.get_by_role("button", name="Save reviewed changes", exact=True)).to_have_attribute("value", "3")
        self.assertEqual(self.sql("SELECT count(*) FROM world_collaborators WHERE world_id = ?", (world_id,)), [(0,)])
        self.assertEqual(self.sql("SELECT access_revision FROM worlds WHERE id = ?", (world_id,)), [(3,)])
        self.submit("Save reviewed changes", destination=world + "/access")
        self.assertEqual(self.sql("SELECT user_id FROM world_collaborators WHERE world_id = ?", (world_id,)), [(outsider_id,)])
        self.assertEqual(self.sql("SELECT count(*) FROM world_access_revisions WHERE world_id = ?", (world_id,)), [(4,)])
        expect(self.page.get_by_role("main")).to_contain_text("Access revision 4 · grant")
        expect(self.page.get_by_role("main")).to_contain_text("Review outsider grant")


class DesktopAuthoringTests(AuthoringWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 800}


class NarrowAuthoringTests(AuthoringWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
