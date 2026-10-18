# 0027. 配置格式冻结（C1–C9）与既有 ADR 局部 supersede 批次

- Status: accepted（任务 5（实例模型）批次已吸收 = [ADR-0028](0028-instance-model.md) 批次 #6/#7（2026-10-12：`--config` 扩展 = `--fleet-dir` 多租户 + 解析失败 fleet 细化 + secrets 同族传播；D31/D32 触发器句兑现记录 = 复审维持不立）；任务 6（journal）批次已吸收 = [ADR-0029](0029-journal.md) 批次 #4（2026-10-13：C8 节 `core.audit` 键面/三档 supersede = 不立——可配审计面撤销，`core:` 节容器与封闭集/additive 机制原位；Consequences 任务 6 行结案）；2026-10-18 局部 supersede：D44 fs scope 两档格 = [ADR-0036](0036-fs-scope-three-layers.md)（三层；单数 workspace 仍不立）；2026-10-19 再收窄：ADR-0036 Decision 第 2 条命名根局部 supersede = [ADR-0039](0039-fs-capability-two-tier.md)（现形态 = private 隐式 / `fs` 标量 global；单数 workspace 仍不立）；C1.1 `data/db.sqlite` 行失效 = ADR-0039（sqlite 能力撤回）；C1.1 布局 additive（新增 `values.yml`）+ C2.6 管线 + C6 引用语法修订（`{ secret }` → `{ $value }`）= [ADR-0040](0040-config-shared-variables.md)（统一引用，含 secret）；其余不变）
- Date: 2026-10-11

## Context

任务 4（配置格式设计）2026-10-11 四站收官（C1–C9，决策点 D29–D52）：第 1 站部署根布局 + entry 树语法、第 2 站补丁分层 + diff 协调与 reload、第 3 站 fs scope 收窄与 process 环境 + secrets store、第 4 站数据生命周期 + 核心节 + 判环预检收口。定案全文、淘汰选项、概念地基（依赖为何不住配置树 / config 为何住插件目录等）与联动登记汇总表在 [config-format](../design/config-format.md)（living doc）。前提：[ADR-0002](0002-config-as-pure-data.md)（配置纯数据）/ [0004](0004-plugin-forms.md)（插件形态）/ [0006](0006-config-schema-ssot.md)（schema SSOT）/ [0007](0007-yaml-config-subset.md)（YAML 受限子集与分层）/ [0008](0008-creator-mode-self-modification.md)（审批事务管线）；[session-model](../design/session-model.md) §3 来源分层 × 作用范围两根轴；Luau ABI 冻结 = [ADR-0026](0026-luau-abi.md)、接缝能力集冻结 = [ADR-0025](0025-seam-capabilities.md)。本 ADR 冻结配置格式的决策面，并同批吸收各站在旧 ADR 上登记的局部 supersede 意向（interim 注已在各 ADR Status 行）。

**定调（任务 4 全程）**：审批单位 = 插件目录一体（代码 + 配置一体审批；运行期区 `data/` 不进审批面）；核心不当配置保管人（core agent-agnostic 延伸）；装配单 = 名单形态（config 值不出现在 `metis.yml`）；依赖不住配置树（唯一住处 = manifest）；检查时机 = 启用期（安装期只做纯数据索引）。

## Decision

### C1 部署根布局与配置树物理组织

- **部署根 = 配置树文件所在目录**：`--config path/to/metis.yml` 选定文件即选定根（ADR-0007 机制），`plugins/`、`lib/`、`.luaurc` 全部相对它解析；不立专门 deploy-root 旗标。
- **目录骨架**：`metis.yml`（配置树正本，人类手写层）/ `overlay.yml`（机器唯一可写的配置树文件）/ `.luaurc`（`@lib` → `./lib/`）/ `secrets.yml`（secrets store）/ `lib/`（部署级契约包）/ `journal/`（段文件 + blob 预留）/ `plugins/<name>/`（`manifest.yml` 可选 / `config.yml` 部署区 / 入口与代码 / `lib/` 随包契约 / `boxes/` / `data/fs/` private 根 + `data/db.sqlite` 私有 db）。
- **私有数据同居插件目录**（淘汰集中数据根）：「一个插件的全部」= 一个目录，卸载处置与备份/迁移粒度顺手；global scope 豁免清单物理落点 = `secrets.yml` + `journal/` 两处。
- **版本管理纪律**：部署根可整体入库；gitignore = `secrets.yml`、`journal/`、`plugins/*/data/`。
- **`.luaurc` 布线**：核心启动检查部署根 `.luaurc`——不存在则自动生成（固定内容，幂等）；存在则校验 `@lib` alias，不符响亮报错指路 `metis sdk`。
- **配置树单文件制**，不立 include 机制（扩展位，触发器 = 真实文件拆分/分发需求）。

