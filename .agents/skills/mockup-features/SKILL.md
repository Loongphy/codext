---
name: mockup-features
description: Create temporary Codex feature mockups for screenshots, demos, and product review without preserving production code changes.
---

# Mockup Features

Use this skill when the user asks to turn a temporary local feature change into a reusable mockup,
demo, or screenshot aid instead of keeping the implementation in product code.

## Workflow

- Inspect the current diff and identify the user-visible state the mockup was trying to show.
- Capture the smallest repeatable recipe: trigger, fake data, visible UI state, and cleanup notes.
- Prefer an environment-variable gate for screenshot-only behavior, named after the feature and
  scoped so normal runtime behavior is unchanged.
- Keep mockup code isolated and easy to delete. Do not add compatibility fallbacks for a mockup.
- After documenting the recipe in this skill, remove the temporary product-code changes unless the
  user explicitly asks to keep them.

## Usage-Limit Queue Mockup

For screenshots or demos of queued messages during a usage limit:

- Gate the mockup with an env var such as `CODEXT_USAGE_LIMIT_SCREENSHOT`.
- When enabled, inject a rate-limit snapshot that is fully exhausted and resets in a short,
  deterministic-looking interval such as 42 minutes.
- Show the normal usage-limit warning text using the same production formatter as real
  `UsageLimitReachedError` messages.
- Mark queued-message autosend as blocked by the rate limit.
- Seed a couple of Tab-queued follow-up user messages so the queue UI is visible.
- Keep Tab queueing available while rate-limited; when quota recovery is later simulated, only the
  first queued user message should be eligible for autosend and the rest should remain queued for
  FIFO draining.

## Auth Change Account Description Mockup

For screenshots of the TUI account-change notice after `auth.json` changes:

- Gate the mockup with `CODEXT_AUTH_CHANGE_SCREENSHOT`.
- Start the TUI with the env var enabled; no `auth.json` edit is required.
- When enabled, inject a deterministic switch from
  `alex@example.com (Pro)` to `workspace@example.com (Business)`.
- Update the status account state to the destination account so the status panel and history notice
  agree.
- Cleanup: remove `AUTH_CHANGE_SCREENSHOT_ENV_VAR`, `auth_change_screenshot_mock`, and the
  startup injection in `tui/src/app.rs`.

## Status Header Mockup

For screenshots or demos of the TUI status header:

- Gate the mockup with `CODEXT_STATUS_HEADER_SCREENSHOT`.
- Start the TUI with the env var enabled; no backend rate-limit response or `auth.json` edit is
  required.
- When enabled, seed deterministic account state for `ddl@loongphy.com(Pro)`.
- Inject a `codex` rate-limit snapshot with 95% remaining on the primary window and a near reset
  time so the header shows the rate-limit segment immediately.
- Re-apply the mock after app-server account and rate-limit notifications; those events can
  otherwise overwrite the mocked email or Pro plan after startup.
- Header account rendering should prefer the mocked `StatusAccountDisplay` plan label over any
  refreshed internal plan type so `ddl@loongphy.com(Pro)` stays stable.
- Let the normal header renderer supply the real model, cwd, and git segments so screenshots stay
  representative of the current checkout, with account rendered as the final header segment.
- Cleanup: remove `STATUS_HEADER_SCREENSHOT_ENV_VAR`, `status_header_screenshot_mock`,
  `apply_status_header_screenshot_mock`, and the startup injection in `tui/src/app.rs`.

## Cleanup

- Revert any mockup-only imports, fields, helpers, constructor wiring, and test-helper plumbing from
  product code after extracting the recipe.
- Commit only the skill artifacts unless the user requested a production implementation.
