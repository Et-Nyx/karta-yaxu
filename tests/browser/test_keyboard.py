"""The four original authentication keyboard workflows at two viewport widths."""

import unittest

from playwright.sync_api import expect

from support import BrowserFixture


class KeyboardWorkflows(BrowserFixture):
    def test_skip_link_and_reverse_navigation(self):
        self.skip_to_main()
        self.page.keyboard.press("Shift+Tab")
        self.assert_focused_control(self.page.get_by_role("link", name="Account", exact=True))
        self.page.keyboard.press("Shift+Tab")
        self.assert_focused_control(self.page.get_by_role("link", name="Creatures", exact=True))
        self.page.keyboard.press("Tab")
        self.assert_focused_control(self.page.get_by_role("link", name="Account", exact=True))

    def test_keyboard_sign_in_and_sign_out(self):
        self.sign_in()
        self.logout()
        # Use the real keyboard navigation/redirect to confirm loss of access.
        self.open_login_with_keyboard()

    def test_rejected_sign_in_retains_input_and_can_be_corrected(self):
        self.open_login_with_keyboard()
        self.submit_login("not-invited", expected_status=403)
        expect(self.page.get_by_role("alert")).to_contain_text("has not been invited")
        field = self.page.get_by_label("Username", exact=True)
        expect(field).to_have_value("not-invited")
        self.tab_to(field)
        self.page.keyboard.press("Control+A")
        self.submit_login("alice")
        self.logout()

    def test_failed_logout_offers_keyboard_reachable_retry(self):
        self.sign_in()
        self.sql("CREATE TRIGGER reject_browser_logout BEFORE UPDATE ON sessions BEGIN SELECT RAISE(ABORT, 'browser test failure'); END")
        self.logout(expected_status=503)
        expect(self.page.get_by_role("alert")).to_contain_text("session may still be active")
        self.assertTrue(any(cookie["name"] == "karta_session" for cookie in self.context.cookies()))
        self.assertEqual(self.sql("SELECT count(*) FROM sessions WHERE revoked_at IS NULL"), [(1,)])
        retry = self.page.get_by_role("button", name="Try signing out again", exact=True)
        self.tab_to(retry)
        self.sql("DROP TRIGGER reject_browser_logout")
        self.submit_logout()
        self.open_login_with_keyboard()


class DesktopKeyboardTests(KeyboardWorkflows, unittest.TestCase):
    viewport = {"width": 1280, "height": 800}


class NarrowKeyboardTests(KeyboardWorkflows, unittest.TestCase):
    # Reflow at 320 CSS pixels, deliberately not described as browser zoom.
    viewport = {"width": 320, "height": 800}