### C2 entry 树语法

- **顶层形状 = `plugins` map，key 即身份**；entry 值 `null` ≡ `{}` 归一；key = 合法目录名；加载顺序 inject 拓扑驱动，声明序无语义。已知牺牲：Cordis `id` 的同插件多实例（v1 = 复制目录；复审触发器 = 首个真实撞名需求或任务 5 子树，同 D32）。
- **entry 字段集 = `{ enabled }` 封闭集**（缺省 `true`；`false` = 休眠件：索引收录、不加载不对账不激活）；未知字段 = 响亮报错；`isolate`/`intercept`/`group`/重命名不立（触发器 = 首个真实撞名需求或任务 5 子树，同批复审）。
- **顶层节封闭集 = `{plugins, core}`**；未知节 = 响亮报错；演进 = 新节 additive + 老核心响亮自然兼容，不立版本号机制。
- **休眠件运行期不提醒**（显式声明态，审批 diff 可见）；**未装配件运行期不吵**（`sdk check` 提示级列出）；**部署索引 = `plugins/` 目录扫描全量**（休眠件/未装配件/残迹带状态标记；解析失败 = 索引标记 + host warn 一条、登记启用时响亮报错）；inject 未装配件 = 启用期响亮，与「查无」区分指路。
- **插件形态 = 目录单形态**：一切插件 = 目录 `plugins/<name>/`；manifest 可选（无 = 零仪式插件，§A1.3 差异面平移）；三类文件规则 = 包内容（升级替换、分发内容）/ 部署区 `config.yml`（升级保留、分发排除）/ 运行期区 `data/`（升级保留、分发排除、gitignore）；审批事务 diff 视野排除 `data/`。
- **配置通道 = `config.yml` + 注入签名不变**：`setup(ctx, config, deps)` 不动；启用期全管线 = 校验（缺 required/类型错/多给键 = 响亮失败，插件代码一行未跑）→ 秘密引用解析（真值只进属主 VM）→ 默认值合成 → 冻结表注入；休眠件的 `config.yml` 不校验；无 `config:` 节 ≡ 空 schema（存在且有键 = 多给键响亮拒绝）。
- **reload 权利归插件**：`config.yml` 变更默认重挂载（安全缺省）+ 通知否决通道（C4）；人直接编辑文件 = 下次 boot 生效；fs watch = 扩展位。
- **依赖不住配置树**：依赖 = 作者意图，唯一住处 = manifest（`inject` 双列表 + `events` 双名单）；事件 listen ≠ 依赖。

### C3 补丁分层

- **overlay 与 metis.yml 同 schema**（同 parser 同校验器）；**合并 = 按 key 字段级稀疏覆盖**（生效树 = base ∪ overlay；提到的 key 按字段赢，未提到零意见）；不立 Cordis 补丁词汇。
- **卸载语义 = `enabled: false` 墓碑 + 审批事务处置目录**（处置粒度见 C7）；不立删除语法（`null` 已占用、墓碑可逆留痕、Cordis 先例平移）；**机器对 overlay.yml 整文件所有权**（规范形重写自由），永不碰 `metis.yml`。
- **机器写 overlay 唯一时机 = 审批事务提交**；落盘 = tmp+rename 原子替换 + 串行写队列；读-改-写在事务内；**boot 解析失败 = 响亮拒启动**（配置树 = 装配单，读不出不猜着跑）；debounce 不立。
- **`config.yml` 机器写 = 最小 diff 注释保真纪律**（未触碰区域字节级保留；受限子集使行块级补丁器可实现）；退档 = 顶层键块整体替换（审批 diff 事先显形注释损失）。

### C4 diff 协调与 reload 机制

