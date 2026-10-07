# npm releases

Use the staging helper in the repo root to generate npm tarballs for a release. For
example, to stage the CLI, responses proxy, and SDK packages for version `0.6.0`:

```bash
./scripts/stage_npm_packages.py \
  --release-version 0.6.0 \
  --package codex \
  --package codex-responses-api-proxy \
  --package codex-sdk
```

This downloads the required native package archive artifacts, hydrates `vendor/` for
each package, and writes tarballs to `dist/npm/`.

When `--package codex` is provided, the staging helper builds the lightweight
`@loongphy/codext` meta package plus all platform-native `@loongphy/codext-*`
variants that are later published under platform-specific dist-tags.

Native packages expect `--vendor-src` to point at a prehydrated `vendor/` tree
where each `vendor/<target>/` is a complete codex-package root (`bin/`,
`codex-path/`, `codex-resources/`, `codex-package.json`). Produce those trees
with `scripts/build_codex_package.py`; `install_native_deps.py` lays out the
same structure when installing downloaded binaries for local development.
`scripts/stage_npm_packages.py` remains usable when given artifacts that
contain `codex-package-<target>.tar.gz` archives.

Direct `build_npm_package.py` invocations are still useful for package-specific
debugging. Release packaging should use `scripts/stage_npm_packages.py`
