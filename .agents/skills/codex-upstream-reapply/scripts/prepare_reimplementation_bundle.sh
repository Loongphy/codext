#!/usr/bin/env bash
set -euo pipefail

print_usage() {
  cat <<'EOF'
prepare_reimplementation_bundle.sh

Create a "re-implementation bundle" from an old customization branch:
- compute BASE_COMMIT vs a selected tag (or explicit old base tag)
- export changed file list + diff patch + commit list
- copy changed Markdown intent docs (and optionally all changed files) for offline reading

Usage:
  prepare_reimplementation_bundle.sh [options]

Options:
  --old-branch <name>     Old customization branch (default: current branch)
  --base-ref <ref>        Selected tag (or commit ref) used to infer merge-base (required)
  --old-base-tag <tag>    Explicit base tag for OLD_BRANCH (overrides merge-base inference)
  --remote <remote>       Remote for optional tag fetch (default: upstream)
  --out <dir>             Output directory (default: /tmp/codex-upstream-reapply/<repo>/<old>/<timestamp>)
  --copy-all              Copy ALL changed files (ACMR) from old branch into bundle/old/
  --no-copy-docs          Do not copy changed Markdown docs into bundle/old/ (docs are copied by default)
  --no-fetch              Do not run git fetch (default: fetch tags best-effort)
  -h, --help              Show help

Outputs:
  META.md, changed-files.txt, diff.patch, diffstat.txt, commits.txt, old/...
EOF
}

die() {
  echo "[ERROR] $*" >&2
  exit 1
}

timestamp_utc() {
  date -u +"%Y%m%dT%H%M%SZ"
}

is_markdown_path() {
  local path="$1"
  local lower="${path,,}"
  case "${lower}" in
    *.md|*.mdx|*.markdown)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

require_git_repo() {
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || die "Not inside a git repository."
}

ensure_no_in_progress_ops() {
  git rev-parse -q --verify REBASE_HEAD >/dev/null 2>&1 && die "Rebase in progress. Finish it first (git rebase --continue/--abort)."
  git rev-parse -q --verify CHERRY_PICK_HEAD >/dev/null 2>&1 && die "Cherry-pick in progress. Finish it first."
  git rev-parse -q --verify MERGE_HEAD >/dev/null 2>&1 && die "Merge in progress. Finish it first."
  return 0
}

require_ref() {
  local ref="$1"
  git rev-parse --verify "${ref}^{commit}" >/dev/null 2>&1 || die "Ref not found: ${ref}"
}

ref_commit() {
  git rev-parse "${1}^{commit}"
}

hint_tag_from_history() {
  git describe --tags --abbrev=0 "${1}" 2>/dev/null || true
}

OLD_BRANCH=""
BASE_REF=""
OLD_BASE_TAG=""
REMOTE="upstream"
OUT_DIR=""
COPY_ALL=0
COPY_DOCS=1
NO_FETCH=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --old-branch)
      OLD_BRANCH="${2:-}"
      shift 2
      ;;
    --base-ref)
      BASE_REF="${2:-}"
      shift 2
      ;;
    --old-base-tag)
      OLD_BASE_TAG="${2:-}"
      shift 2
      ;;
    --remote)
      REMOTE="${2:-}"
      shift 2
      ;;
    --out)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --copy-all)
      COPY_ALL=1
      shift
      ;;
    --no-copy-docs)
      COPY_DOCS=0
      shift
      ;;
    --no-fetch)
      NO_FETCH=1
      shift
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    *)
      die "Unknown argument: $1 (use --help)"
      ;;
  esac
done

require_git_repo
ensure_no_in_progress_ops

if [[ -z "${OLD_BRANCH}" ]]; then
  OLD_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
fi

[[ -n "${OLD_BRANCH}" ]] || die "--old-branch resolved to empty"
[[ "${OLD_BRANCH}" != "HEAD" ]] || die "Detached HEAD; pass --old-branch <name>."
[[ -n "${BASE_REF}" ]] || die "--base-ref is required (selected tag or commit ref)"
[[ -n "${REMOTE}" ]] || die "--remote must not be empty"

if [[ "${NO_FETCH}" != "1" ]]; then
  echo "[INFO] Fetching tags from ${REMOTE} (best-effort)..."
  if ! git fetch "${REMOTE}" --tags --prune; then
    echo "[WARN] git fetch failed; continuing with local refs."
  fi
fi

require_ref "${OLD_BRANCH}"
require_ref "${BASE_REF}"
[[ -z "${OLD_BASE_TAG}" ]] || require_ref "${OLD_BASE_TAG}"

repo_root="$(git rev-parse --show-toplevel)"
repo_name="$(basename "${repo_root}")"
ts="$(timestamp_utc)"

if [[ -z "${OUT_DIR}" ]]; then
  OUT_DIR="/tmp/codex-upstream-reapply/${repo_name}/${OLD_BRANCH}/${ts}"
fi

mkdir -p "${OUT_DIR}"

