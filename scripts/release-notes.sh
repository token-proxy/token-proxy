#!/usr/bin/env bash
#
# 生成 GitHub Release 的发布说明（release body）。
#
# 内容分四块：
#   1. 容器镜像：镜像名、digest、平台、拉取命令与离线包加载方式；
#   2. 变更内容：`git-cliff` 按 `cliff.toml` 约定生成的**该版本**条目 —— 与 CHANGELOG.md 同源，
#      不另起一套分组规则（tag 与上一个 tag 的区间由脚本自己推断，`--from` 可覆盖）；
#   3. 发布产物：Release 附件（离线镜像包等）的大小与 SHA-256；
#   4. 完整变更：与上一个 tag 的比较链接（首个版本指向 tree 链接）。
#
# 只读 git 历史与传入文件，**不访问网络**，可以在本地随意试跑：
#
#   scripts/release-notes.sh --tag 1.8.2 \
#       --image ghcr.io/your-org/token-proxy --digest sha256:deadbeef \
#       --platforms linux/amd64 --asset dist/token-proxy-1.8.2.tar.gz
#
# 依赖：git；生成变更内容需要 git-cliff（`cargo install git-cliff`，或 `--changelog` 传入现成文件）。
set -euo pipefail

TAG=""
FROM=""
REPO="${GITHUB_REPOSITORY:-}"
IMAGE=""
DIGEST=""
PLATFORMS=""
CHANGELOG_FILE=""
OUT=""
PRINT_RANGE=0
ASSETS=()

