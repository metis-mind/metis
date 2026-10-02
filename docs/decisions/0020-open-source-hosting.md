# 0020. 开源托管：GitHub org metis-mind + 双 license + 外部贡献政策

- Status: accepted
- Date: 2026-10-02

## Context

[ADR-0009](0009-engineering-baseline.md) 的两个前提不再成立。其一，"自托管"当时并非核心价值，而是默认假设：没有既有自托管设施，Forgejo 实例与 runner 的搭建、备份、升级是持续的纯成本，release 工具链与 action 可达性两笔 spike 成本也尚未支出。其二，"无外部协作"被推翻：项目决定**开源做生态**——[ADR-0019](0019-crate-layout.md) 三作者语境中的第三方插件作者必须能看到平台才能写插件，托管地的发现性成为真实需求。

命名核查（2026-10-02；GitHub 侧逐一查验，crates.io 侧查验相关名）：GitHub 上 `metis` / `metis-ai` / `metis-agent` / `metis-project` / `metis-platform` 等均已占用。`metis-rs` GitHub 侧可用，但 crates.io 同名已被占（恰为纯 Rust 的 METIS 系图分区库，正是被否定的语义域），且 "-rs" 本是"X 的 Rust 实现/绑定"的语义惯例，与 Rust+Luau 混合栈应用平台的定位不符——主动放弃。最终选定 `metis-mind`：GitHub 与 crates.io 两侧均可用，语义贴合（Metis 为希腊智慧女神，mind 贴合 agent 定位）。crates.io 的 `metis` 是活跃维护的图分区库绑定。

被淘汰的选项：维持自托管 Forgejo（前提已消失，成本无对价）；Codeberg（托管版 Forgejo，零运维但 CI 配额紧、生态位弱）；核心仓现阶段接受外部代码 PR（评审负担全落在唯一维护者身上，且本仓协作协议为 1+N agent 开发设计，外部贡献者难以实际遵守）。

## Decision

局部 supersede [ADR-0009](0009-engineering-baseline.md) 的三处条款（该 ADR 的 Status 行已注记）：

- **托管条款**（"自托管 Forgejo；workflows `.github/` → `.forgejo/`"）→ **GitHub org `metis-mind`，双仓格局**：`metis-mind/metis`（核心）与 `metis-mind/marketplace`（生态贡献面，先建空壳占位）。workflows 恢复时直接使用模板原版 `.github/`，无需翻译。
- **发布条款**（"私有项目不发布"）→ **即日起 public**；license 为模板自带的 **MIT OR Apache-2.0**（GPL 系劝退商用插件作者，淘汰）。crates.io opt-in 随之激活；`metis` crate 名被占，发布形态（二进制 crate 改名 / cargo-dist 分发）随 release 流水线议题定夺。
- **PR 条款**（"无 PR 流程"）→ 内部**直推 main 不变**；**外部：核心仓现阶段不接受代码 PR**（公开措辞："code contributions temporarily closed while the core architecture lands"），issue 与 Discussions 欢迎。生态贡献面在 marketplace 仓：插件投稿以 PR 进入，审核走机器可判定的 schema 校验 CI（元规则见 `../design/testing-strategy.md` §1）。核心仓放开与否待核心稳定后重议——非单向门。

同时确认与登记：

- **语言政策维持 [ADR-0012](0012-docs-governance.md) 分界不变**：公开面（README / CONTRIBUTING / rustdoc / 未来的插件作者文档）英文；`docs/` 中文正本不变。开源不改变分界，只让分界的后果显现。
- **marketplace 格式设计排入设计链**，跟在 Luau ABI 之后：manifest 的 SSOT 在核心仓（ABI 设计产物），marketplace 消费它；schema 版本化与信任模型随该议题展开。

## Consequences

- "测试全部本地跑"条款的前提（自托管算力成本）消失：GitHub public repo CI 免费，远程跑测试从浪费变为外部 PR 的免费门禁——重议登记入 `../design/testing-strategy.md` §6，与 workflows 恢复同议题
- release-plz / cargo-dist 的 Forgejo spike（ADR-0009 遗留②）取消——GitHub 下两者开箱即用，release 流水线议题随之简化
- ADR-0009 遗留①（用户 review 方案）与外部贡献政策联动，讨论时一并处理
- 公开性核查：git 历史经 secret 形状扫描无敏感信息（8 commits 均为文档与 crate 骨架）；commit author 身份随历史永久公开，是本变更唯一真正的单向门——author email 是否重写为 GitHub noreply 由维护者在首次 push 前拍板（本 ADR 不预决）
