---
name: release
description: 执行完整的发布流程。当用户输入 /release <version> [<description>] 命令时使用。
---

# 发布流程技能

本技能处理 `/release <version> [<description>]` 命令，执行完整的 Git 发布流程。

使用此技能时必须同时加载 `chm:write-chinese`、`chm:write-document`、`chm:use-terminal-command` 技能。

## 前置技能

- `chm:use-terminal-command` — 执行所有终端命令时遵循其规范
- `chm:write-document` — 编写和更新 CHANGELOG.md 时遵循其文档规范
- `chm:write-chinese` — 编写中文内容时遵循其书写规范

## 命令语法

```
/release <version> [<description>]
```

- `<version>` — 发布版本号，无 `v` 前缀（如 `0.1.0`、`0.2.0-rc.1`）
- `<description>` — 可选的发布说明，不提供时使用 CHANGELOG 内容

## 关键决策

- Git tag 格式：无 `v` 前缀（如 `0.1.0`、`0.2.0-rc.1`）
- main 分支版本号为占位 `0.0.0`，版本号只在 release 分支上变更
- CHANGELOG 按发布日期倒序排列
- 使用 rebase 策略，无 merge commit
- 发布流程在 release 分支上拆分为两个独立提交：
  - 提交 A：仅 CHANGELOG.md
  - 提交 B：仅版本号变更（Cargo.toml + package.json）
  - tag 打在提交 B 上
- cherry-pick 提交 A 回 main（不带版本号）
- 使用 git-cliff 生成 CHANGELOG
- 使用 gh CLI 创建 GitHub Release
- RC 版本：如果 release/<major>.<minor> 分支已存在，则 checkout 已有分支而非创建新分支

## 项目路径

```
/home/viktor/dev/projects/github/token-proxy/token-proxy
```

## 执行流程

### Phase 0：参数校验

```bash
# 验证版本号格式（要求 x.y.z 或 x.y.z-<pre> 格式）
echo "<version>" | grep -E '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'
# 提取 major.minor（如 0.1.0 → 0.1, 0.2.0-rc.1 → 0.2）
MAJOR_MINOR=$(echo "<version>" | grep -oE '^[0-9]+\.[0-9]+')
```

解析 `<description>` 参数（可选），如未提供则后续使用 CHANGELOG 内容。

### Phase 1：前置检查

所有命令在项目根目录执行。先切换到项目目录：

```bash
cd /home/viktor/dev/projects/github/token-proxy/token-proxy
```

**检查步骤（按顺序执行，任一步失败则中止）：**

1. **检查当前分支是否为 main**

   ```bash
   git branch --show-current
   ```

   必须输出 `main`，否则中止并提示：必须在 main 分支上执行发布操作

2. **检查工作区是否干净**

   ```bash
   git status --porcelain
   ```

   必须无输出，否则中止并提示：工作区有未提交的更改，请先提交或暂存

3. **检查 main 有无未推送的提交**

   ```bash
   git log origin/main..HEAD
   ```

   必须无输出，否则中止并提示：main 分支有未推送的提交，请先推送

4. **检查 version tag 是否已存在**

   ```bash
   git tag -l "<version>"
   ```

   必须无输出，否则中止并提示：版本号 `<version>` 对应的 tag 已存在

5. **检查 git-cliff 是否已安装**

   ```bash
   command -v git-cliff || which git-cliff || (echo "error: git-cliff 未安装" && exit 1)
   ```

   如未安装则中止，提示用户安装：`cargo install git-cliff`

6. **检查 gh CLI 是否已登录**

   ```bash
   gh auth status
   ```

   如未登录则中止，提示用户登录：`gh auth login`

7. **检查 CHANGELOG.md 是否存在，不存在则创建空文件**
   ```bash
   test -f CHANGELOG.md || echo "" > CHANGELOG.md
   ```

### Phase 2：创建或复用 release 分支