- **生效树 diff 两时机**：boot = 全量无 diff；审批事务提交 = 新旧生效树 diff。**逐 key 动作表**：key 增 = 安装 + 启用；key 减 = 卸载（幂等，目录已不在 = 仅摘树）；`enabled` 翻转 = 卸载/启用；`core` 节变更 = host journal 换档即时生效，不涉插件重挂载。失败隔离 = 逐 entry 非事务（失败者落 Failed + `internal/plugin` 事件；依赖方走 Pending + warn 机器）；同批顺序 = 先拆后建，建设半相 inject 拓扑驱动。边界：`config.yml` 变更走下条；目录内容变更 = 安装事务管线。
- **`on_config` 通知通道**（入口 table 可选字段，A1.1 封闭集 additive 扩展）：新 `config.yml` 先过全管线，**校验失败 = 拒绝变更**（运行中实例持旧配置不动 + 响亮 surface）；结构相等 = no-op；handler 收解析后冻结全量新 config，**返回 `true` = live-apply 成交，其余一切（缺 handler / 返回非 true / 抛错）= 重挂载**（卸载排空 + 重启用，消费者 epoch 重启既有机器）；不立 transform 返回与永久否决（文件 = 真相）；竞态 = 合并取最新、stale 成交不结案；秘密纪律按构造成立（handler 住属主 VM，无广播泄露面——`internal/update` 不开，见批次 #10）。
- **部署 `lib/` 契约增删改重对账 = boot / 显式 reload 触发**，级联机器复用（diff 契约命名空间 → additive 吸收 / 形状变更 = 反向索引级联重启用）；agent 提议部署根 `lib/` 变更 = 任务 9 扩展位。

### C5 fs scope 与 process 环境

- **fs scope = `{private, global}` 两档**，workspace 与绑定机制整体不立（单数 workspace 无规范指称；「插件需要某目录」= 插件 `config.yml` 路径值 + global 能力已覆盖）；中间层唯一形态 = 命名根扩展位（词汇 + 绑定机制同批走 ABI rev）。代价写实：fs 审批两档化；偏执部署走 OS 级沙箱（外墙归部署组成，核心不立机制）。
- **process env = 继承核心 env + 覆盖，不立收紧层**（挡不住同用户 `/proc/<pid>/environ` = 假边界；secrets 不住 env 既定）；触发器 = 多租户/托管形态。
- **cwd 缺省 = 用户家目录**（落选私域 = 假 confinement 暗示）；家目录不可解析 = 响亮报错指路显式 `cwd`；显式 cwd = 任意绝对路径；相对 cwd 相对家目录解析；cwd 不存在 = spawn 自然失败不预检。

### C6 secrets store

- **`secrets.yml` = 扁平 map，值仅字符串**（多行 = PEM 类正例；结构化秘密拆多个名字）；**部署级共享命名空间**（无 per-plugin 隔离；撞名 = 同值无害）；文件缺席 ≡ 空 store；引用查无 = 启用期响亮；**淘汰静态加密 at rest**（主密钥住处无解；触发器 = 托管/合规需求）。
- **纪律**：权限锁 = boot 检查，group/other 可读 → warn 不拒启动（容器挂载常 0644，拒启动太脆）；加载 = boot 一次读入内存；轮转感知 = 显式 reload / boot（重读 → 受影响插件重跑 config 管线 → D42 通道）；**机器永不写**（agent 无提议秘密变更通道）；**解析失败两分** = boot 响亮拒启动 / reload 拒本次持旧 map。

### C7 数据生命周期

- **卸载处置 = 两档可选、缺省保留**：**keep（缺省）** = 包内容移除，`config.yml` + `data/` 保留为残迹；**purge** = 目录整体移除。**档位选择归部署者**（卸载动作发起时）；**agent 提议卸载固定 keep 档**（删数据不进 agent 提议面）；残迹的事后 purge = `sdk purge` / 手工删除（部署者专属）。理由：持久意图数据不可再生（「意图数据持真相」），删数据 = 显式动作不是移除代码的副作用；禁用 → 卸载 → purge 逐级加强。
- **残迹索引政策**：残迹 = 目录内无 manifest 且无入口文件、但有 `config.yml` 或 `data/` 残留；部署索引收录标记；**运行期不吵**；`sdk check` 提示级列出 + purge 指路；inject 面无特殊 casing（残迹无 provides，依赖方走「查无」同族响亮）；**同名重装 = 复活**（`config.yml`/`data/` 自然接上；同名新插件继承 = sdk/审批面提示「含前任残迹」）；空目录索引不收录；残迹的 `config.yml` 不校验。

### C8 核心节

- **节名 `core:`**；audit 降档键 = `core.audit` map：**键 = audit 族名封闭集**（v1 = `fs` / `sql`；★ replay 必需族与 process 元数据不降档写死，不在可调面），**值 = `off | summary | full` 三档，缺省 `off`**；未知键/值 = 响亮报错。**overlay 可补丁 `core:` 节**（同 schema 稀疏合并不豁免；agent 提议开/关审计窗均审批可见）。**生效 = host journal append 侧换档即时生效**，无重挂载。
- **一等 cron 复审结案 = 维持不立**：周期意图 = config 键约定（`config.yml` 时代完整保留）；一等 `schedule` 字段 = 第三台调度机器的原始理由不动摇；梯度二持久调度 = 插件层议题（schedule 插件立项归默认插件集批次）；触发器 = schedule 插件立项遇 config 键表达不了的声明式调度需求，走 ABI rev additive。

