"""Real-browser workflows, with keyboard-only page interaction.

Run through run.py, which builds the binaries before discovery. Locators and DOM
evaluation only inspect state; no click/focus/fill/script-submit shortcuts.
"""

import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from urllib.parse import urlsplit

from playwright.sync_api import expect, sync_playwright

BIN_DIR = Path(os.environ["KARTA_BROWSER_BIN_DIR"])
WORK_ROOT = Path(__file__).resolve().parents[2] / "target" / "browser-tests"


class BrowserFixture:
    """Lifecycle and inspection helpers, deliberately without discoverable tests."""
    @classmethod
    def setUpClass(cls):
        playwright = sync_playwright().start()
        cls.addClassCleanup(playwright.stop)
        cls.browser = playwright.chromium.launch(channel="chromium", headless=True)
        cls.addClassCleanup(cls.browser.close)

    def setUp(self):
        self.work = tempfile.TemporaryDirectory(prefix="karta-browser-", dir=WORK_ROOT)
        self.addCleanup(self.work.cleanup)
        self.database = Path(self.work.name) / "auth.db"
        # Never inherit a developer database, provider, origin, bind or cookie policy.
        self.env = {key: value for key, value in os.environ.items() if not key.startswith("KARTA_")}
        self.env.update(
            KARTA_DATABASE_URL=f"sqlite://{self.database}",
            KARTA_DEV_AUTH="1",
            KARTA_COOKIE_SECURE="0",
            KARTA_SESSION_TTL_SECONDS="3600",
        )
        self.admin("migrate")
        self.admin("seed-fixtures")
        self.admin("invite-user", "--subject", "alice", "--role", "editor")
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            port = reservation.getsockname()[1]
        self.origin = f"http://127.0.0.1:{port}"
        self.env.update(KARTA_BIND=f"127.0.0.1:{port}", KARTA_PUBLIC_ORIGIN=self.origin)
        self.server_log = tempfile.TemporaryFile()
        self.addCleanup(self.server_log.close)
        self.server = subprocess.Popen(
            [str(BIN_DIR / "web-app")], env=self.env, cwd=self.work.name,
            stdout=self.server_log, stderr=subprocess.STDOUT,
        )
        self.addCleanup(self.stop_server)
        self.wait_for_server()
        self.external_requests = 0
        self.addCleanup(lambda: self.assertEqual(self.external_requests, 0, "Unexpected external browser request"))
        self.context, self.page = self.new_context()

    def new_context(self, path="/", authenticated_context=None):
        options = {"viewport": self.viewport, "locale": "en-US", "service_workers": "block"}
        if authenticated_context is not None:
            # In-memory only: never write or print authentication state.
            options["storage_state"] = authenticated_context.storage_state()
        context = self.browser.new_context(**options)
        self.addCleanup(context.close)
        context.route("**/*", self.local_requests_only)
        page = context.new_page()
        page.set_default_timeout(5000)
        # Keep the failure screenshot pointed at the page currently under test,
        # including a failure during this context's initial navigation.
        self.page = page
        response = page.goto(self.origin + path)
        self.initial_status = response.status
        return context, page

    def admin(self, *args):
        subprocess.run(
            [str(BIN_DIR / "admin"), *args], env=self.env, cwd=self.work.name,
            check=True, capture_output=True, timeout=30,
        )

    def stop_server(self):
        if self.server.poll() is None:
            self.server.terminate()
        try:
            self.server.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.server.kill()
            self.server.wait(timeout=5)

    def wait_for_server(self):
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if self.server.poll() is not None:
                self.fail("The owned test server exited before startup")
            # Read without moving the file offset shared with the child's stdout.
            if self.origin.encode() in os.pread(self.server_log.fileno(), 65_536, 0):
                try:
                    with opener.open(self.origin + "/healthz", timeout=1) as response:
                        if response.status == 200:
                            return
                except (urllib.error.URLError, TimeoutError):
                    pass
            time.sleep(0.05)  # Bounded process readiness polling, not a UI assertion delay.
        self.fail("The owned test server did not become healthy within 10 seconds")

    def local_requests_only(self, route):
        url = urlsplit(route.request.url)
        if f"{url.scheme}://{url.netloc}" == self.origin:
            route.continue_()
        else:
            self.external_requests += 1
            route.abort()

    def sql(self, statement, parameters=()):
        # Failure injection and inspection are confined to the test-owned DB.
        with sqlite3.connect(self.database, timeout=5) as connection:
            return connection.execute(statement, parameters).fetchall()

    def assert_focused_control(self, locator):
        expect(locator).to_be_focused()
        # Chromium scrolls a textarea's caret into view on Tab, rather than its
        # entire multi-line box. Other controls retain the original strict check.
        ratio = 0.1 if locator.evaluate("element => element.tagName === 'TEXTAREA'") else 0.99
        expect(locator).to_be_in_viewport(ratio=ratio)
        style = locator.evaluate("""element => {
            const style = getComputedStyle(element);
            return { keyboardFocus: element.matches(':focus-visible'),
                outline: style.outlineStyle, width: parseFloat(style.outlineWidth),
                color: style.outlineColor };
        }""")
        self.assertTrue(style["keyboardFocus"], "Keyboard focus is not visually indicated")
        self.assertNotEqual(style["outline"], "none")
        self.assertGreaterEqual(style["width"], 2)
        self.assertNotIn(style["color"], ("transparent", "rgba(0, 0, 0, 0)"))
        self.assert_no_horizontal_overflow()

    def assert_no_horizontal_overflow(self):
        overflow = self.page.evaluate(
            "document.documentElement.scrollWidth > window.innerWidth"
        )
        self.assertFalse(overflow, "Page requires horizontal scrolling")

    def tab_to(self, locator):
        # Bound navigation so an unreachable control fails instead of looping.
        for _ in range(80):
            if locator.evaluate("element => element === document.activeElement"):
                self.assert_focused_control(locator)
                return
            self.page.keyboard.press("Tab")
        self.fail("Expected control could not be reached within 80 Tab presses")

    def skip_to_main(self):
        self.page.keyboard.press("Tab")
        self.assert_focused_control(self.page.get_by_role("link", name="Skip to content", exact=True))
        self.page.keyboard.press("Enter")
        expect(self.page.get_by_role("main")).to_be_focused()

    def open_login_with_keyboard(self):
        self.skip_to_main()
        self.page.keyboard.press("Shift+Tab")
        account = self.page.get_by_role("link", name="Account", exact=True)
        self.assert_focused_control(account)
        self.page.keyboard.press("Enter")
        expect(self.page).to_have_url(self.origin + "/login")
        self.skip_to_main()
        self.page.keyboard.press("Tab")
        self.assert_focused_control(self.page.get_by_label("Username", exact=True))

    def submit_login(self, username, expected_status=303):
        field = self.page.get_by_label("Username", exact=True)
        self.assert_focused_control(field)
        self.page.keyboard.type(username)
        self.page.keyboard.press("Tab")
        self.assert_focused_control(self.page.get_by_role("button", name="Sign in", exact=True))
        self.page.keyboard.press("Shift+Tab")
        self.assert_focused_control(field)
        self.page.keyboard.press("Tab")
        with self.page.expect_response(
            lambda response: response.url == self.origin + "/login" and response.request.method == "POST"
        ) as submission:
            self.page.keyboard.press("Enter")
        self.assertEqual(submission.value.status, expected_status)
        if expected_status == 303:
            expect(self.page).to_have_url(self.origin + "/account")
            expect(self.page.get_by_role("heading", name=username, exact=True)).to_be_visible()
            self.assert_no_horizontal_overflow()

    def sign_in(self, username="alice"):
        self.open_login_with_keyboard()
        self.submit_login(username)

    def logout(self, expected_status=303):
        self.skip_to_main()
        self.page.keyboard.press("Tab")
        self.assert_focused_control(self.page.get_by_role("button", name="Sign out", exact=True))
        self.submit_logout(expected_status)

    def submit_logout(self, expected_status=303):
        with self.page.expect_response(
            lambda response: response.url == self.origin + "/logout" and response.request.method == "POST"
        ) as submission:
            self.page.keyboard.press("Enter")
        self.assertEqual(submission.value.status, expected_status)
        if expected_status == 303:
            expect(self.page).to_have_url(self.origin + "/")
            self.assertFalse(any(cookie["name"] == "karta_session" for cookie in self.context.cookies()))
            self.assertEqual(self.sql("SELECT count(*) FROM sessions WHERE revoked_at IS NULL"), [(0,)])
