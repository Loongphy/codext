# Release CI Sync

Run this procedure together with the npm/release work, after implementation is complete and immediately before the final commit and push. This is a release workflow review, not a GitHub Actions monitoring loop.

## Scope

Only carry upstream release workflow changes that affect:

1. Build and shipped artifacts.
2. GitHub Release assets.
3. npm package staging or publication.

Use the artifact flow to decide scope:

```text
build -> upload/download -> archive/vendor -> GitHub Release or npm publish
```

Include setup, signing, checksum, and verification changes when they affect that flow. Exclude unrelated upstream products and jobs, such as website deployment or an independent wheel, unless the current codext release consumes their output.

## Three-way inputs

Use the explicit old base tag. Do not infer it from branch names:

```text
U_OLD = OLD_BASE_TAG:.github/workflows/rust-release.yml
F_OLD = OLD_BRANCH:.github/workflows/rust-release.yml
U_NEW = NEW_TAG:.github/workflows/rust-release.yml
```

Read `git diff U_OLD..U_NEW` to discover upstream's final release changes. Treat `F_OLD` as the working release baseline because it contains the known-good codext packaging flow. Do not replace it with `U_NEW` wholesale.

## Model decision

The model decides which upstream changes to apply. Inventory and `git diff` are evidence, not an automatic patch authority.

For every upstream release change, classify it as:

- `APPLIED`: patch or re-implement it in the existing codext build, GitHub Release, or npm Release flow.
- `OVERRIDE`: retain the codext behavior because it is an intentional long-term fork policy.
- `NOT_APPLIED`: leave it out when it is outside the three release scopes.

When the workflow structures differ, re-implement the upstream behavior in the existing codext structure. Do not force a textual patch or replace the whole workflow.

Do not hardcode a particular upstream binary in this procedure. Discover binaries from the upstream release matrix and trace each one through the artifact flow. Companion binaries required by an existing codext executable are in scope; independent upstream products are not automatically added.

Changes that cannot be proved relevant to the three scopes are not applied. Report them in the final response with the upstream change, its purpose, and why it was not applied.

## Fork overrides

Read `release-overrides.json` before making the decision. It contains only long-lived codext policy, such as package identity, command name, and intentionally different release triggers.

The model may update the manifest automatically when a difference is clearly a durable fork policy. Include that update in the same commit as the release workflow changes. Explain every manifest change in the final response. Do not add one-off tag-specific observations to the manifest.

## Required checks

Before committing and pushing npm/release work:

1. Run `bash .agents/skills/codex-upstream-reapply/scripts/check_release_artifact_parity.sh` as a generic structural preflight.
2. Compare the upstream binary matrix with the current build commands and follow every in-scope output through upload, archive/vendor, and publish steps.
3. Use the upstream workspace-level Cargo invocation for the selected binary set; do not split companion binaries into separate package builds.
4. Run the skill's local build acceptance. Do not run tests, snapshots, formatters, or linters under the reapply guardrails.
5. Include `APPLIED`, `OVERRIDE`, and `NOT_APPLIED` decisions in the final report.

GitHub Actions is a separate follow-up workflow. Do not wait for or repeatedly repair Actions unless the user explicitly asks to read or fix CI.
