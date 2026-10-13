# 0029. Journal 冻结（J1–J6：核心操作/审计日志）与既有 ADR 局部 supersede 批次

- Status: accepted
- Date: 2026-10-13

## Context

任务 6（journal 正式设计）2026-10-12～13 四站收官（J1 定位与条目模型 / J2 审计政策、音量与写入路径 / J3 范围重定与崩溃恢复 / J4 写入侧工程缩小版；J5 查询接口缓议转扩展位；J6 = 本收口；决策点 D75–D92）。议题前身 = Luau ABI 的 journal 记录格式草案（[ADR-0026](0026-luau-abi.md) A6 节，living doc §A6.7）。**双重方向重定**（2026-10-12 用户拍板）：① 核心**不立 replay 引擎**——崩溃恢复 = fresh-start（[ADR-0023](0023-core-restart-semantics.md) 同路径），replay = 延期扩展位；② **v1 = 写死族骨架**（D84）——可配审计面（`core.audit` 六键/三档/聚合摘要/异步写入管线，J2 定稿次日撤销）整体留档 = 扩展位「可配审计层」预制方案。定案全文、淘汰选项、议题地图与联动登记汇总表在 [journal](../design/journal.md)（living doc）。前提：[ADR-0023](0023-core-restart-semantics.md)（干净重启语义）/ [ADR-0024](0024-carrier-layering.md) §6（信任模型防线 2）/ [ADR-0026](0026-luau-abi.md)（§A6.7 草案 = 头号输入）/ [ADR-0027](0027-config-format.md)（§C1.3/§C7.0/§C8 登记位）/ [ADR-0028](0028-instance-model.md)（D60 每租户一本 / D72 条目不押实例字段 / D73 跨实例调用结案）；调研依据 = [journal-replay-research](../research/journal-replay-research.md)（其 §7 映射的 replay 面不再采用）。本 ADR 冻结 journal 的决策面，并同批吸收各站在旧 ADR 上登记的局部 supersede 意向（interim 注已在各 living doc 对应节）。

**定调（任务 6 全程）**：journal = 核心自己的飞行记录仪（调查，不重建）；**正本退位**——journal 非 system of record，是派生诊断物（轮转/保留/删除 = 纯运维政策，无完整性枷锁）；写死族骨架全录 = 「藏不住」护栏（管理动作与异常永远可见；审计自保 = 写死族无键可拨）；音量/成本论据押结构界不押「典型负载」（2026-10-12 用户指正）；文件面两条用户硬性要求（2026-10-13）：段文件序列按时间有序、长期运行单文件与总量双结构界。

## Decision

### J1 定位与条目模型（D75–D78）

- **D75 定位锤**：journal = 核心的 append-only 操作/审计日志，每租户一本、逻辑单流（host 单点定序单写者）、JSONL 一行一条。消费者三层：① 核心自己 = 崩溃/故障诊断（飞行记录仪本义：调查，不重建）；② 插件层查询接口（缓议转扩展位）；③ 部署者审计（v1 收窄 = 管理动作/错误现场/process 可见）。三条「不是」：不是 replay 事实源、不是运行日志、不是插件 transcript。**崩溃恢复 = fresh-start**（ADR-0023 全量 unload + load 同一路径；在飞邮箱消息丢失诚实 = at-most-once 同族）；**正本退位**（正本 = 部署文件 + 插件私有存储）。
- **D76 公共字段七件**：`seq`（每租户单调）/ `wall_ms` / `kind`（封闭枚举）/ `fiber`（`{slot, generation, plugin}`，host 条目 = null）/ `cause`（触发条目 seq）/ `payload` / `redacted`（脱敏路径 + 策略版本）。**代码版本 = 锚点条目携带制**：`boot` 带 host_version + 配置树 hash；`fiber_state_changed` 进 Active 带 plugin_revision + config hash（实例件 + 声明 hash）；审计按 `seq` 向前找最近锚点，live-apply 漂移 = join 沿途 `config_committed`。
- **D77 自由态 Int = 包裹对象** `{"$int": "<十进制>"}`（已知 kind 的声明字段按 schema 感知还原，不包裹）。
- **D78 kind 表骨架**：生命周期 / 能力使用（仅 `process_spawn`/`process_exit` 两条）/ 错误现场 / 控制四族**写死全录**（低频 + 「藏不住」两面；控制族 = 审计自保同族）；`mount`/`unmount` 进 journal（元权限动作审计藏不住）；消解 7 条（`external_input`/`fork_created`/`session_*`/`llm_*`/`snapshot_written`/`nondeterminism_read`/`replay_diverged`）；撤销 16 条留档（能力族其余十条 + 内部流量族四条 + `window_summary` + `audit_policy_changed`）。kind 表 = payload schema 的 SSOT。

