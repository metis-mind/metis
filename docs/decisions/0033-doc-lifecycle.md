# 0033. 文档生命周期与防膨胀：原地归档制 + freshness + 机器信号 + 巡检与压缩仪式

- Status: accepted
- Date: 2026-10-14

## Context

任务 12（文档防膨胀机制），立项原话 = 火-2（2026-10-14）：「我们的整个结构，落到文档的地方，不管是否是用户看的地方，我们都需要有一个机制来防止其膨胀。目前HANDOFF的膨胀就很严重，以及docs里面的文档的膨胀也很严重，里面有一些是重复逻辑的出现，有一些是没有记录价值信息的堆叠，以及一些我没有想到的问题。这里是文档维护的责任，也是我们在架构设计之初就应该考虑的问题，这应该也是调研的一部分」。调研输入 = [review-economics-research](../research/review-economics-research.md) RQ8 章与映射建议 D7/D8（缺口 5：入口闸门是行业标配，存量生命周期是行业默认缺口；落地顺序 = 止血两件套 → 状态机 + 冷库 → 巡检 + 压缩仪式）；仓内对账 = [review-economics-rq6-inventory](../research/review-economics-rq6-inventory.md)（入口侧已有三问过滤器 / 长度预算 / 孤儿检查 / AGENTS.md freshness 注）。约束发现：冻结 ADR 正文引用 `session-model.md` 共五处（ADR-0007/0013/0017/0027/0028），正文冻结不可改——物理移动文档 = 制造永久断链。1+N 语境下膨胀是双重税：维护者付注意力税，agent 付上下文窗口税。2026-10-14 一批出示 Q1–Q7，维护者整批拍板（无改稿）。

## Decision

细节 SSOT = [doc-lifecycle](../design/doc-lifecycle.md) 对应节；此处冻结决策面：

1. **生命周期状态机** = 现役 ↔ 归档两态 + 删除闸，**原地标记制**（拒物理 attic 目录：本仓默认上下文 = AGENTS.md 地图而非目录浏览，地图标注即退出默认检索；物理移动冻断 ADR 链接且无法修复）。归档 = 状态头 `📦 已归档（日期）` + 原因 + 去向指针 + 地图条目 `archived` 标注；删除 = 维护者批准 + 无入链机器预检，被冻结 ADR 引用者只可归档不可删。decisions 以 `superseded by` 标记充任归档等价；research 时点快照与 backlog（每会话自刷新）豁免。
2. **freshness 元数据**：现役 `docs/design/*.md` 状态头必带 `Last reviewed: YYYY-MM`；纪律 = 改内容必 bump。机器承诺边界 = CI 只查在场与格式，「改动是否伴随 bump」归巡检 git 对账（hve-core「规定 ≠ 执行」教训：机器不谎报语义执行）。staged diff 级硬闸 = 扩展位（触发器 = 巡检反复抓到不 bump）。
3. **机器膨胀信号四项**（进 `just agent-check`，全部结构检查）：文档地图反向悬挂（AGENTS.md 指向的 `docs/...` 路径必须存在）；内部 `.md` 链接文件部分必须解析（锚点不查——顺带机械执行「移动即断链即失败」）；freshness 在场与格式；归档标记 ↔ 地图 `archived` 标注双向一致。明确不做：相似度冗余检测（误报风险，归巡检人判）、引用计数（扩展位）、TODO 机器告警（先靠巡检）。
4. **gardener 巡检**：每任务收官轻巡检（清单五项：freshness git 对账 / 冗余候选对 / 零维护文档列表 / agent-check 绿确认 / TODO 待决滞留扫描），产物进 backlog 遗留登记或任务块一行；深巡检 = 维护者明示。人类决策点仅两处 = 删除批准 + 压缩抽检。
5. **压缩仪式三原则**：保原文优先（删 > 移 > 并 > 改，生成式重写是最后手段）/ 压缩者不自评（self-review 子代理 pass + 维护者抽检 diff）/ 原文可恢复（git 史保底）。流程 = 提案出示 → 拍板 → 独立 change 执行 → 抽检；backlog 收官压单行 = 已运行实例，推广至全文档面。
6. **入口闸门增补**：新文档与地图条目同一 commit 入库 + 状态头声明（类别 / 冻结面指针 / 寿命预期）；canonical 指定条款化（重复文档合并或标「已吸收」转归档）；「废弃标记优于沉默」成协议条款。

### 对既有 ADR 的局部 supersede（同批吸收）

- [ADR-0012](0012-docs-governance.md) Decision 分类学格的 design / research 两格（「活文档」「时点快照」描述）——增补：归档态横切该两类（原地标记制 + 删除闸，本 ADR 条款 1）；decisions 以 `superseded by` 标记充任归档等价，四分类与 backlog 类原位。

## Consequences

- **T10 开工门槛自此全备**：审计流程 ✅（[ADR-0031](0031-testing-strategy.md)）+ 开发流程 ✅（[ADR-0032](0032-operating-protocol.md)）+ HANDOFF 迁移 ✅（同 T11 收官 commit）+ T12 收官 ✅（本 ADR + doc-lifecycle + agent-check 扩展随收官 commit 入库）——T10 阻塞解除。
- 机器承诺边界显式化：agent-check 只判结构（引用 / 日期 / 标注完整性），语义正确性归巡检与维护者抽检——任何「机器已保证文档仍正确」的读法都是过度承诺（RQ8 机制清单 #15 元纪律）。
- 同批：`docs/design/doc-lifecycle.md` 立（living 教义面 SSOT）；AGENTS.md（地图 +1 行 / `archived` 标注约定 / Commands 与 Freshness 注更新）；`doc-maintenance` skill 加生命周期节；`crates/xtask/src/agent_docs.rs` 四项检查 + 测试（ASK 面授权 = 2026-10-14 Q7 拍板，仅 `crates/xtask` 一处门禁面，`.pre-commit-config.yaml` / workflows / justfile 不动）；`session-model.md` = 首个正式归档件；现役 design docs 补 `Last reviewed` 基线（= 2026-10）；火-2 消费（引文与症状映射固化 = doc-lifecycle §1 与本 ADR Context，backlog 火花区条目删除）。
- 扩展位登记（SSOT = doc-lifecycle §8）：staged diff 级 freshness 硬闸 / 相似度冗余机器检测 / 引用计数埋点 / TODO 滞留机器告警。
- 落地即绿：四项检查上线时存量实测 0 断链 / 0 反向悬挂 / 0 freshness 缺失（基线补齐后）/ 0 归档不一致。
