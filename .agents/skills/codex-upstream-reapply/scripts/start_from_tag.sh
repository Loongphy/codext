#!/usr/bin/env bash
set -euo pipefail

print_usage() {
  cat <<'EOF'
start_from_tag.sh

Fetch tags, let the user choose a tag, generate a re-implementation bundle from OLD_BRANCH,
then create NEW_BRANCH from the selected tag.

Usage:
  start_from_tag.sh [options]

Options:
  --remote <remote>       Remote to fetch tags from (default: upstream)
  --tag <tag>             Selected tag (required; if missing, list tags and exit)
  --old-branch <name>     Old customization branch (default: current branch)
  --new-branch <name>     New branch to create from tag (default: feat/<tag-name>)
  --old-base-tag <tag>    Explicit base tag for OLD_BRANCH (override base inference)
  --out <dir>             Bundle output directory (optional)
  --copy-all              Copy ALL changed files into bundle/old/
  --no-copy-docs          Do not copy changed Markdown docs into bundle/old/
  --no-fetch              Do not run git fetch (default: fetch tags best-effort)
  -h, --help              Show help
EOF
}

die() {
  echo "[ERROR] $*" >&2
  exit 1
}

timestamp_utc() {
  date -u +"%Y%m%dT%H%M%SZ"
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

list_tags() {
  git for-each-ref --sort=-creatordate \
    --format='%(creatordate:iso8601) %(refname:short) %(objectname:short)' refs/tags
}

copy_file_from_old_branch() {
  local old_branch="$1"
  local path="$2"

  if git cat-file -e "${old_branch}:${path}" 2>/dev/null; then
    git show "${old_branch}:${path}" > "${path}"
    git add "${path}"
    echo "[INFO] Copied ${path} from ${old_branch}"
  else
    echo "[WARN] ${path} not found in ${old_branch}; skipping."
  fi
}

copy_path_from_old_branch() {
  local old_branch="$1"
  local path="$2"

  if git cat-file -e "${old_branch}:${path}" 2>/dev/null; then
    git checkout "${old_branch}" -- "${path}"
    echo "[INFO] Copied ${path} from ${old_branch}"
  else
    echo "[WARN] ${path} not found in ${old_branch}; skipping."
  fi
}

REMOTE="upstream"
TAG=""
OLD_BRANCH=""
NEW_BRANCH=""
OLD_BASE_TAG=""
OUT_DIR=""
COPY_ALL=0
COPY_DOCS=1
NO_FETCH=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --remote)
      REMOTE="${2:-}"
      shift 2
      ;;
    --tag)
      TAG="${2:-}"
      shift 2
      ;;
    --old-branch)
      OLD_BRANCH="${2:-}"
      shift 2
      ;;
    --new-branch)
      NEW_BRANCH="${2:-}"
      shift 2
      ;;
    --old-base-tag)
      OLD_BASE_TAG="${2:-}"
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

if [[ "${NO_FETCH}" != "1" ]]; then
  echo "[INFO] Fetching tags from ${REMOTE} (best-effort)..."
  if ! git fetch "${REMOTE}" --tags --prune; then
    echo "[WARN] git fetch failed; continuing with local refs."
  fi
fi

if [[ -z "${TAG}" ]]; then
  echo "[INFO] Available tags (newest first):"
  list_tags | head -n 50
  echo
  echo "Re-run with: --tag <tag>"
  exit 0
fi

git rev-parse --verify "${TAG}^{commit}" >/dev/null 2>&1 || die "Tag not found: ${TAG}"

if [[ -z "${OLD_BRANCH}" ]]; then
  OLD_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
fi

[[ -n "${OLD_BRANCH}" ]] || die "--old-branch resolved to empty"
[[ "${OLD_BRANCH}" != "HEAD" ]] || die "Detached HEAD; pass --old-branch <name>."

if [[ -z "${NEW_BRANCH}" ]]; then
  tag_name="${TAG#refs/tags/}"
  NEW_BRANCH="feat/${tag_name}"
fi

if [[ "${NEW_BRANCH}" == "${OLD_BRANCH}" ]]; then
  die "--new-branch must differ from --old-branch"
fi

if git show-ref --verify --quiet "refs/heads/${NEW_BRANCH}"; then
  die "Branch already exists: ${NEW_BRANCH}"
fi

if [[ "$(git rev-parse --abbrev-ref HEAD)" == "${OLD_BRANCH}" ]]; then
  if [[ -n "$(git status --porcelain)" ]]; then
    die "Working tree is dirty on ${OLD_BRANCH}. Commit or stash first."
  fi
fi

if [[ -z "${OUT_DIR}" ]]; then
  repo_root="$(git rev-parse --show-toplevel)"
  repo_name="$(basename "${repo_root}")"
  ts="$(timestamp_utc)"
  tag_dir="${TAG//\//-}"
  OUT_DIR="/tmp/codex-upstream-reapply/${repo_name}/${OLD_BRANCH}/${tag_dir}/${ts}"
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bundle_script="${script_dir}/prepare_reimplementation_bundle.sh"

bundle_args=(--old-branch "${OLD_BRANCH}" --base-ref "${TAG}" --remote "${REMOTE}" --out "${OUT_DIR}")
if [[ -n "${OLD_BASE_TAG}" ]]; then
  bundle_args+=(--old-base-tag "${OLD_BASE_TAG}")
fi
if [[ "${COPY_ALL}" == "1" ]]; then
  bundle_args+=(--copy-all)
fi
if [[ "${COPY_DOCS}" != "1" ]]; then
  bundle_args+=(--no-copy-docs)
fi
if [[ "${NO_FETCH}" == "1" ]]; then
  bundle_args+=(--no-fetch)
fi

echo "[INFO] Creating re-implementation bundle..."
"${bundle_script}" "${bundle_args[@]}"

echo "[INFO] Creating new branch ${NEW_BRANCH} from tag ${TAG}..."
git switch -c "${NEW_BRANCH}" "${TAG}"

echo "[INFO] Copying README.md, CHANGED.md, and .agents/skills from ${OLD_BRANCH} (no modifications)..."
copy_file_from_old_branch "${OLD_BRANCH}" "README.md"
copy_file_from_old_branch "${OLD_BRANCH}" "CHANGED.md"
copy_path_from_old_branch "${OLD_BRANCH}" ".agents/skills"
if ! git diff --cached --quiet; then
  if git commit -m "docs: copy README.md, CHANGED.md, and .agents/skills from ${OLD_BRANCH}"; then
    echo "[OK] Committed README.md, CHANGED.md, and .agents/skills copy"
  else
    echo "[WARN] Unable to commit copied docs/skills (git user.name/user.email?)."
    echo "[WARN] Commit manually with: git commit -m \"docs: copy README.md, CHANGED.md, and .agents/skills from ${OLD_BRANCH}\""
  fi
fi

echo "[OK] New branch created: ${NEW_BRANCH}"
echo "[OK] Bundle: ${OUT_DIR}"
echo
echo "Next:"
echo "  - Read intent docs in ${OUT_DIR}/old/"
echo "  - Use: git show ${OLD_BRANCH}:path/to/file"
echo "  - Re-implement changes on ${NEW_BRANCH}"
