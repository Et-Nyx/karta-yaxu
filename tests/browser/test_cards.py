"""Card Foundation native keyboard workflows at desktop and 320px.

The shared fixture owns databases and servers. Provisioning uses native forms;
SQL is read-only and used only to identify the test-created world.
"""
import unittest

from playwright.sync_api import expect
from test_authoring import AuthoringSupport


class CardWorkflows(AuthoringSupport):
    def setUp(self):
        super().setUp()
        self.admin("invite-user", "--subject", "card-librarian", "--role", "administrator")
        self.sign_in("card-librarian")

    def open(self, path):
        self.new_context(path, authenticated_context=self.page.context)
        self.assertEqual(self.initial_status, 200)
        self.assert_no_horizontal_overflow()

    def send(self, name="Save operation", status=303):
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
        self.assert_no_horizontal_overflow()

    def create_card(self, name, world="", source="", source_revision="1"):
        self.open(world + "/cards/new")
        self.enter("Display name", name)
        self.enter("Description (inert plain text)", "Inert standalone card description")
        if source:
            self.enter("Optional shared source identity", source)
            self.enter("Optional shared source revision", source_revision)
        self.send()
        path = self.page.url.removeprefix(self.origin)
        return path, path.rsplit("/", 1)[1]

    def create_field(self, name, kind, world="", source=""):
        self.open(world + "/card-fields/new")
        self.enter("Display name", name)
        self.enter("Description (inert plain text)", "Reusable inert field definition")
        self.select("Field kind", kind)
        if source:
            self.enter("Optional shared source identity", source)
            self.enter("Optional shared source revision", "1")
        self.send()
        path = self.page.url.removeprefix(self.origin)
        return path, path.rsplit("/", 1)[1]

    def assign(self, card, field, state, text="", revision="1", reference=""):
        self.open(card)
        self.follow("Edit assignment")
        self.enter("Field identity (same scope)", field)
        self.enter("Exact field revision", revision)
        self.select("Field value state", state)
        self.enter("Loaded field value (number, text or stable choice ID)", text)
        if reference:
            self.enter("Reference card identity", reference)
            self.enter("Exact reference revision", "1")
        self.send()

    def test_standalone_loaded_unicode_rules_labels_and_exact_history(self):
        path, _ = self.create_card("Standalone ability")
        self.follow("Edit rules_text")
        self.select("Rules text state", "defined")
        prose = "🐉" * 1995 + "\nline"
        self.enter("Loaded rules text (inert plain text)", prose)
        self.send()
        self.follow("Edit label")
        self.enter("New label", "Designer organization")
        self.send()
        self.follow("Edit rules_text")
        expect(self.page.get_by_label("Loaded rules text (inert plain text)", exact=True)).to_have_value(prose)
        self.assertEqual(self.page.locator('textarea[name="text"]').count(), 1)
        self.assertFalse(self.page.locator('input[type="hidden"]').evaluate_all(
            "(elements, prose) => elements.some(element => element.value === prose)", prose
        ), "Loaded prose must not be duplicated in hidden controls")
        self.enter("Loaded rules text (inert plain text)", "Changed <script>inert</script>")
        self.send()
        self.follow("History")
        self.follow("Revision 2")
        expect(self.page.get_by_role("main")).to_contain_text(prose)
        self.open(path)
        expect(self.page.get_by_role("main")).to_contain_text("Designer organization")
        expect(self.page.get_by_role("main")).to_contain_text("Changed <script>inert</script>")

    def test_all_five_field_kinds_false_zero_empty_and_raw_validation(self):
        _, number = self.create_field("Exact cost", "number")
        _, text = self.create_field("Empty authored text", "text")
        _, boolean = self.create_field("Optional marker", "boolean")
        choice_path, choice = self.create_field("Stable category", "choice")
        self.follow("Edit option")
        option = "a" * 32
        self.enter("Stable option identity (blank generates one)", option)
        self.enter("Option label", "First option")
        self.send()
        _, reference = self.create_field("Definition relationship", "card_reference")
        _, target = self.create_card("Referenced definition")
        card, _ = self.create_card("All typed states")
        self.follow("Edit assignment")
        self.enter("Field identity (same scope)", number)
        self.enter("Exact field revision", "1")
        self.select("Field value state", "number")
        self.enter("Loaded field value (number, text or stable choice ID)", "1e2")
        self.send(status=422)
        expect(self.page.get_by_label("Loaded field value (number, text or stable choice ID)", exact=True)).to_have_value("1e2")
        self.enter("Loaded field value (number, text or stable choice ID)", "0")
        self.send()
        self.assign(card, text, "text")
        self.assign(card, boolean, "boolean")
        self.assign(card, choice, "choice", option, revision="2")
        self.assign(card, reference, "card_reference", reference=target)
        for expected in ("Number 0", "Text (empty)", "Boolean false", "Choice stable option", "CardReference shared / standalone"):
            expect(self.page.get_by_role("main")).to_contain_text(expected)
        self.follow("Load assignment Empty authored text")
        expect(self.page.get_by_label("Loaded field value (number, text or stable choice ID)", exact=True)).to_have_value("")
        self.select("Field value state", "unknown")
        self.send()
        expect(self.page.get_by_role("main")).to_contain_text("Unknown")
        self.open(choice_path)
        self.follow("Load option First option")
        self.enter("Option label", "Renamed stable option")
        self.send()
        self.open(card)
        expect(self.page.get_by_role("main")).to_contain_text(option)
        self.assertEqual(self.sql("PRAGMA foreign_key_check"), [])

    def test_world_explicit_field_source_adoption_and_whole_copy_preview(self):
        shared_path, shared = self.create_field("Shared source flag", "boolean")
        _, shared_text = self.create_field("Shared source text", "text")
        source_path, source = self.create_card("Shared source card")
        self.assign(source_path, shared, "boolean")
        self.assign(source_path, shared_text, "text", "Copied exact text")
        self.create_world("Card workshop")
        world_id = self.sql("SELECT id FROM worlds WHERE display_name=?", ("Card workshop",))[0][0]
        world = "/worlds/" + world_id
        local_path, local = self.create_field("Kept local name", "boolean", world, shared)
        _, local_text = self.create_field("Local source text", "text", world, shared_text)
        self.open(shared_path)
        self.follow("Edit name")
        self.enter("Display name", "New shared name")
        self.send()
        self.open(local_path)
        self.follow("Edit adopt")
        self.enter("Shared source revision to adopt (keep local)", "2")
        self.send("Preview changes", status=200)
        for heading in ("Old shared source", "Current local definition", "New shared source", "Proposed state"):
            expect(self.page.get_by_role("heading", name=heading, exact=True)).to_be_visible()
        self.send("Confirm reviewed replacement or adoption")
        expect(self.page.get_by_role("heading", name="Kept local name", exact=True)).to_be_visible()
        card, _ = self.create_card("Local card", world, source, "3")
        self.follow("Edit copy")
        self.enter("Mapped shared field identity (optional initial mapping)", shared)
        self.enter("Mapped local field identity", local)
        self.enter("Exact mapped local field revision", "2")
        mapping_text = f"{shared_text} {local_text} 1"
        self.enter("Additional explicit mappings (one shared_field_id local_field_id revision per line)", mapping_text)
        self.send("Preview changes", status=200)
        expect(self.page.get_by_role("main")).to_contain_text("Boolean false")
        expect(self.page.get_by_label("Additional explicit mappings (one shared_field_id local_field_id revision per line)", exact=True)).to_have_value(mapping_text)
        expect(self.page.get_by_role("main")).to_contain_text("Removed local assignments (0 total")
        self.send("Confirm reviewed replacement or adoption")
        expect(self.page).to_have_url(self.origin + card)
        expect(self.page.get_by_role("main")).to_contain_text("Kept local name")
        expect(self.page.get_by_role("main")).to_contain_text("Boolean false")
        expect(self.page.get_by_role("main")).to_contain_text("Copied exact text")
        # Public card inspection uses the same frozen exact card/field evidence,
        # with ordinary keyboard links into direct and representative root usage.
        public, _ = self.new_context(world + "/inspection/cards")
        self.assertEqual(self.initial_status, 200)
        self.assertEqual(public.cookies(), [])
        expect(self.page.get_by_role("main")).to_contain_text("Coverage: Complete")
        expect(self.page.get_by_role("main")).to_contain_text("Known boolean: false")
        self.follow("Field Kept local name · Where used · exact revision")
        expect(self.page.get_by_role("main")).to_contain_text("Field assignment slot 1")
        self.follow("Representative root paths")
        expect(self.page.get_by_role("heading", name="Representative root path", exact=True)).to_be_visible()
        self.assertEqual(public.cookies(), [])
        self.assert_no_horizontal_overflow()

    def test_native_form_metadata_carries_common_component_and_attributed_history(self):
        self.follow("Creatures")
        self.follow("Create species")
        self.content("Card native creature", "Shared species for native card editing")
        self.submit("Create species")
        species_id = self.canonical_id("species")
        form_id = self.sql("SELECT default_form_id FROM species WHERE id=?", (species_id,))[0][0]
        common = f"/cards/form/{form_id}"
        self.open(common)
        self.follow("Edit rules_text")
        self.select("Rules text state", "defined")
        self.enter("Loaded rules text (inert plain text)", "Preserved native component")
        self.enter("Change reason (optional)", "Attributed custom change")
        self.send()
        self.follow("Specialized metadata editor")
        self.content("Renamed native form", "New specialized description")
        self.submit("Save changes")
        self.open(common)
        expect(self.page.get_by_role("main")).to_contain_text("Preserved native component")
        expect(self.page.get_by_role("heading", name="Renamed native form", exact=True)).to_be_visible()
        self.follow("History")
        expect(self.page.get_by_role("main")).to_contain_text("card-librarian")
        expect(self.page.get_by_role("main")).to_contain_text("Attributed custom change")
        self.follow("Revision 2")
        expect(self.page.get_by_role("main")).to_contain_text("Preserved native component")

    def test_archive_review_and_revoked_editor_keep_raw_draft(self):
        path, _ = self.create_card("Explicit archive review")
        self.follow("Edit archive")
        self.select("Archive state", "yes")
        self.send("Preview changes", status=200)
        expect(self.page.get_by_role("heading", name="Proposed archive state", exact=True)).to_be_visible()
        self.send("Confirm reviewed archive state")
        expect(self.page.get_by_role("main")).to_contain_text("Archived")
        self.follow("Edit rules_text")
        self.select("Rules text state", "defined")
        self.enter("Loaded rules text (inert plain text)", "Keep revoked raw 🐉 text")
        self.enter("Change reason (optional)", "Unsaved revoked reason")
        librarian_id = self.user_id("card-librarian")
        self.admin("disable-user", "--id", librarian_id)
        self.send(status=401)
        expect(self.page.get_by_label("Loaded rules text (inert plain text)", exact=True)).to_have_value("Keep revoked raw 🐉 text")
        expect(self.page.get_by_label("Change reason (optional)", exact=True)).to_have_value("Unsaved revoked reason")
        self.open(path)
        expect(self.page.get_by_role("main")).not_to_contain_text("Keep revoked raw")


class DesktopCardTests(CardWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 800}


class NarrowCardTests(CardWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
