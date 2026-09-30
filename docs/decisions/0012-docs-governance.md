# 0012. 文档治理：模板对齐 + 中文正本 + HANDOFF 易失

- Status: accepted
- Date: 2026-09-30

## Context

工程化落地对齐 `duskgrow/rust-template`（[ADR-0009](0009-engineering-baseline.md) 遗留③）。模板的文档政策是英文正本 + zh-CN 译本；而本项目全部既有语料为中文、唯一人类读者（维护者）母语中文、设计讨论以中文进行——翻译为英文会丢失讨论语境且成本高。`docs/HANDOFF.md` 经维护者明确为易失交接物：不入库、看完即销毁、每次交接重写。

被淘汰的选项：全面英文正本（对齐模板但语料迁移成本高、维护者评审摩擦大）；全部中文（AGENTS.md/skills 与模板生态分叉，丧失回流可能）。

## Decision

- ADR 采用模板 MADR 轻量格式：一文件一条，`docs/decisions/NNNN-title.md`，编号四位零填充、永不复用；✅ 后正文冻结，推翻 = 新 ADR supersede（旧文件标 `superseded by ADR-NNNN`，不删）；别处引用只写编号
- **语言分层**：`docs/`（decisions / design / research）中文为正本；代码、rustdoc、AGENTS.md、`.agents/skills/`、README / CONTRIBUTING 英文。这是对模板语言政策的显式偏离
- 文档分类学：`docs/decisions/` 冻结决策；`docs/design/` 活文档（头部状态行，决策点确认后晋升为 ADR）；`docs/research/` 时点快照（更正走增补节）；`HANDOFF.md` 在仓库根、gitignored、易失
- 不建文档模板库：格式由既有样例 + `doc-maintenance` skill 承载；治理规则落 AGENTS.md 文档节

## Consequences

`doc-maintenance` skill 的 EN-canonical 段落以 AGENTS.md 的项目覆盖为准（skill 内已加注指向本 ADR）；README/CONTRIBUTING 不维护 zh-CN 译本（私有项目无双语读者）。HANDOFF 的交接纪律：新会话先读HANDOFF，工作结束时重写。