### C9 判环预检兑现

- **analyze #8 数据源精确化**：读部署根树文件（`metis.yml` ∪ `overlay.yml` = 生效树）拿 **enabled 集**；**边 = manifest `inject` required 列表**（唯一住处）经 provides key 解析到提供方。
- **图构造**：节点 = 树内 enabled 插件（休眠件/未装配件不入图）；provides key 查无/提供方休眠 = 边悬空不参与判环（悬空 = 启用期 Pending/响亮既定面）。级别维持**炸**。
- **命令形态**：`metis sdk analyze` 在部署根运行自动含 #8；单插件目录模式跳过。

### 演进纪律（冻结面的变更规则）

1. **配置树演进 = additive**：新顶层节 / 新 entry 字段 / 新 `core.audit` 族键 / overlay 同 schema 延伸，老核心遇未知节/字段响亮报错 = 不静默误读（版本号机制的检测职能多余）；**不立版本号机制**——配置是部署者文件、非跨边界 ABI，无 `abi_rev` 对应物（ABI 面演进走 ADR-0026 纪律）。
2. **破坏式变更**：新 ADR supersede（点名取代本 ADR 正文格）。
3. **living doc 保持 living**：细则澄清、教学条、实现期登记继续沉淀于 [config-format](../design/config-format.md)；**凡改语义的修订不允许只走 doc 修订注**，必须按本条 1/2 走。
4. 各章登记的扩展位（include 机制 / fs watch 实时感知 / isolate·intercept·group·重命名 / 命名根 / env 收紧 / 静态加密 at rest / agent 提议 `lib/` 通道等）与复审触发器以 doc 各章联动登记汇总表为 SSOT；触发器到来时按本条 1/2 走。

### 对既有 ADR 的局部 supersede（同批吸收）

各旧 ADR 正文冻结不动、Status 行记录本条；以下逐条命名被取代/兑现/结案的确切条款：