```bash
cd /home/viktor/dev/projects/github/token-proxy/token-proxy

# 提取 major.minor
MAJOR_MINOR=$(echo "<version>" | grep -oE '^[0-9]+\.[0-9]+')

# 检查 release/<major>.<minor> 分支是否已存在（本地或远程）
EXISTING_BRANCH=""
if git branch --list "release/${MAJOR_MINOR}" | grep -q "release/${MAJOR_MINOR}"; then
  EXISTING_BRANCH="local"
elif git branch -r --list "origin/release/${MAJOR_MINOR}" | grep -q "origin/release/${MAJOR_MINOR}"; then
  EXISTING_BRANCH="remote"
fi

if [ -n "$EXISTING_BRANCH" ]; then
  # 分支已存在 —— RC 场景，复用分支
  # 提示用户：release/<major>.<minor> 分支已存在，将复用此分支
  # 注意：这是一个 RC 发布，分支上已有之前的工作
  git checkout "release/${MAJOR_MINOR}"
  # 如果只存在于远程，还需创建本地跟踪分支
  if [ "$EXISTING_BRANCH" = "remote" ]; then
    git checkout -b "release/${MAJOR_MINOR}" "origin/release/${MAJOR_MINOR}"
  fi
else
  # 分支不存在 —— 正常发布，从 main 创建新分支
  git checkout -b "release/${MAJOR_MINOR}" main
fi

# 确保版本号一致：rebase main 的最新提交
# 注意：仅在从 main 创建分支时需要；复用分支时不需要
```

### Phase 3：生成 CHANGELOG（提交 A）

```bash
cd /home/viktor/dev/projects/github/token-proxy/token-proxy

# 使用 git-cliff 生成 CHANGELOG（prepend 模式，将新内容插入文件头部）
git cliff --tag "<version>" --prepend CHANGELOG.md

# 创建仅包含 CHANGELOG.md 的提交
git add CHANGELOG.md
git commit -m "chore(release): add CHANGELOG for <version>"

# 记录提交 A 的 hash 以便后续 cherry-pick
COMMIT_A_HASH=$(git rev-parse HEAD)
```

**注意：** 如果初次使用 git-cliff，可能需要确保项目根目录存在 `cliff.toml` 配置文件。如不存在，git-cliff 会使用默认配置。

### Phase 4：Bump 版本号（提交 B） + Tag

```bash
cd /home/viktor/dev/projects/github/token-proxy/token-proxy

# 1. 修改 Cargo.toml 的 version 字段
# 将 version = "0.0.0" 替换为 version = "<version>"
# 使用 sed 确保只修改 [package] 段的 version 字段
sed -i 's/^version = ".*"/version = "<version>"/' Cargo.toml

# 2. 修改 package.json 的 version 字段
sed -i 's/"version": ".*"/"version": "<version>"/' package.json

# 3. 提交版本号变更
git add Cargo.toml package.json
git commit -m "chore(release): bump version to <version>"

# 4. 打 tag（无 v 前缀）
git tag "<version>"
```

### Phase 5：推送分支和 tag

```bash
cd /home/viktor/dev/projects/github/token-proxy/token-proxy

# 推送 release 分支
git push origin "release/${MAJOR_MINOR}"

# 推送 tag
git push origin "<version>"
```

### Phase 6：创建 GitHub Release

```bash
cd /home/viktor/dev/projects/github/token-proxy/token-proxy

# 获取 CHANGELOG 中当前版本的发布说明
# 根据 git-cliff 生成的格式提取版本对应的正文内容
if [ -z "$RELEASE_DESCRIPTION" ]; then
  # 从 CHANGELOG.md 提取当前版本的发布说明
  # 假设 git-cliff 标准格式：## [version] - date
  RELEASE_DESCRIPTION=$(awk "/^## \\[${version}\\]/,/^## \\[/{if(!/^## \\[${version}\\]/ && !/^## \\[/) print}" CHANGELOG.md)
fi

# 创建 GitHub Release
gh release create "<version>" \
  --title "<version>" \
  --notes "详细内容请查看 [CHANGELOG.md](https://github.com/OWNER/REPO/blob/main/CHANGELOG.md)" \
  --target "release/${MAJOR_MINOR}"
```

**注意：**

- 发布说明指向 CHANGELOG.md 链接
- `OWNER/REPO` 需要根据实际项目替换，可通过 `gh repo view --json nameWithOwner --jq .nameWithOwner` 获取
- 如果提供了 `[<description>]` 参数，可将描述附加到 release notes 中

### Phase 7：Cherry-pick 提交 A 回 main

```bash
cd /home/viktor/dev/projects/github/token-proxy/token-proxy

# 切回 main 分支
git checkout main

# 确保 main 是最新的
git pull origin main

# Cherry-pick 提交 A（仅 CHANGELOG.md 变更）
git cherry-pick "$COMMIT_A_HASH"
```

### Phase 8：推送 main

```bash
cd /home/viktor/dev/projects/github/token-proxy/token-proxy

# 推送 main（cherry-pick 后的 CHANGELOG 更新）
git push origin main
```