### J2 撤销留档（D79–D83，扩展位「可配审计层」预制方案）

可配审计面整体不立：`core.audit` 六键（`{http, fs, sql, timer, stream, internal}`）/ 三档（缺省六族全 `summary`，结构有界原则：默认面 = O（窗口数））/ `window_summary` 聚合形态（聚合窗 = dispatch 窗口 + `dispatch_completed` 增强）/ 异步写入管线（每租户 writer + 有界通道 + 丢弃制诚实降级 + `journal_gap`）/ 分级 sync（写死族落盘即 fsync、可配族周期 flush）。**存活面**：写失败 = 降级续跑 + host 告警 + 恢复补 `journal_gap`（fail-stop 淘汰——正本退位）；优雅关闭 drain 排干义务。重开触发器 = 首个真实审计需求或安全事件。

### J3 范围重定与崩溃恢复（D84–D88）

- **D84 v1 范围收缩 = 写死族骨架**：写入模型塌缩 = intercept 点同步构造 + 序列化 + append + **逐条 fsync**（无通道无 writer 无背压；seq 由写入点单点分配，文件序 = seq 序）；`journal_gap` 收窄 = 写失败恢复标记。防线 2 收窄结案：v1 默认面 = 管理动作 + 错误现场 + process 可见，能力调用细节不立记录面（不一致写实：`curl` 外泄可见、`ctx.http` 不可见）。D23.3 审计写实死亡（内部流量族不立；analyze #7 静态面原位）。
- **D85 崩溃自证**：`shutdown_clean`（drain 末尾该租户最后一条 + 最终 fsync）+ `boot` 条目 `previous_run: clean | unclean | first_boot`（判定三分；多段精确化 = D90 兼容注）；unclean = host 日志 warn 一条。插件侧无 core 信号（需崩溃信号的插件自建心跳文件）。
- **D86 在飞丢失写实**：邮箱未投递 + 在飞 handler 丢失诚实；对外副作用不回滚；journal 自身至多丢正在写的一条；不丢 = 部署文件 / 待审批提案 / 插件已落盘私有状态。**崩溃孤儿漏洞写实**：drain 不运行则 spawn 子进程可能残留，v1 不承诺崩溃时无孤儿（OS 级随父死机制 = pdeathsig / Job Object 登记实现期）。
- **D87 插件持久化与实例复活结案**：核心零新机制——插件跨重启状态 = 插件自建 fs/sql；**实例不自动复活**（挂载方 setup 重跑重 mount，天然幂等；自管档声明不复活）；agent 连续性 = 插件层。
- **D88 崩溃窗口定稿**：已 fsync = 存在、正在写 = 不曾存在；撕裂尾行一切读取侧静默丢弃（崩溃是常态不是腐败；不搞双写/校验和自保 = 正本退位）。

### J4 写入侧工程（D89–D91）

