"""Public inspection: three keyboard workflows at desktop and 320px.

Only complete-fixture CLI and ordinary native forms create/edit content. SQL writes
below are explicit integrity-failure injection confined to the owned temporary DB.
"""
import hashlib
import sqlite3
import unittest
from urllib.parse import parse_qs, urlsplit

from playwright.sync_api import expect
from test_authoring import AuthoringSupport

PROBE = "42c00000000040008000000000000001"
CLASSIC = "da05ae68560e4534ae229ce6139494ae"


class InspectionWorkflows(AuthoringSupport):
    # A capture has a ten-second cooperative server budget, followed by analysis
    # and rendering. Configure this before initial navigation too; ordinary
    # small-content workflows keep the fixture's five-second timeout.
    browser_timeout = 30000

    def setUp(self):
        super().setUp()
        self.admin("invite-user", "--subject", "librarian", "--role", "administrator")
        self.sign_in("librarian")
        self.admin("seed-fixtures")
        self.editor_context = self.context

    def snapshot(self):
        # Compare every row without dumping authentication/profile data on failure.
        digest = hashlib.sha256()
        with sqlite3.connect(self.database) as connection:
            for statement in connection.iterdump():
                digest.update(statement.encode())
                digest.update(b"\n")
        return digest.hexdigest()

    def open_public(self, path):
        context, _ = self.new_context(path)
        self.assertEqual(self.initial_status, 200)
        self.assertEqual(len(context.cookies()), 0, "Public inspection must not set cookies")
        self.assert_no_horizontal_overflow()

    def get_submit(self, name):
        button = self.page.get_by_role("button", name=name, exact=True)
        self.assertEqual(button.locator("xpath=ancestor::form").get_attribute("method"), "get")
        self.tab_to(button)
        with self.page.expect_navigation(wait_until="load") as navigation:
            self.page.keyboard.press("Enter")
        self.assertEqual(navigation.value.status, 200)
        self.assert_no_horizontal_overflow()

    def fingerprint(self):
        return self.page.locator('input[name="fingerprint"]').input_value()

    def test_public_filters_pagination_and_saved_policy_comparison(self):
        before = self.snapshot()
        self.open_public("/worlds/variantlab")
        self.follow("Authoring inspection")
        expect(self.page.get_by_role("heading", name="Coverage: Complete", exact=True)).to_be_visible()
        expect(self.page.get_by_role("main")).to_contain_text("not combat or export readiness")
        fingerprint = self.fingerprint()
        self.follow("Next page")
        expect(self.page.get_by_role("main")).to_contain_text("Page 2")
        self.assertEqual(self.fingerprint(), fingerprint)
        self.assertEqual(parse_qs(urlsplit(self.page.url).query)["fingerprint"], [fingerprint])
        self.select("Finding category", "incomplete")
        self.select("Object kind", "preset")
        self.select("Compare saved presets with current policy", "yes")
        self.get_submit("Apply inspection filters")
        expect(self.page.get_by_role("main")).to_contain_text("Page 1")
        expect(self.page.get_by_role("main")).to_contain_text("Saved validator policy 1 (retained)")
        expect(self.page.get_by_role("main")).to_contain_text("validator.preset.saved")
        expect(self.page.get_by_role("main")).to_contain_text("validator.preset.comparison")
        expect(self.page.get_by_role("main")).to_contain_text("Validator policy 2")
        expect(self.page.get_by_role("main")).to_contain_text("Ad-hoc current-policy comparison; no attributed save")
        self.assertEqual(self.fingerprint(), fingerprint)
        self.assertEqual(self.page.locator('form[method="post"]').count(), 0)
        self.assertEqual(self.snapshot(), before)

    def test_exact_all_paths_repeated_slots_and_concurrent_native_edit_refresh(self):
        before = self.snapshot()
        self.open_public("/worlds/variantlab/inspection?kind=move&category=incomplete")
        self.follow(self.page.get_by_role("link", name="Where used · exact revision", exact=True).first)
        expect(self.page.get_by_role("heading", name="Direct uses", exact=True)).to_be_visible()
        expect(self.page.get_by_role("main")).to_contain_text("preset selection slot 1")
        expect(self.page.get_by_role("main")).to_contain_text("preset selection slot 2")
        expect(self.page.get_by_role("main")).to_contain_text("preset selection slot 3")
        expect(self.page.get_by_role("main")).to_contain_text("acquisition slot 1")
        expect(self.page.get_by_role("main")).to_contain_text("acquisition slot 2")
        self.assertEqual(parse_qs(urlsplit(self.page.url).query)["version"], ["1"])
        self.assertEqual(parse_qs(urlsplit(self.page.url).query)["id"], [PROBE])
        self.enter("Revision (all or exact positive revision)", "all")
        self.select("Usage view", "paths")
        self.get_submit("Show where used")
        expect(self.page.get_by_role("heading", name="Representative root paths", exact=True)).to_be_visible()
        expect(self.page.get_by_role("main")).to_contain_text("one canonical shortest representative per root")
        fingerprint = self.fingerprint()
        public_page = self.page
        self.assertEqual(self.snapshot(), before)

        # The competing edit uses a second native browser page. It also renames
        # the parent world, exercising ID-bound links across readable-address changes.
        self.new_context("/worlds/variantlab", authenticated_context=self.editor_context)
        self.follow("Edit world")
        self.enter("Display name", "Inspection <script>renamed</script>")
        self.enter("Description", "Public <img src=x onerror=alert(1)> inert prose")
        self.submit("Save changes")
        after_edit = self.snapshot()
        self.page = public_page
        self.get_submit("Show where used")
        expect(self.page.get_by_role("status", name="Capture refreshed")).to_contain_text("World changed")
        expect(self.page.get_by_role("main")).to_contain_text("Pagination reset to page 1")
        expect(self.page.get_by_role("main")).to_contain_text("Inspection <script>renamed</script>")
        self.assertEqual(self.page.locator("main script, main img").count(), 0)
        self.assertNotEqual(self.fingerprint(), fingerprint)
        self.follow("Refresh inspection")
        self.assertNotIn("fingerprint", parse_qs(urlsplit(self.page.url).query))
        self.assertNotIn("page", parse_qs(urlsplit(self.page.url).query))
        expect(self.page.get_by_role("status", name="Capture refreshed")).to_have_count(0)
        self.assertEqual(self.snapshot(), after_edit)

    def test_partial_corrupt_coverage_privacy_escaping_and_no_mutation_routes(self):
        self.open_public("/worlds/classiclab/inspection")
        old_fingerprint = self.fingerprint()
        # Failure injection: inconsistent current metadata in the temporary DB.
        self.sql("UPDATE forms SET description='corrupt current projection' WHERE id='31e00000000040008000000000000001'")
        before = self.snapshot()
        self.select("Finding category", "not_evaluated")
        self.get_submit("Apply inspection filters")
        expect(self.page.get_by_role("heading", name="Coverage: Partial", exact=True)).to_be_visible()
        expect(self.page.get_by_role("status", name="Capture refreshed")).to_contain_text("previous fingerprint cannot be confirmed")
        expect(self.page.get_by_role("main")).to_contain_text("Integrity coverage gap")
        expect(self.page.get_by_role("main")).to_contain_text("No complete fingerprint is available")
        expect(self.page.get_by_role("main")).not_to_contain_text("usage.unused_in_capture")
        expect(self.page.get_by_role("main")).to_contain_text("No incoming content uses observed in the inspected portion")
        self.assertEqual(self.page.locator('input[name="fingerprint"]').count(), 0)
        self.assertEqual(self.snapshot(), before)
        self.select("Finding category", "incomplete")
        self.select("Object kind", "move")
        self.get_submit("Apply inspection filters")
        self.follow(self.page.get_by_role("link", name="Where used · exact revision", exact=True).first)
        expect(self.page.get_by_role("main")).to_contain_text("Usage coverage: Partial")
        self.enter("Revision (all or exact positive revision)", "99")
        self.get_submit("Show where used")
        expect(self.page.get_by_role("main")).to_contain_text("Target not captured")
        expect(self.page.get_by_role("main")).not_to_contain_text("Unused in this world capture:")
        owner_ids = self.sql("SELECT id FROM users")
        content = self.page.content()
        for (owner_id,) in owner_ids:
            self.assertTrue(owner_id not in content, "Private account identity leaked into public HTML")
        for private in ("owner_user_id", "access_revision", "provider_subject", "after_json", "before_json", old_fingerprint):
            self.assertNotIn(private, content)
        self.assertEqual(self.page.locator('form[method="post"], input[name="csrf"]').count(), 0)
        public_requests = self.page.context.request
        for path in ("/worlds/classiclab/inspection", self.page.url.removeprefix(self.origin)):
            response = public_requests.head(self.origin + path)
            self.assertEqual(response.status, 200)
            self.assertEqual(response.body(), b"")
            self.assertEqual(response.headers.get("cache-control"), "no-store")
            self.assertNotIn("set-cookie", response.headers)
        self.assertEqual(len(self.page.context.cookies()), 0, "Public inspection must not set cookies")
        for suffix in ("inspection", "inspection/where-used"):
            response = self.editor_context.request.post(self.origin + f"/worlds/{CLASSIC}/{suffix}")
            self.assertEqual(response.status, 405)
        self.assert_no_horizontal_overflow()
        self.assertEqual(self.snapshot(), before)


class DesktopInspectionTests(InspectionWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 900}


class NarrowInspectionTests(InspectionWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
