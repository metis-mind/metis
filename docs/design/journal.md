# Metis — Journal 设计（核心操作/审计日志；living doc）

> 状态：J1 ✅ 定位与条目模型（2026-10-12，D75–D78）；J2–J6 待收官。
> 前提文档：[luau-abi](luau-abi.md) §A6.7（journal 记录格式草案；replay 面修订注见彼）/ [seam-capabilities](seam-capabilities.md) §5.3/§5.4 / [fiber](fiber.md) §10 / [config-format](config-format.md)（D29/D50）/ [instance-model](instance-model.md)（D60/D72/D73）。
> 调研依据：[journal-replay-research](../research/journal-replay-research.md)（point-in-time 报告；其 §7 映射的 replay 面经本任务方向重定后不再采用——见 D75）。
> **方向重定（2026-10-12 用户拍板）**：核心**不立 replay 引擎**——崩溃恢复 = fresh-start（[ADR-0023](../decisions/0023-core-restart-semantics.md) 同路径），journal = 核心操作/审计日志，replay = 延期扩展位（J6 登记重开触发器）。§A6.7 草案的 replay 面（★ 族轴 / D7.2 生成型秘密加密 / 概念地基「重启 = 重放」「b = 立身之本」）随之推翻或消解，正式 supersede 归 J6 冻结批次（拟 ADR-0029）；D7.3 格式纪律（JSONL + 格式版本字段 + Int 字符串惯例）不受影响，继续有效。

## 0. 术语锚点

- **journal**：核心的 append-only 操作/审计日志——每租户一本（[instance-model](instance-model.md) D60）、逻辑单流（host 单点定序单写者）、JSONL 一行一条（格式纪律 = [luau-abi](luau-abi.md) §A6.7 D7.3）。
- **飞行记录仪**：journal 的自用定位——崩溃/故障后的调查源，不是状态重建源（D75）。
- **锚点条目**：携带版本/配置锚信息的低频生命周期条目（`boot` / fiber 进 Active）；普通条目不背版本税，审计按 `seq` 向前找最近锚点（D76）。
- **族 / 档位**：kind 表按族组织（生命周期 / 能力使用 / 内部流量 / 错误现场 / 控制）；可配族的录制档位 = `core.audit` 三档 `off`/`summary`/`full`（[config-format](config-format.md) §C8.1 D50），默认档归 J2。
- **段文件**：`journal/` 目录下按 seq 区间轮转的物理文件组（布局/保留/归档归 J4）。
- **blob**：超阈值 payload 的内容寻址存储，条目记 hash + 引用（D7.1 平移；阈值与 GC 归 J4）。

## J1 定位与条目模型（2026-10-12 收官）

### J1.0 边界与前提

本题定 journal 的定位、条目公共字段、自由态 Int 标记形态与 kind 表总装。不归本题：档位默认值与写入路径管线（J2）；崩溃恢复与跨重启连续性细则（J3）；脱敏管线 / blob / 段文件 / 保留归档（J4）；查询接口与人读面（J5）；replay 扩展位登记与冻结批次（J6）。前提（既定不议）：D7.3 格式纪律；D60 每租户一本；D72 条目不押实例字段；D73 跨实例调用 = 内部流量；ADR-0023 干净重启语义。

### J1.1 D75 定位锤：journal = 核心的操作/审计日志（2026-10-12 用户拍板）