old_commit="$(git rev-parse "${OLD_BRANCH}")"
base_ref_commit="$(git rev-parse "${BASE_REF}")"
merge_base="$(git merge-base "${BASE_REF}" "${OLD_BRANCH}" 2>/dev/null || true)"

if [[ -z "${merge_base}" ]]; then
  if [[ -n "${OLD_BASE_TAG}" ]]; then
    echo "[WARN] Unable to compute merge-base between ${BASE_REF} and ${OLD_BRANCH}; will use --old-base-tag."
  else
    die "Unable to compute merge-base between ${BASE_REF} and ${OLD_BRANCH}. Provide --old-base-tag."
  fi
fi

base_commit="${merge_base}"
old_base_tag_commit=""
hint_tag="$(hint_tag_from_history "${OLD_BRANCH}")"
hint_tag_commit=""

if [[ -n "${hint_tag}" ]]; then
  hint_tag_commit="$(ref_commit "${hint_tag}")"
fi

if [[ -n "${OLD_BASE_TAG}" ]]; then
  old_base_tag_commit="$(ref_commit "${OLD_BASE_TAG}")"
  if ! git merge-base --is-ancestor "${old_base_tag_commit}" "${OLD_BRANCH}"; then
    die "--old-base-tag ${OLD_BASE_TAG} is not an ancestor of ${OLD_BRANCH}"
  fi
  base_commit="${old_base_tag_commit}"
else
  if [[ -n "${hint_tag_commit}" ]]; then
    if ! git merge-base --is-ancestor "${hint_tag_commit}" "${base_commit}"; then
      die "Inferred base (${base_commit}) conflicts with hint tag (${hint_tag}). Re-run with --old-base-tag <tag>."
    fi
  fi
fi

echo "[INFO] Repo:      ${repo_root}"
echo "[INFO] Remote:    ${REMOTE}"
echo "[INFO] Tag/Base:  ${BASE_REF}"
echo "[INFO] OLD:       ${OLD_BRANCH}"
echo "[INFO] OUT:       ${OUT_DIR}"
echo "[INFO] merge-base ${merge_base}"

cat > "${OUT_DIR}/META.md" <<EOF
# Re-implementation Bundle

- repo_root: \`${repo_root}\`
- repo_name: \`${repo_name}\`
- created_utc: \`${ts}\`
- remote: \`${REMOTE}\`
- base_ref: \`${BASE_REF}\`
- base_ref_commit: \`${base_ref_commit}\`
- old_base_tag: \`${OLD_BASE_TAG}\`
- old_base_tag_commit: \`${old_base_tag_commit}\`
- hint_tag: \`${hint_tag}\`
- hint_tag_commit: \`${hint_tag_commit}\`
- base_commit: \`${base_commit}\`
- old_branch: \`${OLD_BRANCH}\`
- old_commit: \`${old_commit}\`
- merge_base: \`${merge_base}\`

## Suggested next step

\`\`\`bash
git fetch ${REMOTE} --tags --prune
git switch -c <NEW_BRANCH> ${BASE_REF}
\`\`\`
EOF

git diff --name-status "${base_commit}..${OLD_BRANCH}" > "${OUT_DIR}/changed-files.txt"
git diff --stat "${base_commit}..${OLD_BRANCH}" > "${OUT_DIR}/diffstat.txt"
git diff "${base_commit}..${OLD_BRANCH}" > "${OUT_DIR}/diff.patch"
git log --reverse --oneline "${base_commit}..${OLD_BRANCH}" > "${OUT_DIR}/commits.txt"

mkdir -p "${OUT_DIR}/old"

changed_paths_cmd=(git diff --name-only -z --diff-filter=ACMR "${base_commit}..${OLD_BRANCH}")

copied_count=0
docs_count=0
while IFS= read -r -d '' path; do
  if [[ "${COPY_ALL}" == "1" ]]; then
    :
  else
    if [[ "${COPY_DOCS}" != "1" ]]; then
      continue
    fi
    if ! is_markdown_path "${path}"; then
      continue
    fi
    docs_count=$((docs_count + 1))
  fi

  dest="${OUT_DIR}/old/${path}"
  mkdir -p "$(dirname "${dest}")"

  if git show "${OLD_BRANCH}:${path}" > "${dest}"; then
    copied_count=$((copied_count + 1))
  else
    echo "[WARN] Failed to copy ${path} from ${OLD_BRANCH} (skipping)."
    rm -f "${dest}"
  fi
done < <("${changed_paths_cmd[@]}")

if [[ "${COPY_ALL}" == "1" ]]; then
  echo "[OK] Copied ${copied_count} changed files into: ${OUT_DIR}/old/"
else
  if [[ "${COPY_DOCS}" == "1" ]]; then
    echo "[OK] Copied ${copied_count}/${docs_count} changed Markdown docs into: ${OUT_DIR}/old/"
  else
    echo "[OK] Bundle created (no docs copied): ${OUT_DIR}"
  fi
fi

echo "[OK] Bundle ready: ${OUT_DIR}"