## 错误处理

### 版本号已存在 tag

**症状：** `git tag -l "<version>"` 输出不为空

**处理：** 提示用户 tag 已存在，询问是否要：

1. 使用不同的版本号重新执行命令
2. 删除已有 tag 后重试（仅当确定不需要时）

**中止发布流程。**

### main 有未推送提交

**症状：** `git log origin/main..HEAD` 有输出

**处理：** 提示用户 main 分支有本地提交尚未推送到远端。询问用户是否：

1. 先推送这些提交（`git push origin main`）后重新执行命令
2. 暂不处理，先解决此问题

**中止发布流程。**

### git-cliff 未安装

**症状：** `command -v git-cliff` 失败

**处理：** 提示用户安装 git-cliff：

```bash
cargo install git-cliff
```

**安装完成后重新执行命令。**

### gh CLI 未登录

**症状：** `gh auth status` 失败

**处理：** 提示用户先登录 GitHub CLI：

```bash
gh auth login
```

**登录完成后重新执行命令。**

### cherry-pick 冲突

**症状：** `git cherry-pick` 报告冲突

**处理：** 冲突通常出现在 CHANGELOG.md 中，因为 main 分支可能已有其他改动。按以下步骤解决：

1. 检查冲突文件：

   ```bash
   git status
   ```

2. 如果只有 CHANGELOG.md 冲突（最常见），手动打开文件解决冲突。

3. **冲突解决策略：按发布日期倒序排列。**
   - CHANGELOG 的格式是最近的版本在最顶部
   - cherry-pick 过来的内容应该插入到文件头部（最新的位置）
   - 如果 main 上已有其他版本的 CHANGELOG 条目，需要确保当前版本的条目放在正确的位置（按日期倒序，最新的在上面）

4. 标记冲突已解决：

   ```bash
   git add CHANGELOG.md
   git cherry-pick --continue
   ```

   保持原始提交消息不变。

5. 如果有其他文件的冲突，逐文件解决后标记完成。

### release 分支已存在（RC 场景）

**症状：** `git branch --list "release/<major>.<minor>"` 或 `git branch -r --list "origin/release/<major>.<minor>"` 不为空

**处理：** 这是正常的 RC 发布场景。分支已存在意味着之前的版本（如 `0.1.0` 的 release/0.1 分支）已被创建。

1. checkout 已有分支而非创建新分支
2. 确保分支已 rebase 到 main 的最新提交（注意：如果多人协作，rebase 已推送的分支需要 force push）
3. 继续执行 Phase 3-8

**注意：** release 分支从 main 创建后不再同步 main 的变更。RC 版本的发布总是在已有分支上叠加新的提交。

### 工作区不干净

**症状：** `git status --porcelain` 有输出

**处理：** 提示用户工作区有未提交或未暂存的变更。建议：

1. `git stash` 暂存变更
2. 或 `git add` 并 `git commit` 提交变更
3. 完成后重新执行发布命令

**中止发布流程。**

### 不在 main 分支

**症状：** `git branch --show-current` 输出不是 `main`

**处理：** 提示用户必须在 main 分支上执行发布命令。建议先切换到 main：

```bash
git checkout main
```

**切换后重新执行命令。**

## RC 版本特别说明

RC（Release Candidate）版本有着与正式版本不同的流程：

1. **分支复用逻辑：**
   - `0.1.0-rc.1` → major.minor 为 `0.1`，检测 `release/0.1` 是否已存在
   - 如果 `release/0.1` 已存在（从之前 `0.1.0` 的发布创建），则 checkout 该分支
   - 在已有分支上叠加新的 CHANGELOG 和版本号变更

2. **git-cliff 配置：**
   - 确保 git-cliff 配置中包含 RC 版本的标签匹配规则
   - 默认配置通常能正确处理 `0.2.0-rc.1` 格式的 tag

3. **GitHub Release 标记：**
   - RC 版本的 GitHub Release 应标记为 `--prerelease`

   ```bash
   gh release create "<version>" \
     --title "<version>" \
     --notes "..." \
     --prerelease \
     --target "release/${MAJOR_MINOR}"
   ```

4. **CHANGELOG 内容：**
   - RC 版本的 CHANGELOG 应清晰标注为 Pre-release
   - git-cliff 默认生成的格式已包含版本号，无需额外处理

## 完整脚本示例

以下为完整执行流程的参考脚本（将 `<version>` 替换为实际版本号）：

