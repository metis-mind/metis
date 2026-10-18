# 0036. fs scope 三层（E4 固化）：private preopen / 命名根 / global 自研

- Status: accepted（Decision 第 2 条「命名根」局部 supersede = [ADR-0039](0039-fs-capability-two-tier.md)，2026-10-19；正文冻结）
- Date: 2026-10-18

## Context

[ADR-0027](0027-config-format.md) D44 把 fs scope 收窄为 `{private, global}`，并把 workspace 与绑定机制整体判为不立。wasm-first 转向（[ADR-0034](0034-wasm-first-carrier.md)）改变了成本结构：WASI filesystem 的 preopen 模型让 private 可以结构性收口；原 workspace 需求的诚实形态是 per-name 的命名根，可由 manifest 声明 + 部署者绑定表达；global 是减法语义（用户世界 − 两条豁免），preopen 表达不了，仍需 host-mediated 自研逃逸舱。

E4 于 2026-10-16 拍板（SSOT = 台账 T13 状态行）。本 ADR 固化语义面，preopen 挂载、命名根解析与 global 自研的实现形态归 W3/W9 章与 T10.11。

## Decision

fs scope = 三层：

1. **private**：`plugins/<name>/data/fs/` 以 WASI preopen 授予；guest 在 preopen 内使用常规文件 API，逃逸防护 = 运行时结构保证。
2. **命名根**：manifest 声明 + 部署者绑定，preopen 承载；读 / 写分档按声明授予。命名根 = 原 workspace 绑定需求的 per-name 形态，不做单数工作区语义。
3. **global**：减法语义（用户世界 − `secrets.yml` + `journal/` 两条豁免），host-mediated 自研逃逸舱；TOCTOU 硬化与路径解析留在宿主实现。

workspace / session 语义归插件层服务投影，核心不做一一映射。写 = 临时文件 + rename 原子写不变。

## Consequences

- 局部 supersede [ADR-0027](0027-config-format.md) D44 的 fs scope 两档格与「绑定机制整体不立」中的命名根部分；单数 workspace 仍不立。
- 局部 supersede [ADR-0025](0025-seam-capabilities.md) §5.2 fs 通道的两档格；能力集语义本体（fs/http/sqlite/timer/process）不变。
- 审批面：命名根在 manifest 声明中可见，部署者绑定时审批；process 对 fs 分段与两条豁免的支配关系不变。
- 实现形态（preopen 挂载、命名根解析、global 自研）归 W3/W9 章与 T10.11。