- **D89 骨架脱敏 = 注册秘密值精确匹配扫**：一切条目 append 前统一过扫（payload 字符串值；map 键不扫）；注册集 = secrets load/reload 管线按身份注册的秘密值（每租户一份）；全值精确匹配、串内子串替换为 `[redacted:<身份名>]`、长者先替；`redacted` = `[{path, policy}]`（policy = 注册集内容 hash 截断 12 hex，同身份换值也变版）；`redaction_applied` 紧随肇事条目一条（`cause` 指回，`hits` 按条目聚合；两条之间崩溃 = `redacted` 仍在、计数条目可缺）；短秘密护栏（值长 < N 不注册 + host warn；代价明写 = 真渗入时明文落盘）。v1 不立字段级可配脱敏钩子（随可配审计层扩展位）。
- **D90 段文件物理组织**：文件名 `seg-<seq_start 零垫20位>-<创建时间 UTC yyyymmddTHHMMSSZ>.jsonl`（排序键 = seq_start：字典序 = seq 序 = 时间序，时钟回拨免疫；时间戳纯展示位）；格式版本住段文件**头行** `{"v":1,"seq_start":N}`（条目体不背版本税；头行非条目，读侧跳过）；**轮转 = 尺寸阈值 T**（实现期配置；封闭段只读不可变）；**保留 = 核心内建总量封顶 B**（目录总字节超 B 删最旧封闭段至 ≤ B；活跃段永不删；B ≫ T；封顶删除只进 host tracing 不进 journal；B 账只认 `seg-*.jsonl`，归档/压缩品须移出目录）；手工面 = 封闭段随时 `rm`/`rsync`/`gzip`，唯一约束 = 活跃段不可动（误判双向写实）；**D85 判定兼容**：看最新段——有完整条目 → 最后完整条目行，头行孤段 = `unclean`（不回退读上一段尾）；seq 续接同此读（最后完整条目 seq + 1；头行孤段 = 其 seq_start；首 boot = 0 起）。消解落档：blob / 单条目字节上限（产源预算已有界）/ 写入量实测。
- **D91 数据生命周期**：生 = 首个 append 建 `journal/` + 首段；存 = D90；灭 = 随租户目录（fleet 减员 = 改目录 + 干净重启）；插件卸载/purge 不相交（journal = 租户级）；部署索引不收（扫描面 = `plugins/`）；gitignore 既定；备份/留史 = 部署者归档面自选。

### J6 replay 结案与 v1 收口（D92）

- **D92 replay 引擎 = 扩展位正式结案**。重开触发器（任一成立即重议）：① 「新代码重跑旧史」首个真实需求（自我修改审批的实证化 = 历史输入验证新版本行为差）；② agent 层 eval/确定性重放需求实锤到插件层机器不够（需 host 注入点虚拟化边界）；③ 崩溃后连续性损失被证实不可接受（fresh-start 代价从理论变痛点）。**重开时复活资产**：调研 §7 映射 replay 面（M2 边界录制 / M3 行为化发散判定 / M9 对外网关纪律）；kind 表扩族（D78 撤销记录的边界输入类 + `nondeterminism_read` 类，D7.2 生成型秘密加密议题随之复活）；wasm 盒 IO 全走 host imports = 天然可录制；wasmtime 确定性工具箱（fuel/wasi-virt/NaN 规范化，[ADR-0024](0024-carrier-layering.md) Consequences 留档）；ABI 面演进走 `abi_rev` 既定通道。机器必须长 host 侧（边界注入点是 host 的，插件无法虚拟化另一个插件的边界）。
- **J5 缓议转扩展位**：插件面 journal 查询接口（触发器 = 首个插件侧查询需求）；部署者消费 = jq/文本工具直读段文件。
- **v1 收口**：journal v1 设计范围 = 写死族骨架 + 崩溃自证 + 骨架脱敏 + 段文件工程（D75–D78、D84–D91）；扩展位三条（replay 引擎 / 可配审计层 / 插件面查询接口）不进 v1、原样存活。

### 演进纪律（冻结面的变更规则）

1. **journal 落盘格式不进 ABI**（[ADR-0026](0026-luau-abi.md) A10 节既定：自带格式版本字段）；格式演进 = additive 不 bump（新 kind / 新可选字段——JSONL 行自描述，读侧忽略未知）；破坏性（字段语义改/删、还原规则变更）= 格式版本 bump + 新 ADR supersede。新 kind 若由新 ctx 方法/新能力产生，插件可见面仍随 ADR-0026 纪律（additive → `abi_rev`）。
2. **机制面破坏式变更**：新 ADR supersede（点名取代本 ADR 正文格）。
3. **living doc 保持 living**：细则澄清、教学条、实现期登记继续沉淀于 [journal](../design/journal.md)；凡改语义的修订不允许只走 doc 修订注，必须按本条 1/2 走。
4. 各站登记的扩展位与重开触发器以 doc 各站联动登记汇总表为 SSOT；触发器到来时按本条 1/2 走。