- **定位**：journal = 核心的 append-only 操作/审计日志。消费者三层：**① 核心自己** = 崩溃/故障诊断（飞行记录仪本义：调查，不重建）+ 自审计；**② 插件层** = journal 查询接口（平台能力，J5 设计）；**③ 部署者** = 能力使用审计与调试（ADR-0024 §6 防线 2「藏不住」的兑现面）。
- **三条「不是」**（边界写实）：不是 replay 事实源（replay 引擎不立 = 扩展位）；不是运行日志（tracing/debug 输出 = 人读诊断面，既有边界不动）；不是插件 transcript（会话轨迹 = 插件层经 fs/sql 自建——DSH session log 已验证的形态；查询接口给插件的是审计关联面，不是存储面——**审计档位变化不许弄坏插件功能**）。
- **崩溃恢复 = fresh-start**：复用 ADR-0023 同一路径（全量 unload + 全量 load），崩溃与干净重启的差别只在在飞工作丢失多少，恢复路径不分叉。在飞邮箱消息崩溃丢失 = 诚实（D11.4/D11.5 同族：at-most-once、拒绝重投——被杀 handler 可能已发出过回复）。插件跨重启连续性 = 插件自己的 fs/sql 持久化（seam 落盘原子写纪律兜底）；待审批状态 = 审批机器文件原子写，不靠 journal。
- **正本退位**：fresh-start 后 journal 不是 system of record（正本 = 部署文件 + 插件私有存储），是派生诊断物——段文件轮转/保留/删除 = 纯运维政策，无完整性枷锁（Akka「不删事件」戒律只对 event-sourcing 正本成立）。journal 数据生命周期细则归 J4（[config-format](config-format.md) §C7.0 登记位）。
- **随之消解**（登记落档，各一句理由）：
  - §A6.7 概念地基「重启 = 重放」「b 崩溃恢复 = 核心层立身之本」——推翻为 fresh-start + 诊断；
  - 调研开放问题 1（单双流）——replay 流不存在，「保留期分叉」论据随之消失，单流自明；
  - 调研开放问题 3（fork 前缀物理组织）/ 4（发散比对强度）——replay 不立，消解；
  - M8 host 侧快照——replay 检查点，随 replay 消解；
  - 私有状态族（fs/sql）replay 政策（[seam-capabilities](seam-capabilities.md) §5.4 头号问题）——无 replay 则无政策；
  - `nondeterminism_read` 族与 D7.2 生成型秘密加密——不录制时间/随机读取，则无密可泄、加密议题消失；
  - D8.2 在飞预算数值钉版登记、D1.6 免录恢复顺序的「确认」、seam §1.6 指针身份串比对政策——replay 语义需求，消解（调度纯函数性本身 = ABI 既定，不动）。
- **replay 扩展位**（延期非删除）：replay 的消费者（调试 / eval / 「新代码重跑旧史」验证自我修改 = Fowler bug-fix 重放）真实存在，但在插件/agent 层且非 v1 必需；机器必须长在 host 侧（边界注入点是 host 的，插件无法虚拟化另一个插件的边界）。重开触发器与形态归 J6 登记。wasm 盒 IO 全走 host imports = 天然可录制（wasm-research §5.7），届时是红利。
- **淘汰**：replay 引擎进 v1（为无消费者的 host 级虚拟化预埋复杂度）；「只审计不管诊断」（崩溃后没有飞行记录仪，核心故障无从调查）。

### J1.2 D76 公共字段定稿（2026-10-12 用户拍板）

每条必备：

| 字段       | 类型           | 说明                                                                                            |
| ---------- | -------------- | ----------------------------------------------------------------------------------------------- |
| `seq`      | u64            | **每租户**单调序号（每租户一本 = D60 的必然推论；host 单点定序单写者）                          |
| `wall_ms`  | i64            | 墙钟毫秒（展示用，不参与逻辑；[ADR-0015](../decisions/0015-payload-value-model.md) 时间戳纪律） |
| `kind`     | string         | 条目类型，封闭枚举（D78 kind 表 = SSOT）                                                        |
| `fiber`    | object \| null | `{slot, generation, plugin}`；host/核心动作条目 = null                                          |
| `cause`    | u64 \| null    | 触发本条目的条目 seq（因果链；Temporal 反向 ID 引用的单序列简化版）                             |
| `payload`  | Value          | 条目体（脱敏后；落盘纪律 = D7.3 + D77）                                                         |
| `redacted` | array \| null  | 被脱敏的 payload 路径 + 策略版本（M6 平移；审计可见「哪里被擦过」）                             |

- **代码版本 = 锚点条目携带制**：普通条目只带 fiber 身份；版本锚信息由低频生命周期条目携带——`boot` 条目带 host_version + 配置树 hash；`fiber_state_changed` 进 Active 带 plugin_revision + config hash（实例件另带声明 hash——与 [instance-model](instance-model.md) D65 挂载快照三样一一对应：声明拷贝 → 声明 hash、配置合成结果 → config hash、代码版本 → plugin_revision）。审计问「当时跑的什么代码」= 按 seq 向前查最近锚点条目。漂移写实：`on_config` live-apply（合并取最新、不重挂载 = [config-format](config-format.md) D42）后 fiber 锚点的 config hash 变陈旧——「当时跑的是什么配置」= 锚点 join 沿途 `config_committed`（机器齐备，查询侧一次串联）。
- **淘汰**：(b) per-entry `code_ref`（调研 M4 原形：冗余税压在最密的能力族条目上，信息量不变）；(c) 不记代码版本（「当时跑的是什么」失明，违藏不住）。M4 的 `preset_revision` 消解——任务 5 后核心无 preset 概念，实例件由声明 hash 接替。
- 无租户字段（每租户一本，物理分离天然解决）；无实例字段（D72 结案原位）。

