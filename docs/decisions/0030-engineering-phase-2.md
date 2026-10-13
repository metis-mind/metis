# 0030. 工程化阶段 2：workflows 恢复、远程 CI 定位、发布形态与评审方案

- Status: accepted
- Date: 2026-10-13

## Context

[ADR-0009](0009-engineering-baseline.md) 落地裁剪时整体删除 `.github/`（Forgejo 迁移时再从模板翻译）；[ADR-0020](0020-open-source-hosting.md) 迁移 GitHub 后登记三个待办：workflows 恢复（直接用模板原版，无需翻译）、远程测试重议（旧前提「自托管算力 → 测试全部本地跑」消失，public repo CI 免费）、release 管线与 crates.io 形态定夺；[ADR-0009](0009-engineering-baseline.md) 遗留①（用户 review 方案）按 0020 的登记与本议题联动。四件同属工程化阶段 2 收尾，一批拍板。

执行前实测的托管侧状态：仓库无任何 ruleset；合并策略三种全开；Actions 默认 token 只读且「允许 Actions 开 PR」开关在 org 与 repo 两级均为关。

## Decision

### 1. workflows 恢复范围

恢复六件 + 移植一件 + 三件模板外围：

- `ci.yml`（模板原版裁剪）：删 dist-drift（无 cargo-dist 配置）、smoke-init（模板自用）、merge_group 触发（直推 main 不用 merge queue）；触发 = push main + PR；quality-gate（prek 全文件只读）+ 测试矩阵两个 job。
- `commits.yml` / `pr-guard.yml`（模板原版）：PR 标题/正文规范检查与 PR diff 秘密扫描。本地钩子管不到网页侧产生的提交；这两个门禁的真实对象 = dependabot PR 与未来外部 PR（外部贡献放开是非单向门，届时零改动复用）。
- `flake-update.yml`（周一）/ `toolchain-update.yml`（周二，in-job 先过 `just ci` 再开 PR）：模板原版，owner guard 落 `metis-mind`。
- `dependabot.yml`（模板原版，cargo + github-actions 双生态周更分组）：ADR-0009 当年删 dependabot 是 Forgejo 无此物；GitHub 下它是 SHA 钉死 action 的保鲜机制，且 dependabot PR 不受 GITHUB_TOKEN 递归护栏限制、能正常触发 CI 全量验证。
- `dependency-audit.yml`（周三定期 `just deny`，Cadmus 件移植为 GitHub 原生缓存）：新披露 advisory / 上游 yank 会随时阻断 `just deny`，定期远程跑让下一次本地跑不是第一个听到的。
- `ISSUE_TEMPLATE/` 三件（bug/feature 双语表单 + 开放问题导 Discussions）：ADR-0020 外部 issue 欢迎的承接面。
- 不恢复：`init.yml`（模板自用）；`release.yml` / `release-plz.yml`（归条款 3）；`pull_request_template.md`（核心仓不收外部代码 PR，无读者）。
- 配套：actionlint 进 flake.nix 与 `just lint`（testing-strategy §5 登记兑现）。

### 2. 远程 CI 定位（testing-strategy §6 第 8 条结案）

远程与本地跑**同一套门禁**（justfile 是唯一逻辑面，CI 只买四样东西：fresh checkout 独立复核、macOS 覆盖、dependabot PR 门禁、未来外部 PR 的门禁基建）。矩阵 = **ubuntu + macOS**；不立 Windows native——支持面维持「Windows 走 WSL2」（AGENTS.md 既有口径），首个真实 Windows native 需求 = 从模板原件恢复 test-windows job 的触发器。

托管侧硬边界（当次已执行）：main 分支 ruleset = 禁 force-push + 禁删分支（NEVER 纪律的托管侧兜底）；合并策略收窄为 squash-only + 合并后删分支（bot PR 标题入史与 commits.yml 对齐）。**不开** require status checks：flake-update / toolchain-update 的 PR 由默认 GITHUB_TOKEN 开出，受 GitHub 递归护栏不触发 CI（前者无 in-job 验证，第一信号 = 合并后的 push CI；后者靠 in-job `just ci`），开了 require checks 会反卡住这类 bot PR 的合并；外部 PR 放开时再开。

### 3. release 形态定夺，管线推迟（testing-strategy §6 第 9 条结案）

形态四条款：

1. 管线 = 模板原版 **release-plz**（Conventional Commits → Release PR 人工闸门，合并即打 tag）+ **cargo-dist**（tag → 跨平台二进制 + GitHub Release）+ crates.io **OIDC trusted publishing**（crates.io 信任 GitHub Actions 身份的免令牌发布，仓库侧零秘密）；tag 形态 `v*`。
2. crates.io 发布面 = **仅二进制 crate**（cargo install 通道）；内部 lib crates 维持 `publish = false` 不背 semver 负担（插件生态是 Luau 文本，Rust crate 消费者只有嵌运行时的 Rust 用户）。
3. 二进制 crate 命名 = **`metis-mind`**（`metis` 被图分区库占用，ADR-0020 已核实 `metis-mind` 两侧可用）；改名随管线回归执行（`publish = false` 下名字纯内部）。
4. 管线回归触发器 = **首个可发布里程碑**（任务 10 实现产出可跑骨架）；届时恢复 `release.yml` / `release-plz.yml` + `dist-workspace.toml` / `release-plz.toml` + crate 改名 + OIDC 配置 + 捡回 `release-review` skill。

**例外执行（用户拍板，2026-10-13）**：为防抢注，立即以 0.0.0 空 crate 占位 `metis-mind`——一次性名字预留，不打 tag、不开 GitHub Release、占位 crate 源不进仓；「NEVER 推 tag / 发版」纪律维持不变（占位不是发布工程回归）。真实发布自 0.1.0 起。

### 4. 用户 review 方案（ADR-0009 遗留① 结案）

常态面维持三件套：议题拍板制（设计逐条用户拍板）+ `self-review` skill（每 commit：作者机械 sweep + 子代理判断 pass）+ HANDOFF 收工记录。模板暂删件 `pr-preflight` 的流程面判断项（机器门禁先行 / 文档协同 / 快照意图 / commit type 与 semver 对应 / ADR 必要性 / ASK 面确认）**吸收进 `self-review` skill**，不立第二个 skill；`release-review` 随条款 3 管线回归原版捡回。轻 PR 评审面不立——触发器 = 外部贡献放开或维护者主动要求。

## Consequences

- 结案：ADR-0009 遗留①（Status 行已注）；testing-strategy §6 第 8/9 条、§5 workflow lint / 轻 PR 评审面两行（正文已注）。
- 登记触发器：Windows native CI（首个真实需求）；require status checks（外部 PR 放开）；release 管线回归（首个可发布里程碑，清单见条款 3 第 4 条）。
- 托管侧执行记录（2026-10-13）：squash-only + 合并后删分支 ✓；main ruleset（禁 force-push + 禁删分支）✓；crates.io `metis-mind` 0.0.0 占位发布 ✓（本 ADR 推送前执行，库源不进仓）。
- 托管侧待办：「允许 Actions 开 PR」是 org 级开关（repo 级设置被其 409 压住），org 与 repo 两级都须维护者网页操作或带 `admin:org` scope 的 token；开启前 flake-update / toolchain-update 的定时与手动运行会在开出 PR 一步失败（403），其余步骤正常。
- marketplace 仓的插件投稿 schema 校验 CI 归任务 9（marketplace 格式设计），不在本 ADR 范围。
- 遗留②（Forgejo spike）已随 ADR-0020 取消；至此 ADR-0009 的全部遗留清空。