### 对既有 ADR 的局部 supersede（同批吸收）

各旧 ADR 正文冻结不动、Status 行记录本条；以下逐条命名被取代/兑现/结案的确切条款：

1. **[ADR-0026](0026-luau-abi.md)** A6 节「journal 记录格式草案」条——supersede：录制粒度「**★ 族（replay 必需）全录 + audit 族 v1 默认降档**（可配开/聚合摘要；`process_spawn`/`process_exit` 不降档写死）+ 大 body 阈值 blob 引用」与「生成型秘密 = kind 感知加密按 source 子类分流（`crypto.random` 系加密落盘 / 时钟明文）」全撤——v1 = 写死族骨架（生命周期 + 错误现场 + 控制 + `process_*` 全录）；blob 不立；生成型秘密加密随 `nondeterminism_read` 不立消解。「服务对象三层 = 核心崩溃恢复 / 插件层平台能力 / 部署者审计」改读：① = 崩溃**诊断**（fresh-start，journal 不参与状态重建）；② = 缓议转扩展位；③ 收窄 = 管理动作/错误现场/process 可见。存活原位：「核心自己的飞行记录仪」定位 / 「四需求按层重分配（fork 退出核心需求集 = 插件层功能）」/「kind 表去 agent 化」/「payload 落盘 = JSONL + 格式版本字段 + Int 十进制字符串惯例（读侧 kind/schema 感知还原）」。同 ADR Consequences「journal 记录格式草案（A6 节 A6.7）= 任务 6（journal/replay 正式设计）的头号输入」结案 = 本 ADR（任务 6 已收官冻结）。
2. **[ADR-0026](0026-luau-abi.md)** A4 节调度条括注「（replay 免录恢复顺序）」与「预算数值 = 调度第四输入，replay 期必须用录制期值（钉版）」、A6 节 Rust 侧表示条括注「（确定性迭代序，journal/replay 白送）」——措辞失效注：replay 引擎不立（本 ADR D92）；调度规则纯函数化与 `BTreeMap` 决策本体不动（确定性动机自立，非 replay 专需）；钉版括注随 replay 消解（预算数值 = 实现期配置不变）。
3. **[ADR-0025](0025-seam-capabilities.md)** §1 非确定性源处置条括注「（journal record 族）」「（per-fiber 种子，replay recompute）」「安全级（`crypto.random`）= CSPRNG/record」、§5 timer 格「replay 按史注入」、§5「journal kind 映射（syscall 部分）」格全格、§5「replay 分族原则」格全格、§5 时间调度四梯度格「replay 中的时间 = 按史注入」——supersede/措辞失效：宿主受控时钟 / 确定性 PRNG / timer 原语决策本体不动（确定性沙盒动机自立）；journal record 族与 replay 分族政策消解；journal kind 表 SSOT 移交 journal.md D78（syscall 侧存活 = `process_spawn`/`process_exit` 两条全录写死）；私有状态族 replay 政策 = 无 replay 则无政策。Consequences「journal kind 表 syscall 部分与私有状态族 replay 政策 = journal 正式设计（任务 6）输入」结案 = 本 ADR；「audit 族降档开关面 = 配置键，归任务 4」经 ADR-0027 批次 #12 兑现后再撤销（键面不立 = 本批 #4）。
4. **[ADR-0027](0027-config-format.md)** C8 节「audit 降档键 = `core.audit` map：**键 = audit 族名封闭集**（v1 = `fs` / `sql`；★ replay 必需族与 process 元数据不降档写死，不在可调面），**值 = `off | summary | full` 三档，缺省 `off`**；未知键/值 = 响亮报错。**overlay 可补丁 `core:` 节**（…agent 提议开/关审计窗均审批可见）。**生效 = host journal append 侧换档即时生效**，无重挂载」格——supersede：`core.audit` 键面与三档整体不立（可配审计面撤销 = D84）；「节名 `core:`」、顶层节封闭集、overlay 可补丁 `core:` 节机制原位；演进纪律第 1 条中「新 `core.audit` 族键」示例随之失效（additive 机制本身不动）。Consequences「任务 6（journal/replay）= `core.audit` 键值消费侧 + 配置变更条目政策 + `journal/` 段文件细节」结案：键值消费侧不立；配置变更条目 = `config_committed`/`config_rejected` 写死族（D78）；段文件细节 = D90/D91。
5. **[ADR-0028](0028-instance-model.md)** S4 节「跨实例服务调用 = 内部流量结案」格的「`service_call` 族 replay 政策（重算不注入）不动」半句——失效注：replay 不立（D75）+ 内部流量族撤销（D84）；「D68 边界下跨实例调用结构性不存在」与「扩展位（跨租户共享实例 / handle 服务出口）开放时重开」原位。Consequences「任务 6（journal/replay）= journal 每租户一本（S2）+ 条目不押实例字段（S4）+ 跨实例调用 = 内部流量（S4）」结案：全部吸收（每租户一本 = D75/D76；不押实例字段 = D72；内部流量族随 D84 撤销）。
6. **[ADR-0024](0024-carrier-layering.md)** §6 防线 2「**行为透明**：每次跨界调用进 journal，可回放、可归因——代码不可读，行为藏不住。」——收窄 supersede：v1 兑现面 = 管理动作（boot/挂载/配置提交）+ 错误现场 + process spawn 可见；能力调用细节（读哪些文件、联系哪些域名）v1 不立记录面；「可回放」随 replay 不立同死；不一致写实（`curl` 外泄可见、`ctx.http` 外泄不可见）；逐调用能力审计 = 扩展位「可配审计层」预制方案（触发器 = 首个真实审计需求或安全事件）。「代码不可读，行为藏不住」原则原位（收窄的是默认记录面，不是信任模型）。Consequences 议题地图回填行「**journal/replay 设计**（host imports 全量可拦截 + wasmtime 确定性工具箱：fuel/wasi-virt/NaN 规范化）」结案 = 任务 6 已收官（本 ADR）；确定性工具箱随 replay 不立转扩展位资产（D92 重开时复用）。
7. **[ADR-0023](0023-core-restart-semantics.md)** Consequences 派生登记条「**跨重启状态政策**——进程死则 VM 全灭，什么状态落盘活下来（落盘 seam 原子写）、agent 自身连续性（进行中的对话、待审批）如何恢复，与 journal/replay 设计强耦合」——结案：fresh-start 面收官 = D84–D88（journal 不参与状态重建；插件持久化 = 插件自建 fs/sql；崩溃自证 = `shutdown_clean` + `previous_run`；实例不自动复活）；同条「优雅关闭细则」半句原位（journal 侧义务 = drain 末尾 `shutdown_clean` + 最终 fsync = D85）。

