# 0009. 工程化基线：模板继承 + Forgejo + 1+N

- Status: accepted
- Date: 2026-09-29

## Context

自托管 + 无外部协作 → 本地/远程双跑测试是浪费；agent 是主要开发者 → 门禁必须机器可判定（元规则详见 `../design/testing-strategy.md` §1）。

被淘汰的选项：GitHub 托管 + 完整远程 CI（与自托管前提冲突）；PR 评审流（solo；评审方案定后可能捡回轻 PR 作评审面）。

## Decision

- 以 `duskgrow/rust-template` 为工程化基线，继承：Nix flake + direnv（`rust-toolchain.toml` SSOT）、just 唯一任务入口、workspace 集中 lint、nextest + doctest + insta、cargo-deny、xtask commit 规范（改良版 Conventional Commits）、AGENTS.md + `.agents/skills`、MADR、dprint / prek hooks
- 托管：**自托管 Forgejo**；workflows `.github/` → `.forgejo/`（语法大体兼容，action 可达性需处理）
- 开发模式 **1+N**：用户 1 人 + agent N 个，**agent 是主要开发者**；AGENTS.md / skills 即协作协议核心
- **无 PR 流程**：本地门禁 + 直推远程（用户 review 方案日后单开话题，见遗留①）
- **测试全部本地跑**；远程 CI 只跑固定/定期例程：tag/release、依赖检查（deny/audit/outdated）、toolchain/flake 更新
- 发布流水线：release-plz / cargo-dist 均 GitHub-centric，Forgejo 可行性需 spike；退路 = `tag → cargo build 多平台 → Forgejo Release API 上传`
- 删除：crates.io OIDC Trusted Publishing（私有项目不发布，留 opt-in 备注）、dependabot（由定期依赖检查替代）

## Consequences

遗留：① 用户 review 方案（日后单开话题）；② release-plz / cargo-dist 的 Forgejo spike（首个可发布里程碑前）；③ ~~模板 MADR 编号制与单文件制的对齐~~ ✅ 2026-09-30 随工程化落地完成（本文件群即产物，治理规则见 [ADR-0012](0012-docs-governance.md)）。

2026-09-30 落地裁剪记录：`.github/` 整体删除（Forgejo 迁移时从模板原版参照翻译，不预写无法验证的 YAML）；release-plz / cargo-dist / CHANGELOG 随 spike 一并回归；`pr-preflight` / `release-review` 两个 skill 随无 PR 流暂删（评审方案定后从模板捡回）。