1. **[ADR-0002](0002-config-as-pure-data.md)** Context「keyed diff 的相等性语义平凡；`volatile` 快路径干净」条——取代：配置值 keyed diff 机器与 `volatile` 快路径随配置值移出配置树退役（本 ADR C2/C4）；替代语义 = 生效树 diff 协调 + `on_config` 定向 hook。Consequences「`!!luau` 复活时配套机制」行的 `internal/update` 半句改指批次 #10。Decision 纯数据纪律不变。
2. **[ADR-0004](0004-plugin-forms.md)** Decision「两种插件形态」表与「统一插件名解析」四条规则——取代：合并为目录单形态（本 ADR C2），名字解析缩为「目录存在即可」（歧义格按构造消失）；Consequences 成长路径「`foo.luau` → `foo/`」平移为「加 `manifest.yml`，入口文件不动」。三层概念分离表（module/plugin/package）不变。
3. **[ADR-0006](0006-config-schema-ssot.md)** Context「`volatile` 等价标注（loader 快路径）」、Decision 示例 `volatile: true` 行、Consequences「`volatile` 快路径由 loader 消费」——取代：`volatile` 标注退役（哪些键可热应用 = 作者文档 + `on_config` handler 自判，本 ADR C2/C4）。另注明：config 值住处 = 插件目录 `config.yml`（本 ADR C2；ADR-0006 正文原无显式值住处条款，当时默认配置树）。schema SSOT 与「校验先于插件代码运行」不变。
4. **[ADR-0007](0007-yaml-config-subset.md)** Decision「机器序列化输出只落 agent overlay 层；人类手写层永不被机器重写 → comment 保留问题架构性消解」——取代：机器写面 = overlay + 插件目录 `config.yml`（经审批事务）；`config.yml` = 双作者文件，注释保真纪律重立（最小 diff + 退档顶层键块整体替换，本 ADR C3）；overlay 收缩为装配补丁层；`metis.yml` 人类层仍永不被机器写。
5. **[ADR-0008](0008-creator-mode-self-modification.md)** Decision 管线「配置补丁」落点与 Consequences「模型可见的配置写入面只剩 agent overlay 层」——取代：模型配置写入面 = 插件目录 `config.yml` 经审批事务（审批单位 = 插件目录 diff，代码 + 配置一体）+ overlay 装配补丁（本 ADR C2/C3）。
6. **[ADR-0026](0026-luau-abi.md)** A2 节字段形状条「`fs` = scope map `{private, workspace, global}` × `read|write`」——取代：scope 键收窄 = `{private, global}`（本 ADR C5）。
7. **[ADR-0026](0026-luau-abi.md)** A3 节顶层组织条效应命名空间（`fs`/`workspace`/`global`/…）与 fs 条「三 scope 对应三命名空间（`ctx.fs`/`ctx.workspace`/`ctx.global`）」——取代：两 scope 两命名空间（`ctx.fs`/`ctx.global`），`ctx.workspace` 移除（本 ADR C5）。
8. **[ADR-0026](0026-luau-abi.md)** A3 节 process 条的运行环境细部（living doc §A3.10：「cwd 缺省 = 绑定的 workspace 根，未绑 = 私域」「env 收紧政策归任务 4」）——取代：cwd 缺省 = 用户家目录（不可解析 = 响亮报错指路显式 cwd）；env 收紧结案 = 不立（本 ADR C5）。本条命名 living doc 节（cwd/env 语义不住 ADR 正文），按演进纪律 #2 通道经本批吸收。
9. **[ADR-0025](0025-seam-capabilities.md)** Decision §5 fs 条「三 scope 与两段式路径解析」与 process 条「fs 三 scope 与两条豁免」表述——取代：收窄为两 scope（本 ADR C5）；同条「外墙 = OS 级沙箱归部署组成」结案确认（核心不立机制）。
10. **[ADR-0016](0016-internal-hooks.md)** Decision 表 `internal/update` 💤 行——兑现：配置更新否决职能以定向 hook（入口 table `on_config` 字段）实现（本 ADR C4）；钩子本身维持不开；`internal/config` 仍随 `!!luau` 复活处置（ADR-0002）。
11. **[ADR-0024](0024-carrier-layering.md)** §5「在途调用排空细节归插件热更新设计」——兑现：重挂载复用 §A4.5 卸载排空路径（本 ADR C4）；盒与属主同生共死原样。
12. **[ADR-0025](0025-seam-capabilities.md)** Consequences「audit 族降档开关面 = 配置键，归任务 4」与 **[ADR-0026](0026-luau-abi.md)** Consequences「配置树绑定、secrets store 细节、audit 降档开关面归任务 4」——兑现/结案：audit 降档开关面 = `core.audit` 三档枚举（本 ADR C8）；secrets store 细节 = 本 ADR C6；配置树绑定 = 不立（本 ADR C5）。
13. **[ADR-0026](0026-luau-abi.md)** A2 节「周期调度不立一等 cron 字段」的复审登记——结案：复审维持不立（本 ADR C8；触发器 = schedule 插件立项遇 config 键表达不了的声明式调度需求）。
14. **[ADR-0026](0026-luau-abi.md)** A9 节 inject 缺失三分条「已安装未启用 = Pending 等人为动作 + warn 提醒」格与部署索引条「安装期 = 纯数据部署索引（…含休眠件）」——收窄/细化（本 ADR C2）：「已安装未启用」收窄 = **休眠件**（树里有 entry、`enabled: false`）专属；「目录在、树无 entry」的未装配件改判 = 索引查无同族启用期响亮 + 指路区分（「目录存在但树无 entry」）；部署索引覆盖精确化 = `plugins/` 目录扫描全量（休眠件/未装配件/残迹带状态标记；解析失败 = 索引标记 + host warn 一条、登记启用时响亮报错）。

## Consequences

- 配置格式九章（C1–C9）自此冻结；全文 rationale、淘汰选项、概念地基（依赖不住配置树 / config 住插件目录 / 持久调度四件套衔接等）与代码示例以 living doc 为准，doc 头部状态行已指向本 ADR。
- 实现期清单（部署索引目录扫描含残迹识别 / tmp+rename 原子写与串行写队列 / 行块级 YAML 补丁器 / 生效树 diff 器含 `core` 节换档 / config 管线重入与结构相等比较 / 卸载两档处置 / `core` 节解析校验 / analyze 部署级判环模式 / `.luaurc` 自动生成与校验 / gitignore 脚手架 / `lib/` 重扫级联 / secrets 内存 map 与 reload 重读管线）不进冻结面，归 doc 各章登记。
- 后续任务输入：任务 5（session 模型）= per-session/preset 实例级配置通道张力登记；任务 6（journal/replay）= `core.audit` 键值消费侧 + 配置变更条目政策 + `journal/` 段文件细节；任务 9（marketplace）= 包内容 vs 部署区/运行期区三分定稿 + agent 提议 `lib/` 审批通道。均以 doc 联动登记汇总表为 SSOT。
- 本次同批更新：ADR-0002/0004/0006/0007/0008/0016/0024/0025/0026 Status 行记录批次条目号；[config-format](../design/config-format.md) 头部状态行转冻结指向。