## Consequences

- journal 六站（J1–J6，D75–D92；J5 缓议转扩展位）自此冻结；全文 rationale、淘汰选项、议题地图与示例以 living doc 为准，doc 头部状态行已指向本 ADR。
- **ABI 面影响 = 零**：journal 落盘格式不进 ABI（ADR-0026 A10 既定）；无新插件可见面（无新 ctx 方法/能力词/kind 对外承诺）——本次不触发 `abi_rev` bump。
- 实现期清单（骨架写入 = 同步 append + 逐条 fsync + 每租户写锁；`shutdown_clean`/`previous_run` 判定 + seq 续接；撕裂尾行读取侧丢弃；脱敏扫描管线；段文件轮转 T / 总量封顶 B / 头行写读；子进程随父死机制 = pdeathsig / Job Object）不进冻结面，归 doc 各站登记。
- 扩展位三条原样存活：replay 引擎（D92 触发器三条）/ 可配审计层（J2 预制方案全文留档；触发器 = 首个真实审计需求或安全事件）/ 插件面查询接口（J5；触发器 = 首个插件侧查询需求）。
- 后续任务输入：任务 10（实现）= 事件系统含 journal 拦截点 + 上述实现期清单（本冻结面为 SSOT）；默认插件集议题的 schedule/shell tool 参考实现面不受影响。
- 本次同批更新：ADR-0023/0024/0025/0026/0027/0028 Status 行记录批次条目号；[journal](../design/journal.md) 头部状态行转冻结指向；[luau-abi](../design/luau-abi.md) / [seam-capabilities](../design/seam-capabilities.md) / [config-format](../design/config-format.md) 相应修订注补批次号。