### J1.3 D77 自由态 Int 标记形态 = 包裹对象（2026-10-12 用户拍板；§A6.7 D7.3 登记位兑现）

- **规则**：已知 kind 的声明字段 = 按 kind 表 schema 感知还原（D7.3 既定读侧），不包裹；**schema 管不到的自由态 Int** 落 `{"$int": "<十进制>"}` 壳——类型信息零静默丢失。
- **淘汰**：(b) 裸字符串不区分（自由态 Int/String 同落 JSON string，查询接口（J5）返回错乱类型 = 插件侧 bug 温床，违严格制）；(c) 安全域内保 number、超界转字符串（同字段类型随数值幅度漂移，违严格制；D7.3 对 seq/wall_ms 的例外是 kind 已知的固定字段，不许推广）。

### J1.4 D78 kind 表总装 + 消解记录（2026-10-12 用户拍板）

**分族原则**：生命周期族 + 错误现场族 + 控制族 + `process_*` 两条 = **写死全录**（低频 + 「藏不住」两面：管理动作与异常永远可见；控制族 = 审计自保同族——`journal_gap`（背压丢弃事实）与 `redaction_applied`（脱敏发生事实）若可配关，等于审计失明可由被审计面自己拨档，与 `audit_policy_changed`「谁关了审计窗必须可查」同构）；能力使用族其余 + 内部流量族 = **可配档**（`core.audit` 三档，键面与默认档归 J2）。本表定 kind 存在性与 payload 形状；本表即 payload schema 的 SSOT（D7.3/D77 的还原依据）。