```bash
# 用户输入
VERSION="<version>"
DESCRIPTION="<description>"

cd /home/viktor/dev/projects/github/token-proxy/token-proxy

# Phase 1：前置检查
echo "=== Phase 1: 前置检查 ==="
[ "$(git branch --show-current)" = "main" ] || { echo "错误：不在 main 分支"; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "错误：工作区不干净"; exit 1; }
[ -z "$(git log origin/main..HEAD)" ] || { echo "错误：main 有未推送提交"; exit 1; }
git tag -l "$VERSION" | grep -q . && { echo "错误：tag $VERSION 已存在"; exit 1; }
command -v git-cliff || { echo "错误：git-cliff 未安装"; exit 1; }
gh auth status || { echo "错误：gh 未登录"; exit 1; }
test -f CHANGELOG.md || echo "" > CHANGELOG.md

# Phase 2：创建或复用 release 分支
echo "=== Phase 2: 创建或复用 release 分支 ==="
MAJOR_MINOR=$(echo "$VERSION" | grep -oE '^[0-9]+\.[0-9]+')
if git branch --list "release/${MAJOR_MINOR}" | grep -q "release/${MAJOR_MINOR}" || \
   git branch -r --list "origin/release/${MAJOR_MINOR}" | grep -q "origin/release/${MAJOR_MINOR}"; then
  echo "分支 release/${MAJOR_MINOR} 已存在，复用"
  git checkout "release/${MAJOR_MINOR}"
else
  echo "创建新分支 release/${MAJOR_MINOR}"
  git checkout -b "release/${MAJOR_MINOR}" main
fi

# Phase 3：CHANGELOG 提交（提交 A）
echo "=== Phase 3: 生成 CHANGELOG ==="
git cliff --tag "$VERSION" --prepend CHANGELOG.md
git add CHANGELOG.md
git commit -m "chore(release): add CHANGELOG for $VERSION"
COMMIT_A_HASH=$(git rev-parse HEAD)
echo "提交 A hash: $COMMIT_A_HASH"

# Phase 4：版本号变更（提交 B）+ tag
echo "=== Phase 4: Bump 版本号 + Tag ==="
sed -i 's/^version = ".*"/version = "'"$VERSION"'"/' Cargo.toml
sed -i 's/"version": ".*"/"version": "'"$VERSION"'"/' package.json
git add Cargo.toml package.json
git commit -m "chore(release): bump version to $VERSION"
git tag "$VERSION"

# Phase 5：推送
echo "=== Phase 5: 推送分支和 tag ==="
git push origin "release/${MAJOR_MINOR}"
git push origin "$VERSION"

# Phase 6：GitHub Release
echo "=== Phase 6: 创建 GitHub Release ==="
IS_RC=false
echo "$VERSION" | grep -q "rc" && IS_RC=true
PRERELEASE_FLAG=""
$IS_RC && PRERELEASE_FLAG="--prerelease"

gh release create "$VERSION" \
  --title "$VERSION" \
  --notes "详细内容请查看 [CHANGELOG.md](https://github.com/OWNER/REPO/blob/main/CHANGELOG.md)" \
  $PRERELEASE_FLAG \
  --target "release/${MAJOR_MINOR}"

# Phase 7：Cherry-pick 回 main
echo "=== Phase 7: Cherry-pick 回 main ==="
git checkout main
git pull origin main
git cherry-pick "$COMMIT_A_HASH"

# Phase 8：推送 main
echo "=== Phase 8: 推送 main ==="
git push origin main

echo "=== 发布完成: $VERSION ==="
```

## 注意事项

- 所有命令必须使用 `cd /home/viktor/dev/projects/github/token-proxy/token-proxy && <command>` 格式，遵循终端命令使用规范
- 版本号不要包含 `v` 前缀
- tag 打在版本号变更提交（提交 B）上，而不是 CHANGELOG 提交（提交 A）上
- CHANGELOG 按发布日期倒序排列，最新版本在最顶部
- Cherry-pick 只带回 CHANGELOG 变更，不带版本号变更
- 创建 GitHub Release 时，`OWNER/REPO` 应通过 `gh repo view --json nameWithOwner --jq .nameWithOwner` 动态获取
- 发布流程执行完毕后，建议验证：
  1. 确认 GitHub Release 已创建成功
  2. 确认 main 分支的 CHANGELOG.md 已更新
  3. 确认 release 分支和 tag 已推送
