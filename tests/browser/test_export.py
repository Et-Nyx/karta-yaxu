"""Frozen package export: three keyboard workflows at desktop and 320px.

Native forms perform every browser mutation. CLI setup/account administration and
read-only HTTP probes use only the fixture-owned server and temporary database.
Downloads stay in that test's temporary directory; validation needs no extra
binary, Cargo invocation, network access or graphical download dialog.
"""

import hashlib
import json
from pathlib import Path
import sqlite3
import unittest

from playwright.sync_api import expect
from test_authoring import AuthoringSupport


HASH_DOMAIN = b"karta-design-package-content-sha256-v1\n"
REVIEW_LABEL = "I reviewed this package and its draft diagnostics."


class ExportWorkflows(AuthoringSupport):
    browser_timeout = 30000

    def setUp(self):
        super().setUp()
        # Seed ownership goes to the sole enabled administrator, as in inspection.
        self.admin("invite-user", "--subject", "librarian", "--role", "administrator")
        self.sign_in("librarian")
        self.admin("seed-fixtures")

    def database_hash(self):
        # Hash logical rows, including private/session tables, without exposing
        # their contents in assertion output or depending on WAL/checkpoint bytes.
        digest = hashlib.sha256()
        with sqlite3.connect(self.database) as connection:
            for statement in connection.iterdump():
                digest.update(statement.encode())
                digest.update(b"\n")
        return digest.hexdigest()

    def open_world(self):
        self.follow("Worlds")
        self.follow("VariantLab")
        return self.canonical_id("world")

    def open_export(self, world_id):
        self.follow("Export package")
        expect(self.page.get_by_role("heading", name="Export package · VariantLab", exact=True)).to_be_visible()
        expect(self.page.get_by_role("button", name="Confirm and download", exact=True)).to_have_count(0)
        expect(self.page.locator('input[name="reviewed_digest"]')).to_have_count(0)
        expect(self.page.locator('form[method="post"]')).to_have_attribute("action", f"/worlds/{world_id}/export")

    def submit_export_html(self, name, status=200):
        """Preview, refresh or denial must finish as HTML with no attachment."""
        button = self.page.get_by_role("button", name=name, exact=True)
        action = button.locator("xpath=ancestor::form").get_attribute("action")
        self.assertRegex(action, r"^/worlds/[0-9a-f]{32}/export$")
        downloads = []
        def record_download(download):
            downloads.append(download)
        self.page.on("download", record_download)
        try:
            with self.page.expect_response(
                lambda response: response.url == self.origin + action
                and response.request.method == "POST"
            ) as submission:
                self.submit(name, status=status)
            response = submission.value
            self.assertIn("text/html", response.headers.get("content-type", ""))
            self.assertIn("no-store", response.headers.get("cache-control", ""))
            self.assertNotIn("content-disposition", response.headers)
            self.assertEqual(len(downloads), 0, "HTML response released an unexpected download")
        finally:
            self.page.remove_listener("download", record_download)

    def preview_digest(self):
        expect(self.page.get_by_role("heading", name="Reviewed package", exact=True)).to_be_visible()
        digest = self.page.locator('input[name="reviewed_digest"]').input_value()
        self.assertRegex(digest, r"^[0-9a-f]{64}$")
        expect(self.saved_value("Content SHA-256")).to_have_text(digest)
        return digest

    def confirm_review(self):
        checkbox = self.page.get_by_role("checkbox", name=REVIEW_LABEL, exact=True)
        self.assertIsNotNone(checkbox.get_attribute("required"))
        expect(checkbox).not_to_be_checked()
        self.tab_to(checkbox)
        self.page.keyboard.press("Space")
        expect(checkbox).to_be_checked()

    def download_package(self, world_id, digest):
        button = self.page.get_by_role("button", name="Confirm and download", exact=True)
        action = f"/worlds/{world_id}/export"
        expect(button.locator("xpath=ancestor::form")).to_have_attribute("action", action)
        self.tab_to(button)
        # An attachment is not a document navigation: expect_navigation would
        # hang or report an aborted navigation after a successful download.
        with self.page.expect_download() as pending:
            self.page.keyboard.press("Enter")
        download = pending.value
        self.assertEqual(download.url, self.origin + action)
        self.assertEqual(download.suggested_filename, f"karta-{world_id}-{digest[:12]}.json")
        self.assertIsNone(download.failure())
        artifact = Path(self.work.name) / download.suggested_filename
        download.save_as(artifact)
        self.assert_no_horizontal_overflow()
        return self.validate_download(artifact, world_id, digest)

    def validate_download(self, artifact, world_id, reviewed_digest):
        """Independent envelope/count/checksum check, not an engine importer."""
        raw = artifact.read_bytes()
        self.assertGreater(len(raw), 0)
        self.assertLessEqual(len(raw), 32 * 1024 * 1024)

        def unique_fields(pairs):
            result = {}
            for key, value in pairs:
                self.assertNotIn(key, result, "Duplicate JSON field in downloaded package")
                result[key] = value
            return result

        package = json.loads(raw, object_pairs_hook=unique_fields)
        self.assertEqual(set(package), {"manifest", "content"})
        manifest, content = package["manifest"], package["content"]
        self.assertEqual(set(manifest), {
            "format", "schema_version", "producer", "content_sha256", "root_count", "record_count",
        })
        self.assertEqual(manifest["format"], "karta-design-package")
        self.assertEqual(manifest["schema_version"], 1)
        self.assertEqual(manifest["producer"], "karta-yaxu")
        self.assertEqual(set(content), {"schema_version", "root_policy", "world", "roots", "records", "bindings"})
        self.assertEqual(content["schema_version"], 1)
        self.assertEqual(content["root_policy"], "active-world-catalog-v1")
        self.assertEqual(content["world"]["id"], world_id)
        self.assertEqual(content["world"]["kind"], "world")
        self.assertIn(content["world"], content["roots"])
        self.assertEqual(manifest["root_count"], len(content["roots"]))
        self.assertEqual(manifest["record_count"], len(content["records"]))
        self.assertGreater(manifest["root_count"], 0)
        self.assertGreaterEqual(manifest["record_count"], manifest["root_count"])

        def exact_key(key):
            return key["kind"], key["world_id"], key["id"], key["revision"]

        roots = {exact_key(key) for key in content["roots"]}
        records = {exact_key(record["key"]) for record in content["records"]}
        self.assertEqual(len(roots), len(content["roots"]))
        self.assertEqual(len(records), len(content["records"]))
        self.assertTrue(roots <= records, "Active root missing from downloaded exact records")
        for binding in content["bindings"]:
            self.assertIn(exact_key(binding["target"]), records)

        # Python preserves parsed field order. Do not sort object keys or change
        # authored array order; the domain marker is distinct from inspection.
        canonical = json.dumps(content, ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode("utf-8")
        digest = hashlib.sha256(HASH_DOMAIN + canonical).hexdigest()
        self.assertEqual(manifest["content_sha256"], digest)
        self.assertEqual(digest, reviewed_digest)
        self.assertEqual(int(self.saved_value("Download bytes").inner_text()), len(raw))
        self.assertEqual(int(self.saved_value("Active roots").inner_text()), len(content["roots"]))
        self.assertEqual(int(self.saved_value("Exact records including dependencies").inner_text()), len(content["records"]))
        self.assertEqual(int(self.saved_value("Capture-time identity bindings").inner_text()), len(content["bindings"]))

        def check_public(value):
            if isinstance(value, dict):
                for key, child in value.items():
                    self.assertNotIn(key, {
                        "owner_user_id", "access_revision", "collaborator_ids", "actor_user_id",
                        "provider_subject", "sessions", "before_json", "after_json", "csrf",
                    })
                    check_public(child)
            elif isinstance(value, list):
                for child in value:
                    check_public(child)

        check_public(package)
        for (user_id,) in self.sql("SELECT id FROM users"):
            self.assertTrue(user_id.encode() not in raw, "Private account identity leaked into package")
        return content

    def probe_denied_entry(self, context, world_id, status):
        before = self.database_hash()
        for path in ("/worlds/variantlab/export", f"/worlds/{world_id}/export"):
            for method in ("get", "head"):
                response = getattr(context.request, method)(self.origin + path)
                self.assertEqual(response.status, status)
                self.assertIn("no-store", response.headers.get("cache-control", ""))
                self.assertNotIn("content-disposition", response.headers)
                self.assertNotIn("set-cookie", response.headers)
                if method == "head":
                    self.assertEqual(response.body(), b"")
        self.assertEqual(self.database_hash(), before)

    def test_native_preview_draft_policy_review_and_verified_download_do_not_write(self):
        world_id = self.open_world()
        before = self.database_hash()
        self.open_export(world_id)
        self.submit_export_html("Preview package")
        digest = self.preview_digest()
        main = self.page.get_by_role("main")
        expect(main).to_contain_text("Structural acceptance is not combat readiness or engine gameplay acceptance")
        expect(main).to_contain_text("Saved preset validation policies are retained")
        diagnostics = self.page.locator("section").filter(
            has=self.page.get_by_role("heading", name="Separate authoring diagnostics", exact=True)
        )
        expect(diagnostics).to_contain_text("Contradiction")
        expect(diagnostics).to_contain_text("Incomplete")
        expect(diagnostics).to_contain_text("Validator policy 1")
        self.confirm_review()
        content = self.download_package(world_id, digest)
        presets = [record["payload"]["preset"] for record in content["records"]
                   if record["key"]["kind"] == "preset"]
        self.assertTrue(presets, "Fixture presets missing from active-catalog package")
        self.assertTrue(all(preset["validator_version"] == 1 for preset in presets))
        repeated = next(preset for preset in presets if preset["fields"]["slug"] == "repeat-exploration")
        entries = repeated["entries"]
        self.assertEqual(entries, [
            "45a00000000040008000000000000001",
            "45a00000000040008000000000000002",
            "45a00000000040008000000000000001",
        ])
        self.assertEqual(self.database_hash(), before)

    def test_competing_native_rename_refreshes_digest_and_keeps_id_bound_confirmation(self):
        world_id = self.open_world()
        old_revision = int(self.saved_value("Content and membership revision").inner_text())
        self.open_export(world_id)
        self.submit_export_html("Preview package")
        old_digest = self.preview_digest()
        self.confirm_review()
        preview_page = self.page

        # A keyboard-opened second tab shares the real session, but leaves the
        # already reviewed native form and its old digest intact in the first.
        self.tab_to(self.link("← World"))
        with self.page.context.expect_page() as pending:
            self.page.keyboard.press("Control+Enter")
        self.page = pending.value
        self.page.set_default_timeout(self.browser_timeout)
        self.page.wait_for_load_state("load")
        self.assertEqual(self.canonical_id("world"), world_id)
        self.follow("Edit world")
        renamed = "Export Flabébé <script>renamed</script>"
        self.enter("Display name", renamed)
        self.enter("Description", "Frozen draft 🌿 & <img src=x onerror=alert(1)> stays inert.")
        self.submit("Save changes")
        self.assertEqual(self.canonical_id("world"), world_id)
        self.assertNotEqual(self.page.url, self.origin + "/worlds/variantlab")
        after_edit = self.database_hash()

        self.page = preview_page
        self.assertEqual(self.preview_digest(), old_digest)
        self.submit_export_html("Confirm and download")
        expect(self.page.get_by_role("status", name="Package preview refreshed")).to_contain_text("no download was released")
        expect(self.page.get_by_role("heading", name=f"Export package · {renamed}", exact=True)).to_be_visible()
        self.assertEqual(self.page.locator("main script, main img").count(), 0)
        new_digest = self.preview_digest()
        self.assertNotEqual(new_digest, old_digest)
        self.assertEqual(self.database_hash(), after_edit)
        self.confirm_review()
        content = self.download_package(world_id, new_digest)
        world = next(record["payload"]["world"] for record in content["records"]
                     if record["key"] == content["world"])
        self.assertEqual(world["fields"]["display_name"], renamed)
        self.assertEqual(content["world"]["revision"], old_revision + 1)
        self.assertEqual(self.database_hash(), after_edit)

    def test_unauthorized_revoked_collaborator_and_disabled_owner_release_no_attachment(self):
        world_id = self.open_world()
        owner_page = self.page
        public_context, _ = self.new_context("/worlds/variantlab")
        expect(self.link("Export package")).to_have_count(0)
        self.probe_denied_entry(public_context, world_id, 401)
        self.assertEqual(len(public_context.cookies()), 0)

        collaborator_context, collaborator_page = self.new_context()
        self.sign_in("alice")
        self.open_world()
        expect(self.link("Export package")).to_have_count(0)
        self.probe_denied_entry(collaborator_context, world_id, 403)

        # Global administrators have no implicit export permission either.
        self.admin("invite-user", "--subject", "outsider", "--role", "administrator")
        outsider_context, _ = self.new_context()
        self.sign_in("outsider")
        self.open_world()
        expect(self.link("Export package")).to_have_count(0)
        self.probe_denied_entry(outsider_context, world_id, 403)

        self.page = owner_page
        self.follow("Manage collaborators")
        self.enter("Target user ID", self.user_id("alice"))
        self.submit("Grant collaboration")
        self.page = collaborator_page
        self.open_world()
        self.open_export(world_id)
        self.submit_export_html("Preview package")
        self.preview_digest()
        self.confirm_review()

        self.page = owner_page
        self.submit("Revoke collaboration for alice")
        after_revoke = self.database_hash()
        self.page = collaborator_page
        self.submit_export_html("Confirm and download", status=403)
        expect(self.page.get_by_role("heading", name="Package export unavailable", exact=True)).to_be_visible()
        expect(self.page.get_by_role("main")).to_contain_text("No package was released")
        expect(self.page.locator('input[name="reviewed_digest"]')).to_have_count(0)
        self.probe_denied_entry(collaborator_context, world_id, 403)
        self.assertEqual(self.database_hash(), after_revoke)

        self.page = owner_page
        self.follow("← Return to world")
        self.open_export(world_id)
        self.submit_export_html("Preview package")
        self.preview_digest()
        self.confirm_review()
        self.admin("disable-user", "--id", self.user_id("librarian"))
        after_disable = self.database_hash()
        self.submit_export_html("Confirm and download", status=401)
        expect(self.page.get_by_role("main")).to_contain_text("Sign in, then return to export this world")
        expect(self.page.locator('input[name="reviewed_digest"]')).to_have_count(0)
        self.probe_denied_entry(owner_page.context, world_id, 401)
        self.assertEqual(self.database_hash(), after_disable)


class DesktopExportTests(ExportWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 900}


class NarrowExportTests(ExportWorkflows, unittest.TestCase):
    viewport = {"width": 320, "height": 800}