| 族       | kind                                   | 内容要点                                                                                                                                                                            | 来源登记                     |
| -------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| 生命周期 | `boot`                                 | 租户 boot 完成：host_version + 配置树 hash + 启用插件集摘要（**锚点条目**）                                                                                                         | D76                          |
| 生命周期 | `fiber_state_changed`                  | 状态迁移 from→to；进 Active 携带 plugin_revision + config hash（实例件 + 声明 hash）；Failed 带错误摘要（**锚点条目**；[ADR-0016](../decisions/0016-internal-hooks.md) 钩子面镜像） | 调研 §7.3                    |
| 生命周期 | `mounted` / `unmounted`                | 根 fiber id + 挂载方 fiber + 声明来源指路与 hash + 插件集摘要（D72 人读面 host 日志的 journal 镜像）                                                                                | D72 边界裁定                 |
| 生命周期 | `config_committed` / `config_rejected` | 应用 = 配置 generation/hash + 来源（审批引用/手写 reload）+ 受影响 entry 摘要；拒绝 = 原因 + 文件指路                                                                               | C4.4                         |
| 生命周期 | `audit_policy_changed`                 | `core.audit` 换档 旧→新（D50 生效路径；**审计自保**：「谁关了审计窗」必须可查）                                                                                                     | C8.1 衍生                    |
| 能力使用 | `http_request` / `http_response`       | 方法/域名/状态码/耗时/字节；body 超阈值走 blob 引用（D7.1 平移）                                                                                                                    | seam §5.3                    |
| 能力使用 | `fs_read` / `fs_write`                 | scope/路径/字节数（`append` 归此族）                                                                                                                                                | seam §5.3 + §A4.1            |
| 能力使用 | `sql_call` / `sql_result`              | 语句/参数摘要/行数                                                                                                                                                                  | seam §5.3                    |
| 能力使用 | `process_spawn` / `process_exit`       | 命令/参数/退出码/输出截断注记（**全录写死** = 「藏不住」护栏；spawn 流式内容不进 = §A3.10 平移）                                                                                    | §A3.10/§A3.13                |
| 能力使用 | `timer_scheduled` / `timer_fired`      | timer id/间隔/届时（sleep 复用 = §A3.6 兑现）                                                                                                                                       | seam §5.3 + §A3.6            |
| 能力使用 | `stream_opened` / `stream_closed`      | 流句柄 id + 源族（http 下载/process stdout/fs openread）+ 方向 + 关流时字节总量；chunk 内容不立                                                                                     | §A6.5 + D7.1 平移            |
| 内部流量 | `message_enqueued`                     | 目标 fiber + 消息类（事件四模式/服务调用/dispose/timer tick）+ payload（降档时摘要化）                                                                                              | 调研 §7.3                    |
| 内部流量 | `dispatch_completed`                   | 四模式派发结果摘要                                                                                                                                                                  | 调研 §7.3                    |
| 内部流量 | `service_call` / `service_result`      | 调用方/提供方/方法/结果或 err kind                                                                                                                                                  | 调研 §7.3                    |
| 错误现场 | `listener_error` / `vm_trap`           | fail-open 监听器抛错落点；VM 层异常                                                                                                                                                 | §A5.5 D15.2 + 调研 §7.3      |
| 错误现场 | `contract_violation`                   | 契约违约合一（提供方返回违约 D19.1 + emit 违约 D22.4）：violation 描述 + 契约指路                                                                                                   | §A7.5/§A8.5                  |
| 错误现场 | `fs_invalid_path_detail`               | `fs.invalid_path` 完整拒绝原因（插件可见 message 通用化的互补面 = D14.2）                                                                                                           | §A5.5                        |
| 错误现场 | `event_shape_degraded`                 | 卸载致事件形状定义缺失的降级注记                                                                                                                                                    | §A8.5 D22.6                  |
| 错误现场 | `pending_warn` / `reconcile_failed`    | D25.1 warn（进 Pending 当场/转 Failed 当场：谁缺哪个 key、提供方、指路两条）；启用期对账失败详情（五格）                                                                            | §A9.2/§A9.4                  |
| 错误现场 | `fanout_cancelled`                     | handler 结束强制取消子协程：handler 指路 + 取消计数                                                                                                                                 | §A4.1 登记                   |
| 控制     | `unload_gate_closed` / `unload_killed` | 关闸丢弃清单（邮箱 + 可调度队列残留摘要、服务调用即错计数）；强杀点在飞线程清单                                                                                                     | D11.1/D11.4/D11.5            |
| 控制     | `redaction_applied`                    | 脱敏策略版本 + 命中计数（聚合条目，非每命中一条）                                                                                                                                   | M6 平移                      |
| 控制     | `journal_gap`                          | 写入背压丢弃计数 + seq 区间（写入路径的诚实降级：丢条目可查；管线联动归 J2）                                                                                                        | 2026-10-12 存储/性能讨论增补 |

**消解记录**（随本条落档，各一句理由）：`external_input`（去 agent 化：核心无「session 第一落点」，外部刺激 = 部署文件动作（boot/config 条目）+ 能力族回流）；`fork_created`（fork 退出核心需求集 = §A6.7 概念地基既定）；`session_started`/`session_ended`（D53：核心无 session 概念）；`llm_*`（LLM 调用 = 插件的一次 http，归能力族）；`snapshot_written`（M8 随 replay 消解）；`nondeterminism_read`（D75）；`replay_diverged`（D75）。

**mount/unmount 进 journal 的取舍**（D72 留下的「人读面与 journal 边界」裁定）：进——mount = 激活新代码的元权限动作（[instance-model](instance-model.md) D63），审计面少它 = 防线 2 破窗；成本 = 低频两条。D72 人读面 host 日志不动（人读告警与机读审计两通道并存）。

### J1.5 议题地图（六站）

