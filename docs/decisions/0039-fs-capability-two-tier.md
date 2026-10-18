# 0039. fs 能力面收窄：private 隐式 / global 声明（命名根撤回）

- Status: accepted（局部 supersede [ADR-0036](0036-fs-scope-three-layers.md) Decision 第 2 条「命名根」；[ADR-0025](0025-seam-capabilities.md) Status 行记本 ADR 注）
- Date: 2026-10-19

## Context

ADR-0036 在 wasm-first 下把 fs 扩为三层：private（preopen）、命名根（manifest 声明 + 部署者绑定）、global（host-mediated 减法语义）。命名根的动机是给「插件需要某个用户目录」一个比 global 窄的中间档，替代原 workspace。

设计复核暴露两条理由：

- 插件的目标目录可能随 project 变化，一个插件也可能同时服务多个 project；命名根要求「一次声明、部署时绑定」，静态形状配不齐动态需求。
- private 是插件自己的数据目录（`plugins/<name>/data/fs/`），不是外部权限；为它要求声明与读写档只增加仪式，不构成安全边界。

同时需要澄清 sqlite 与 fs 的关系：host sqlite 的库住 `plugins/<name>/data/db.sqlite`，与 private fs 根 `data/fs/` 平级；private preopen 结构性够不到 db，写 private fs 不等于写 db。

## Decision

fs 能力面 = **两档**：

1. **private**：`plugins/<name>/data/fs/` 以 WASI preopen 授予；**隐式读写**，不声明、不可收窄。它是插件私有数据面，不是能力门。
2. **global**：减法语义（用户世界 − `secrets.yml` + `journal/` 两条豁免），host-mediated 自研逃逸舱；**必须在 manifest 声明** `capabilities.fs.global: read|write`，缺省 = 无 global。TOCTOU 硬化与路径解析留在宿主实现。

- **命名根不立**：manifest 不设 `roots` 字段，部署根不设 `fs_roots` 绑定节，绑定机制整体撤回。ADR-0036 Decision 第 2 条被本条取代；private preopen 与 global 自研两档机制保留。
- **`capabilities.fs` 形状** = `{ global: read|write }`；`private` 不是合法键（写入 = 未知字段响亮，防旧文档漂移）。
- **sqlite 不变**：`sqlite: true` 是宿主代持 SQLite 句柄的可选便利，不是通用文件写权限；库文件住 `data/db.sqlite`，private fs 够不到。插件使用其他 DB（redb / sled / 自带 SQLite 等）时直接在 private fs 内管理，无需 `sqlite` 声明。
- **mount 不变**：`mount: true` = 允许 `ctx.mount` 创建子树实例（ADR-0028 批次 #1）；`false` / 缺省 = 拒绝。
- **审批面**：private 不进门；global 最高档（碰用户文件 = 要么信要么不信）；无中间档。偏执部署走 OS 级沙箱（外墙归部署组成）。
- **零仪式插件**：private fs 结构性授予读写；global / http / sqlite / process / mount 仍 fail-closed（[wit-abi](../design/wit-abi.md) W1.8 D123 措辞同步）。

## Consequences

- 局部 supersede ADR-0036 Decision 第 2 条；ADR-0036 的 private preopen 与 global 自研两步继续有效。
- [ADR-0025](0025-seam-capabilities.md) Status 行补注：fs 两档格先由 ADR-0036 改三层，再由本 ADR 收窄为 private 隐式 / global 声明；能力集语义本体不变。
- 同步面：[wit-abi](../design/wit-abi.md) W1.8 / W2（D129/D130）、[config-format](../design/config-format.md) C5、[seam-capabilities](../design/seam-capabilities.md) §5.2/§6.1、[instance-model](../design/instance-model.md) S1.2/S1.6/S3.7/S5.3、ADR-0027 Status 行、台账 T10.11 站行。
- 明记代价：没有中间档；需要任意用户目录的插件只能声明 global（高信任），部署者以审批与 OS 沙箱兜底。workspace / session 语义仍归插件层服务投影，核心不做一一映射。
- 动态 project 目录的答案 = global + 插件自己的 config 值（目录路径由部署者在插件配置里给），不再有 per-name 绑定机制。