die() { printf 'release-notes: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<'EOF'
用法: release-notes.sh --tag <X.Y.Z> [选项]

  --tag <tag>          必需。要发布的 tag，例如 1.8.2（不带 v 前缀）
  --from <tag>         起始版本 tag（不含）；默认取上一个版本 tag（按版本号排序，兼容
                       本仓库 tag 打在 release/* 分支、与 main 分叉的发布模型）
  --repo <owner/repo>  仓库，用于生成提交/比较链接；默认取 $GITHUB_REPOSITORY 或 origin
  --image <ref>        镜像名（不含 tag），例如 ghcr.io/your-org/token-proxy
  --digest <sha256:…>  镜像 digest
  --platforms <列表>   镜像平台，例如 linux/amd64
  --changelog <文件>   直接使用该文件作为「变更内容」（跳过 git-cliff）
  --asset <文件>       发布产物文件，可重复；会写入大小与 SHA-256
  --out <文件>         输出到文件；默认 stdout
  --print-range        只打印推导出的 git 区间（供发布流程生成 CHANGELOG 用），随后退出
  -h, --help           显示本帮助
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --tag)       TAG="${2:?--tag 缺少取值}"; shift 2 ;;
        --from)      FROM="${2:?--from 缺少取值}"; shift 2 ;;
        --repo)      REPO="${2:?--repo 缺少取值}"; shift 2 ;;
        --image)     IMAGE="${2:?--image 缺少取值}"; shift 2 ;;
        --digest)    DIGEST="${2:?--digest 缺少取值}"; shift 2 ;;
        --platforms) PLATFORMS="${2:?--platforms 缺少取值}"; shift 2 ;;
        --changelog) CHANGELOG_FILE="${2:?--changelog 缺少取值}"; shift 2 ;;
        --asset)     ASSETS+=("${2:?--asset 缺少取值}"); shift 2 ;;
        --out)       OUT="${2:?--out 缺少取值}"; shift 2 ;;
        --print-range) PRINT_RANGE=1; shift ;;
        -h|--help)   usage; exit 0 ;;
        *)           usage >&2; die "未知参数：$1" ;;
    esac
done

[ -n "$TAG" ] || { usage >&2; die "缺少 --tag"; }
[[ "$TAG" =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]] || die "tag 必须是 X.Y.Z 形式（不带 v 前缀），实际为：$TAG"
git rev-parse -q --verify "refs/tags/$TAG" >/dev/null || die "本地不存在 tag $TAG"

VERSION="$TAG"

# ─── 起始版本与变更区间 ───────────────────────────────────────────────────
# 本仓库的发布模型是「tag 打在 release/* 分支的版本号提交上，main 只收 cherry-pick」，
# 因此 tag 与 main 的提交链**分叉**，直接按祖先关系取区间会有两个坑：
#   1. 新 minor 的首个版本上 `git describe` 找不到祖先版本 tag → 必须退回「版本号排序」取上一个；
#   2. 上一个版本已发布过的改动在 main 上是另一份 sha（cherry-pick），纯祖先区间会重复计入
#      → 用 `git log --cherry-pick --right-only A...B`（按 patch-id 去重）算出真正属于本次的提交，
#        再取其最老者之前的那个提交作为区间下界。
version_tags() { git tag -l '[0-9]*' | sort -V; }

if [ -z "$FROM" ]; then
    FROM=$(git describe --tags --abbrev=0 --match '[0-9]*' "${TAG}^" 2>/dev/null || true)
fi
if [ -z "$FROM" ]; then
    FROM=$(version_tags | awk -v t="$TAG" '{ if ($0 == t) { print prev; exit } prev = $0 }')
fi
if [ -n "$FROM" ]; then
    git rev-parse -q --verify "refs/tags/$FROM" >/dev/null || die "本地不存在起始 tag $FROM"
fi

# 区间下界：纯祖先区间里「被判为上一个版本已发布过」的最新提交。
# 以它为界，区间恰好覆盖本次新增改动（更早那些只存在于 main 的发布簿记提交由
# cliff.toml 的 commit_parsers 跳过）；没有重复提交（同一发布分支上的补丁版本）时不设下界。
TMPDIR_NOTES=$(mktemp -d)
trap 'rm -rf "$TMPDIR_NOTES"' EXIT

RANGE_BASE=""
if [ -n "$FROM" ]; then
    git log --cherry-pick --right-only --no-merges --format=%H "${FROM}...${TAG}" \
        > "$TMPDIR_NOTES/kept"
    RANGE_BASE=$(git log --no-merges --format=%H "${FROM}..${TAG}" \
        | grep -vxF -f "$TMPDIR_NOTES/kept" | head -1 || true)
fi
RANGE_DESC="${FROM:-<首个版本>}..${TAG}"
[ -n "$RANGE_BASE" ] && RANGE_DESC="${RANGE_BASE}..${TAG}"

# --print-range：把推导出的区间交给调用方（发布流程用它生成 CHANGELOG，保持同源）
# 有下界时输出 <下界>..<tag>，否则输出 <起始版本>..<tag>；首个版本无区间可输出。
if [ "$PRINT_RANGE" = "1" ]; then
    if [ -n "$RANGE_BASE" ]; then
        printf '%s\n' "$RANGE_BASE..$TAG"
    elif [ -n "$FROM" ]; then
        printf '%s\n' "$FROM..$TAG"
    fi
    exit 0
fi

if [ -z "$REPO" ]; then
    origin=$(git remote get-url origin 2>/dev/null || true)
    case "$origin" in
        *github.com[:/]*) REPO="${origin#*github.com[:/]}"; REPO="${REPO%.git}" ;;
        *) REPO="" ;;
    esac
fi
[ -n "$REPO" ] || die "无法确定仓库名，请显式传 --repo owner/repo"
REPO_URL="https://github.com/$REPO"

# ─── 变更内容：git-cliff（与 CHANGELOG.md 同源）───────────────────────────
# 首个版本（没有更早的版本 tag）时不传区间，由 git-cliff 把全部历史归入该 tag
# （`..TAG` 会被当成 `HEAD..TAG`，在本场景下为空）。仓库若存在非版本 tag（如 nightly），
# 首个版本请显式传 --from。
cliff_notes() {
    if [ -n "$RANGE_BASE" ]; then
        git-cliff "$RANGE_BASE..$TAG" --tag "$TAG" --strip header
    elif [ -n "$FROM" ]; then
        git-cliff "$FROM..$TAG" --tag "$TAG" --strip header
    else
        git-cliff --tag "$TAG" --strip header
    fi
}

if [ -n "$CHANGELOG_FILE" ]; then
    [ -s "$CHANGELOG_FILE" ] || die "--changelog 指定的文件为空：$CHANGELOG_FILE"
    cp "$CHANGELOG_FILE" "$TMPDIR_NOTES/changelog.md"
else
    command -v git-cliff >/dev/null 2>&1 || die "未找到 git-cliff（cargo install git-cliff，或用 --changelog 传入现成文件）"
    printf 'release-notes: 变更区间 %s（起始版本 %s）\n' "$RANGE_DESC" "${FROM:-无}" >&2
    # git-cliff 的 INFO/WARN 走 stderr：正常路径不打扰输出，失败时才回显
    if ! cliff_notes > "$TMPDIR_NOTES/changelog.md" 2> "$TMPDIR_NOTES/cliff.err"; then
        cat "$TMPDIR_NOTES/cliff.err" >&2
        die "git-cliff 生成变更内容失败（tag $TAG，区间 $RANGE_DESC）"
    fi
fi

emit() { printf '%s\n' "$*"; }

# ─── 输出 ────────────────────────────────────────────────────────────────
{
    if [ -n "$IMAGE" ]; then
        emit '## 📦 容器镜像'
        emit
        emit '| 项目 | 值 |'
        emit '|---|---|'
        emit "| 镜像 | \`$IMAGE:$VERSION\` |"
        if [ -n "$DIGEST" ]; then emit "| Digest | \`$DIGEST\` |"; fi
        if [ -n "$PLATFORMS" ]; then emit "| 平台 | \`$PLATFORMS\` |"; fi
        emit
        emit '```bash'
        emit "docker pull $IMAGE:$VERSION"
        emit '```'
        emit
        emit "> 也可使用下方附件中的离线镜像包：\`docker load -i <文件名>\`，加载后镜像名为 \`$IMAGE:$VERSION\`。"
        emit '> 容器启动时自动执行数据库迁移，升级前请先备份数据库。'
        emit
    fi

    emit '## 📝 变更内容'
    emit
    if [ -s "$TMPDIR_NOTES/changelog.md" ]; then
        cat "$TMPDIR_NOTES/changelog.md"
        emit
    else
        emit '_本次 tag 与上一个 tag 之间没有独立提交（例如仅合并提交）。_'
        emit
    fi

    if [ "${#ASSETS[@]}" -gt 0 ]; then
        emit '## 📎 发布产物'
        emit
        emit '| 文件 | 大小 | SHA-256 |'
        emit '|---|---|---|'
        for asset in "${ASSETS[@]}"; do
            [ -f "$asset" ] || die "附件不存在：$asset"
            name=$(basename "$asset")
            bytes=$(wc -c < "$asset")
            if command -v numfmt >/dev/null 2>&1; then
                size=$(numfmt --to=iec-i --suffix=B "$bytes")
            else
                size="${bytes}B"
            fi
            hash=$(sha256sum "$asset" | cut -d' ' -f1)
            emit "| \`$name\` | $size | \`$hash\` |"
        done
        emit
        emit "校验和汇总见附件 \`sha256sums.txt\`。"
        emit
    fi

    if [ -n "$FROM" ]; then
        emit "**完整变更**：[\`$FROM...$TAG\`]($REPO_URL/compare/$FROM...$TAG)"
    else
        emit "**本版本提交**：[\`$TAG\`]($REPO_URL/tree/$TAG)（首个版本，无上一个 tag 可比较）"
    fi
} > "${OUT:-/dev/stdout}"