| 站 | 内容                                                                                                                                                                                                                                                    | 状态          |
| -- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- |
| J1 | 定位与条目模型（D75–D78）                                                                                                                                                                                                                               | ✅ 2026-10-12 |
| J2 | 审计政策、音量与写入路径：per-family 默认档；聚合摘要形态；高频族整形；写入管线（异步批量 writer / 背压 `journal_gap` / flush/fsync 政策）；ADR-0024 §6 防线 2 张力结案；D23.3 审计写实的档位依赖对账（内部流量档 ≠ off 才保住「藏不住」）              |               |
| J3 | 崩溃恢复与跨重启连续性：fresh-start 细则（与 ADR-0023 load 路径衔接）；在飞丢失写实；插件持久化模式；agent 连续性 = 插件层结案；崩溃窗口（flush 政策联动）                                                                                              |               |
| J4 | 写入侧工程：脱敏管线形态（M6；秘密身份注册 = §A2.5 登记位）；blob 阈值/GC；段文件布局/轮转/保留/归档（[config-format](config-format.md) §C1.3 登记位）；journal 数据生命周期（§C7.0 登记位）；写入量实测                                                |               |
| J5 | 查询接口与人读面：插件面查询接口（形态/能力门/可见范围 = §A3.7 豁免的正当访问面）；部署者 audit 消费面；journal ≠ 运行日志边界写实                                                                                                                      |               |
| J6 | replay 结案 + v1 收口 + 冻结：replay 引擎 = 扩展位（重开触发器明写）；消解清单落档；supersede 批次（§A6.7 修订面 + seam §5.3/§5.4 + fiber §10 + ADR-0024 §6 注评估）；冻结 = 拟 ADR-0029；冻结前统扫 rationale 层 replay 残留（决策本体不动，措辞注记） |               |

### J1.6 联动登记汇总（J1）

| 去向                                                                           | 内容                                                                                                                                                |
| ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| luau-abi §A6.7                                                                 | 方向修订注（replay 不立）；D7.1 ★ 轴 / D7.2 / 概念地基 / 登记行修订注；D7.3 Int 标记兑现注 = D77（本 change 已同步）                                |
| luau-abi §A6.8                                                                 | 任务 6 行 = J1 收官注 + dispatch 更新（本 change 已同步）                                                                                           |
| luau-abi §A2.8 / §A3.6 / §A3.7 / §A3.10 / §A3.13                               | 脱敏注册 dispatch = J4；timer kind / 豁免行 / process 两条与流式政策 兑现注 + replay 族消解注（本 change 已同步）                                   |
| luau-abi §A4.1 / §A4.2 / §A4.5 / §A4.6                                         | append 归族 / fanout 取消 / unload 关闸与强杀两条 兑现注；D8.1/D8.2 免录与钉版 消解注（本 change 已同步）                                           |
| luau-abi §A5.3 / §A5.4 / §A5.5 / §A7.5 / §A8.0 / §A8.5 / §A9.0 / §A9.2 / §A9.4 | listener_error / fs_invalid_path_detail / contract_violation / event_shape_degraded / pending_warn / reconcile_failed 兑现注（本 change 已同步）    |
| luau-abi §A7.4 / §A8.4                                                         | D19.1 正文兑现注；D23.3 审计写实档位依赖注（依赖挂进 J2；本 change 已同步）                                                                         |
| seam-capabilities §5.3 / §5.4 / §6.2 / §1.6 / §4.2                             | kind 表 SSOT 移交注；replay 列 / 分族原则 / 头号问题 / 指针串比对 消解注；输入清单 dispatch 更新（本 change 已同步）                                |
| fiber §10                                                                      | 跨重启状态政策方向注 = fresh-start（细则归 J3；本 change 已同步）                                                                                   |
| config-format §C4.2 / §C4.4 / §C8.1                                            | config_committed / config_rejected 兑现注 + D50 可调面重述注（§C8.1：★ 轴消解 / code_ref 锚点化 / 可调族面 = D78）（本 change 已同步）              |
| config-format §C1.1 / §C1.3 / §C7.0                                            | 段文件 / blob / journal 数据生命周期 dispatch = J4（本 change 已同步）                                                                              |
| config-format §C6.2 / §C6.3                                                    | 加密密钥形态消解注；脱敏管线 dispatch = J4（本 change 已同步）                                                                                      |
| config-format §C8.0 / §C8.3                                                    | audit 档语义 dispatch = J2（本 change 已同步）                                                                                                      |
| instance-model §S1.6 / §S2.2 / §S4.7                                           | 任务 6 已启动注（每租户一本已吸收 = D75/D76）；跨重启连续性 pointer = J3；D73 行 replay 半句消解 + kind 表 SSOT 指针注（§S4.7）（本 change 已同步） |
| ADR-0026                                                                       | §A6.7 修订面 supersede 归 J6 冻结批次（拟 ADR-0029）——Status 行注届时落                                                                             |
| 实现期清单                                                                     | journal append 管线（异步批量 writer + 有界通道 + `journal_gap` 降级 = J2 定案后落地）；段文件轮转（J4）；查询接口（J5）                            |
| 扩展位                                                                         | replay 引擎（重开触发器归 J6 明写）                                                                                                                 |
