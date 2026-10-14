# 0005 — Headless keyboard workflow verification

## Context

Rust HTTP tests verify authentication behavior but cannot exercise browser focus,
keyboard navigation, or rendered layout. Development runs in WSL without a GUI.
The maintainer approved test-only Playwright and Chromium dependencies through
Nix, with an isolated browser suite separate from the normal Rust checks.

## Alternatives

- Manual-only testing leaves this environment unable to verify keyboard workflows.
- Selenium/WebDriver is viable but requires a separate browser/driver pairing.
- JavaScript Playwright Test provides richer reporting, but would add a project
  npm toolchain for a small suite. Python Playwright with standard `unittest` is
  sufficient here; its internally packaged Node driver is managed by Nix.

## Decision

- Add `devShells.browser-tests` using the current locked Nixpkgs, Python Playwright,
  matching driver, Chromium only, and packaged fonts. Use full Chromium's headless
  mode (`channel="chromium"`), without WSLg, Xvfb, a display server, pip/npm
  installation, or browser downloads at test runtime.
- Keep the default development shell and `cargo test --workspace` lightweight.
  A separate Python runner builds the current Rust binaries with `--locked`, then
  runs the browser suite. Missing dependencies/builds fail rather than skip tests.
- Each test owns a temporary migrated database, synthetic invited identity,
  loopback server, and fresh browser context. Ignore inherited app configuration;
  never connect to an existing developer server/database. Block browser requests
  outside the test server. Stop owned processes before removing temporary data.
- Drive page interactions with `page.keyboard` only. Locators and read-only DOM
  evaluation assert focus/layout; never use mouse clicks, element `.focus()`,
  `.fill()`, or script-driven form submissions to bypass keyboard reachability.
- Cover skip-link activation, forward/reverse focus, sign-in, sign-out, retained
  input after rejection, and real logout failure/retry. A failure trigger is added
  only to the test-owned database. Run the workflows at desktop and 320 CSS-pixel
  viewport widths, checking focused-control visibility, focus outlines, and
  horizontal overflow.
- Save failure screenshots to a unique run directory under ignored
  `target/browser-tests/`. Do not record traces, cookie values, storage state, or
  raw requests. These tests use synthetic accounts only.

## Consequences and limits

The existing flake lock pins matching Playwright/driver 1.63.0 and browser builds.
First use downloads browser/tool closures into the Nix store; subsequent test runs
use local packages and fixture content. There is no hosting or runtime app cost.
Recheck driver/browser compatibility when updating the lock.

This verifies document-level keyboard interaction in headless Chromium. A 320px
viewport is a reflow check, not an actual 200% browser-zoom test. Computed focus
outlines and screenshots do not replace human readability/contrast assessment,
screen-reader testing, browser-chrome/address-bar interaction, or other browsers.

The Plan/Build agent types requested by AGENTS.md are unavailable. The maintainer
approved the approach and dependencies before direct implementation.
