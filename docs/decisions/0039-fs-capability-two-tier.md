# 0039. 能力面收窄：fs 两档标量化 + sqlite 撤回

- Status: accepted（局部 supersede [ADR-0036](0036-fs-scope-three-layers.md) Decision 第 2 条「命名根」与 [ADR-0025](0025-seam-capabilities.md) §4 sqlite 项；[ADR-0027](0027-config-format.md) C1.1 `data/db.sqlite` 行同步失效）
- Date: 2026-10-19

## Context

ADR-0036 在 wasm-first 下把 fs 扩为三层：private（preopen）、命名根（manifest 声明 + 部署者绑定）、global（host-mediated 减法语义）。命名根的动机是给「插件需要某个用户目录」一个比 global 窄的中间档，替代原 workspace。

设计复核暴露两条理由：

- 插件的目标目录可能随 project 变化，一个插件也可能同时服务多个 project；命名根要求「一次声明、部署时绑定」，静态形状配不齐动态需求。
- private 是插件自己的数据目录（`plugins/<name>/data/fs/`），不是外部权限；为它要求声明与读写档只增加仪式，不构成安全边界。

同时，host 代持 SQLite 的价值不成立：guest 可以直接在 private fs 内使用 SQLite / redb / sled / 任何自带 DB；`sqlite: true` 只是把「插件私有数据」再包一层宿主句柄，既不是安全边界，也不是唯一实现路径。独立能力留下的只有额外 API、错误族与生命周期面。

## Decision

### 1. fs 两档，global 标量声明

1. **private**：`plugins/<name>/data/fs/` 以 WASI preopen 授予；**隐式读写**，不声明、不可收窄。它是插件私有数据面，不是能力门。
2. **global**：减法语义（用户世界 − `secrets.yml` + `journal/` 两条豁免），host-mediated 自研逃逸舱；**必须在 manifest 声明** `capabilities.fs: read|write`，缺省 = 无 global。`write` ⊃ `read`。TOCTOU 硬化与路径解析留在宿主实现。

- **命名根不立**：manifest 不设 `roots` 字段，部署根不设 `fs_roots` 绑定节，绑定机制整体撤回。ADR-0036 Decision 第 2 条被本条取代；private preopen 与 global 自研两档机制保留。
- **`capabilities.fs` 形状 = 标量** `read|write`（即 global 档）；不写 `global` 子键，也不写 `private`（private 隐式）。`fs` 缺省 = 无 global。
- 未来若重新引入中间档，声明形状随 WIT package version additive 演进（W10），不保留占位子键。

### 2. sqlite 能力撤回

- `capabilities` 不再有 `sqlite` 键；写入 = 未知字段响亮。
- 宿主不再代持 `plugins/<name>/data/db.sqlite`，不再提供 SQL 方法族 / `sql.*` 错误族 / `sql_call` / `sql_result` journal kind（旧设计面随 ADR-0025/0026 supersede 撤回）。
- 插件需要数据库时，自带 guest crate（SQLite / redb / sled 等），库文件住 `data/fs/` 内，由插件自己管理 schema、迁移与生命周期。private fs 隐式读写使这条路径无需任何能力声明。
- 数据库不是宿主能力：不参与能力门、不参与审批面能力栏、不产生宿主侧 DB 句柄。

### 3. 不变项

- **mount 不变**：`mount: true` = 允许 `ctx.mount` 创建子树实例（ADR-0028 批次 #1）；`false` / 缺省 = 拒绝。
- **http / process / timer 不变**：http 与 process 仍需声明；timer 无门；纯算件无需声明。
- **审批面**：private 不进门；global 最高档（碰用户文件 = 要么信要么不信）；无中间档。偏执部署走 OS 级沙箱（外墙归部署组成）。
- **零仪式插件**：private fs 结构性授予读写；global / http / process / mount 仍 fail-closed（[wit-abi](../design/wit-abi.md) W1.8 D123 措辞同步）。

## Consequences

- 局部 supersede ADR-0036 Decision 第 2 条；ADR-0036 的 private preopen 与 global 自研两步继续有效。
- 局部 supersede [ADR-0025](0025-seam-capabilities.md) §4 内建效应能力的 sqlite 项、§5 出区通道的磁盘列举 / journal kind 映射 / replay 分族三条；[ADR-0025](0025-seam-capabilities.md) Status 行记本 ADR 注。
- [ADR-0027](0027-config-format.md) C1.1 的 `data/db.sqlite` 行失效；插件运行期区 `data/` 只剩 `fs/` 根，插件自带 DB 住 private fs 内。Status 行记局部 supersede。
- 同步面：[wit-abi](../design/wit-abi.md) W1.8 / W2（D129/D130）、[config-format](../design/config-format.md) C1/C2/C5/C7/C8、[seam-capabilities](../design/seam-capabilities.md) §2/§4/§5/§6、[instance-model](../design/instance-model.md) S1.2/S3.3、[journal](../design/journal.md) J1.4/J2/J3.4、ADR-0024/0028 Status 注、台账 T10.11 站行。
- 明记代价：没有中间档；需要任意用户目录的插件只能声明 global（高信任），部署者以审批与 OS 沙箱兜底。workspace / session 语义仍归插件层服务投影，核心不做一一映射。
- 动态 project 目录的答案 = global + 插件自己的 config 值（目录路径由部署者在插件配置里给），不再有 per-name 绑定机制。
- 数据库能力撤回的代价：插件重复携带 DB 依赖；换来的是核心从 DB 生命周期、错误族与迁移面退出。触发重议 = 插件 DB 二进制体积/冷启动投诉成串，且 guest 生态无可接受替代。
