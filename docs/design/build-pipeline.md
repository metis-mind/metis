# Metis — 构建管线设计（living doc）

> 状态：living design doc——2026-10-18 开章（T13-e 构建管线议题批拍板写入：十一项定案 = D105–D115）。本文档 = 构建管线（`metis new/check/build/dev`、构建缓存、脚手架、自扩展快车道与两条服务化扩展位）的 SSOT；ABI 层 = [wit-abi](wit-abi.md) W1–W4/W10（§14 登记），作者面 = [plugin-sdk](plugin-sdk.md)，安装事务 = T10.12。
> 前提（已定不议）：插件唯一载体 = wasm 组件、作者不见 WASI/WIT（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §1/§4）；部署根三区（[config-format](config-format.md) D29/D35）；实例即弃与热换语义（[wit-abi](wit-abi.md) W4 D93–D97 + [ADR-0010](../decisions/0010-fiber-core.md)/[0011](../decisions/0011-actor-execution-model.md)/[0023](../decisions/0023-core-restart-semantics.md)）；审批门 v1 接口预留（[ADR-0008](../decisions/0008-creator-mode-self-modification.md)）。
> 证据基线：[build-pipeline-survey](../research/build-pipeline-survey.md)（dev loop / 构建缓存 / 远端构建场三路先例调研，2026-10-18）；[wasm-research](../research/wasm-research.md) 补遗 1、6（实例化 1.7 µs = 补遗 1 + [ADR-0024](../decisions/0024-carrier-layering.md) §8；S1 增量重建 0.22–0.29 s = 补遗 6）；[wasm-first-design-review](../research/wasm-first-design-review.md) B9。
> Last reviewed: 2026-10

---

## 0. 术语锚点

本文档新增（共享术语见 [config-format](config-format.md) §0.1 / [wit-abi](wit-abi.md) §0 / [plugin-sdk](plugin-sdk.md) §0）：

- **源码工程（source project）**：插件作者写代码的目录 = `Cargo.toml` + `src/` + `manifest.yml` + 工具链钉版文件；产出可部署包，不是部署根的一部分。
- **部署根（deployment root）**：一个 Metis 部署的全部内容所住的目录（[config-format](config-format.md) D29）；`plugins/<name>/` 只放包内容、部署区与运行期区。
- **可部署包（deployable package）**：`metis build` 的产物 = `manifest.yml` + 组件 wasm（+ 可选元数据）；安装事务的唯一输入。
- **构建指纹（build fingerprint）**：源码树 hash + 解析后依赖图 + 工具链指纹 + SDK/WIT/ABI 版本 + target/profile/features 的规范化摘要；缓存 key 与跳过判定的依据。
- **产物元数据（artifact metadata）**：追加在组件上的 `metis.build` custom section；记录构建指纹、SDK/ABI 版本、target/profile 等确定性字段。
- **分发 lane**：源码仓 → CI / 构建 → 签名包 → marketplace / 部署根；面向第三方分发。
- **自扩展快车道（self-extension fast lane）**：agent / Creator 生成源码 → 同一构建管线 → 本地信任档安装激活；不经过 git / CI / marketplace / 签名。
- **本地信任档（self-authored local）**：自写自用的信任分级（[ADR-0024](../decisions/0024-carrier-layering.md) §6 防线 4）；零审批不等于零能力门。

## 1. D105 范围与产出（2026-10-18 定案）

**D105 定案：v1 = 本地构建闭环；两条服务化通道登记为扩展位。** 构建管线的 v1 面 = `metis new`（脚手架）、`metis check`（预检 + 数据/类型校验）、`metis build`（可部署包）、`metis dev`（watch → 构建 → 校验 → 安装 → 热换）、构建缓存、自扩展快车道（同机形态）。部署内工具链服务与远端构建场 = 扩展位，只冻结安全边界、接口 schema 与触发器，不实现（与 T10.15 站行「后两件不进 v1 面」一致）。

- **不归本件**：宿主自身源码的增量构建/重启（核心开发走 `cargo run`）；fleet / 多租户 dev；marketplace 发布流程本体；配置树编辑与 `config.yml` 的 fs watch（[config-format](config-format.md) D37 扩展位）；Rust 之外语言 SDK 的本体（语言适配器只留扩展点）。
- **产出**：本文件 + [build-pipeline-survey](../research/build-pipeline-survey.md)；T10.15 以本文件为开工输入，实现期再走 `adding-dependencies` 与门禁面流程。
- **性质**：设计站，不新增 crate、不新增依赖、不改门禁面；F 类窗口预登记 = §15（T10.15 收官时迁入 [testing-strategy](testing-strategy.md) §3.F）；D11 / cov-floor 是否适用随 T10.15 新增 crate 的定位在开工时判定（内核 crate 才触发 station-run 义务）。

## 2. D106 两源模型、命令面与 git 拓扑（2026-10-18 定案）

**D106 定案：源码工程与部署根两源分离；`metis new` 不在部署根的 `plugins/<name>/` 内创建源码。** 部署根 `plugins/<name>/` 的语义已由三区纪律定死（[config-format](config-format.md) D29/D35）——包内容（manifest / 组件）、部署区（`config.yml`）、运行期区（`data/`）。源码、`Cargo.toml`、`target/`、`.metis/` 都不是这三类，塞进去会同时破坏卸载/升级边界、gitignore 边界与构建缓存隔离。

**拓扑三形态**（工具都支持，推荐 A）：

- **A. 独立源码仓 + 独立部署仓（默认）**：插件源码一个仓；部署根一个仓（或运维仓的子目录）；`metis dev --config <path/to/metis.yml>` 把包内容装进部署根。
- **B. 单仓 monorepo（允许）**：顶层 `plugins/<name>/`（源码工程）与 `deploy/`（部署根）**并列**；部署根内部另有 `deploy/plugins/<name>/` 包内容。同一个 git 历史，仍是两个根；不要把源码放进 `deploy/plugins/<name>/`。
- **C. marketplace 分发（长期，接口归 D114）**：源码仓发布签名包；部署根按版本/hash 安装，不保存源码。

**git 边界**：

| 位置          | 跟踪                                                                                                      | 忽略                                                     |
| ------------- | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| 源码工程      | `Cargo.toml`、`Cargo.lock`、`src/`、`manifest.yml`、`rust-toolchain.toml`、`.cargo/config.toml`、`README` | `target/`、`.metis/`、`dist/`、编辑器临时文件            |
| 部署根        | `metis.yml`、`overlay.yml`、`values.yml`、`plugins/<name>/manifest.yml`、组件 wasm、`config.yml`          | `secrets.yml`、`journal/`、`plugins/*/data/`（D29 既定） |
| 临时 dev 部署 | 不入 git                                                                                                  | `.metis/dev/` 整个目录                                   |

- 部署根的组件 wasm 默认入库（部署即代码，可审计）；marketplace 模式可只记版本 / hash，安装时拉取。这个选择归部署者，构建管线不强制。

- `metis new <name>` 默认 `git init`（非 git 目录才初始化；已在 git 仓内则作为子目录，不嵌套建仓）；`--vcs none` 关闭；不自动 commit。构建管线**不依赖 git**——快车道可以直接在任意可写目录写源码后 `metis build --install --config <path/to/metis.yml>`。
- `metis new` 会向上查找 `metis.yml`：目标路径落在 `<deployment>/plugins/` 下时**拒绝**创建并给出指路；当前目录是部署根时警告并要求显式 `--in <source-dir>`。

**命令面**（v1）：

| 命令                | 职责                                                                                                                         |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `metis new <name>`  | 脚手架：源码工程 + manifest + 工具链钉版 + `.gitignore`；`--in <source-dir>` 指定输出目录                                    |
| `metis check`       | 预检 + manifest / WIT / ABI 校验 + `cargo check`；不产出可部署包                                                             |
| `metis build`       | 构建可部署包（默认 `dist/<name>/`，`--out` 可改）；`--install --config <path/to/metis.yml>` 直接装进该部署根（自扩展快车道） |
| `metis dev`         | watch → 构建 → 校验 → 安装 → 热换；默认 `.metis/dev/`，`--config <path/to/metis.yml>` 指向真实部署根                         |
| `metis cache clean` | 清理内容寻址缓存（大小 / 年龄）                                                                                              |

- 宿主运行命令（`metis run` / `metis serve`）的形态与归属在 T10.9/T10.12 开工时定；本文件不定义。
- 旧 `metis sdk` 组不保留：`analyze` 随 [wasm-first-design-review](../research/wasm-first-design-review.md) A8 消亡；`types` 已归编译期宏（[plugin-sdk](plugin-sdk.md) D104）；`new/check` 的职责迁到顶层命令。
- v1 一个源码工程一个插件；Cargo workspace 多插件 = 扩展位（触发器 = 首个真实多插件工程需求）。
- 可部署包内的组件文件名与包布局的 SSOT = [wit-abi](wit-abi.md) W1（**已定 = D116**：缺省 `plugin.wasm`，manifest 可指定）。

## 3. D107 dev 进程拓扑（2026-10-18 定案）

**D107 定案：`metis dev` = 单进程内嵌宿主 runtime + watcher + builder。** 构建成功后经 loader 的既有热更路径重载插件；不做控制 socket、attach 或独立守护进程（v1）。

- 理由：T10.15 的验收语义 = 保存即热换；单进程让「构建失败保留旧实例」「成功才重载」「退出即清理」由同一状态机保证。先例中 Spin 的机制是每次成功后重启整个 `spin up` 进程（见调研 §1）；Metis 有 session / timer / 插件内存态，不能照搬。
- 生命周期：启动（创建或加载部署根 → 加载既有插件）→ 初始构建（可选）→ watch → 构建 / 安装 / 重载 → 退出（停止宿主；`.metis/dev/` 保留供检查）。
- 同一部署根同一时刻只允许一个 `metis dev`：锁文件住 OS 临时/缓存目录，按部署根规范化路径的 hash 命名（不写入部署根）；所有安装/激活路径另由 T10.12 安装事务锁串行（D109）；第二个实例以明确错误退出。
- attach 模式（对已运行宿主开发）= 扩展位；触发器 = 首次「对着共享/远端部署开发」需求。

## 4. D108 构建适配与产物形态（2026-10-18 定案）

**D108 定案：v1 内置 Rust 适配器；manifest 不引入任意 `build.command`；`metis build` 输出可部署包目录。**

- **适配器选择**：按工程标记识别——`Cargo.toml` → Rust 适配器；后续语言各加内置适配器（jco / TinyGo / componentize-py 等），由 metis 统一实现。manifest 的自由 `build.command` = 扩展位（触发器 = 真实自定义工具链需求；启用时必须标记「不可缓存 / 不可远端复现」）。
- **Rust 调用**：`cargo build --profile metis --target wasm32-wasip3 --locked`（profile 定义 = D112；首次无 lockfile 可先生成，`dist` 级构建要求 lockfile 已提交）；环境变量与 `RUSTFLAGS` 由构建管线显式控制，不继承作者的随机环境。
- **构建输出目录**：`dist/<name>/` = `manifest.yml` + 组件 wasm + `lib/`（可选）+ 可选 `build-info` 诊断 sidecar；安装事务只消费可安装子集（manifest + 组件 + `lib/`，过滤 `build-info`）。组件文件名与包布局以 W1 为准。
- **manifest-less 源码**：D35 的零仪式形态保留——源码可以没有 `manifest.yml`；构建时合成最小 manifest（`version: 0.1.0` + `abi: <构建时的 WIT package version>`，无 `name`、无能力声明）；安装目录名 = crate 名 / `dist/<name>`。`metis build` 产出的包内始终有 manifest（部署根形态统一）；部署根仍接受手工放置的无 manifest 零仪式包（[wit-abi](wit-abi.md) W1.8 D123；W2 D132）。
- **SDK 依赖来源**：v1 脚手架生成指向 metis 检出目录的 path 依赖（`metis new --sdk-path <metis-checkout>`，默认按 metis 二进制位置推断），并在生成时写入 SDK 版本与 WIT 文件 hash；发布/分发形态（crates.io / git 依赖）随 marketplace 立项，触发 = 首个外部作者或分发需求。
- **校验阶段**（构建与安装之间，失败即停）：
  1. 校验 SSOT = 进程内 `wasmparser`（固定 feature 集合）；`wasm-tools` 只作 WIT 提取 / 调试，版本钉定，不参与安装判定；
  2. 组件 import 清单机械提取 → 与宿主词汇表对账（import 清单 ⊆ 词汇表，安装期结构事实 = W9 D180–D182）；manifest 能力声明是运行期门，不参与 import 对账（[wit-abi](wit-abi.md) W2.9 D133）；
  3. export / world 与 manifest 的 ABI 声明一致；WIT package version 接受语义归 W10 D187（三合一：声明 / 结构 / 词汇表）；
  4. 资源上限（初始/最大内存、表、组件大小）与禁用特性（threads / shared memory / cm-threading）检查；**v1 拒绝 start 函数与命令类导出**（`wasi:cli/run` / `_start`；start 在激活门与加载预算之前执行；政策已定 = [wit-abi](wit-abi.md) W1.7 D122，不再重裁）；
  5. `metis.build` 元数据在场且可解析（D112）；追加前先剥离源码自带的同名 section，校验恰好一条构建产物。
- **代码归属**：构建管线逻辑住独立库 crate（建议 `crates/metis-build`，T10.15 开工确认）；`metis` 二进制只做 CLI 壳。纯校验 / import 提取住共享 crate（建议新建 `metis-validate`；若最终并入 `metis-component`，依赖方向 = `metis-build` / `metis-loader` → `metis-component`，`metis-component` 不反向依赖构建管线）。运行时 crate 不依赖 `metis-build`。不把 IO / 进程调用塞进 `metis-loader`（纯度边界，[ADR-0019](../decisions/0019-crate-layout.md)）。
- v1 一个源码工程一个插件；workspace / 多插件构建 = 扩展位（D106）。

## 5. D109 安装、重载与失败语义（2026-10-18 定案）

**D109 定案：构建到 staging → 校验 → 调 T10.12 安装事务（含激活/重载边界）→ 失败时旧版本恢复；不触碰运行中实例与旧产物。**

- **顺序**：`metis dev` 把可部署包写入部署根的 staging 区（与包内容同文件系统的临时路径）→ D108 校验 → T10.12 安装事务 → 激活 / 重载（[wit-abi](wit-abi.md) W4.5 D97 的关闸 → 优雅窗口 → `dispose` → 实例即弃 → drain + 重启用；dispose 细节 = W1.6 D121）。**事务边界由 T10.12 开工拍板**：合法形态二选一——① 激活/重载是事务的最后一阶段（新实例 `setup` 成功才提交）；② 事务只提交包内容，激活在外，失败时必须补偿重装备份包并重启旧版本。本文件只冻结结果要求：**系统绝不半重载**，失败后旧版本恢复。
- **失败矩阵**：
  - 构建失败 / 校验失败 = **不安装**；旧可部署包与运行中实例原样保留；错误进 stderr（miette 三段式）。
  - 安装事务失败 = 事务回滚到旧包（T10.12 的备份/回滚语义），运行中实例不动。
  - 重载 / 新实例 `setup` 失败 = 按 T10.12 选定的事务边界收场：提交前 = 回滚并重启用旧版本；提交后 = 补偿重装备份包 + 重启旧版本。`metis dev` 报告失败并继续 watch。
- **不动的目录**：`config.yml` 与 `data/` 永不被构建/安装路径写入；`config.yml` 变更走 [config-format](config-format.md) D42 通道，不是构建触发器。
- **挂载实例联动**：provider 重载按 [instance-model](instance-model.md) D65 只影响之后的新挂载（已有实例持旧代码快照）；D70 的 epoch 失效会级联到实例内消费者，执行归 T10.12/T10.9。dev 重载不绕过这套语义。
- **并发**：所有安装/激活路径（含 `metis build --install`）共用 T10.12 安装事务锁；部署根级另加宿主/开发锁，覆盖「已有宿主在跑」与「第二个 dev 实例」；锁文件住 OS 临时/缓存目录，用操作系统 advisory lock 或 PID + 启动时间检测 stale lock。
- **不做**：v1 不实现「先准备新实例再停旧实例」的可用性优化——那是热换语义的变化，不是构建管线的权限；登记为 T10.12 的评审输入（D97 面）。last-known-good 由「构建/校验失败不安装 + 安装事务/补偿重装的旧版本恢复」保证，不由新的 swap 机器保证。
- **journal**：构建事件不写 journal（构建是工具面，不是核心操作）；安装 / 卸载 / 重载的条目由 T10.12 的安装记录承载；来源 / artifact hash / 能力清单 / 审批结果若需入 journal，作为 T10.12/T10.5 的 journal kind/schema 扩展输入，按 [ADR-0029](../decisions/0029-journal.md) 演进纪律处理。

## 6. D110 watch 语义（2026-10-18 定案）

**D110 定案：源码工程单 lane 监听 + 200 ms debounce + latest-wins + 源码 hash 门。**

- **监听集合**：源码工程根的**全量**（含 `src/**`、`Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`、`.cargo/config.toml`、`build.rs`、`manifest.yml`、工程内 `wit/**`，以及 build script / proc-macro 可能读取的其他工程内文件）；不采用白名单，避免漏掉未声明输入。
- **忽略**：`target/`、`.git/`、`.metis/`、`dist/`、编辑器临时文件（`*.swp`、`*~`、`.#*` 等）；部署根的 `data/`、`journal/` 永不进入监听面。
- **节奏**：debounce 200 ms（实现期配置）；构建进行中到达的变更标脏，当前构建结束后再跑一轮（latest-wins）；构建串行；被 kill 的构建不产生安装。
- **hash 门**：每次事件后先算构建指纹（D111）；指纹未变 = 跳过构建。这条同时解决「保存风暴」与「构建产物自触发」。v1 用全量指纹；增量指纹（Merkle / 受影响路径失效）= 扩展位（触发器 = 大工程指纹成为主导耗时）。
- **环境**：构建子进程使用管线显式设置的 sanitized 环境（`PATH`、`CARGO_HOME`、`RUSTUP_HOME`、`RUSTFLAGS`、`SOURCE_DATE_EPOCH` 等声明项），不继承作者的任意环境变量；build script / proc-macro 读取工程外输入或未声明环境 = 不在缓存正确性保证内（`dist` / farm 构建要求 hermetic）。
- **lane 分工**：源码 / manifest 变更 = 构建 lane；manifest 变更导致包内容变化 = 整包重装；`config.yml` 变更 = 配置 lane（D42），本管线不监听、不处理。
- watch 后端候选 = `notify` crate；具体选型与依赖引入归 T10.15（走 `adding-dependencies`）。

## 7. D111 缓存与指纹（2026-10-18 定案）

**D111 定案：两层缓存——每工程 Cargo target dir + metis 内容寻址产物缓存；不立全局共享 `CARGO_TARGET_DIR`。**

- **第一层**：每工程独立 Cargo target dir（默认 `<project>/target`），负责增量编译。跨项目共享 target dir 的锁竞争、未建模输入与磁盘增长由 metis 承担，收益不抵风险（调研结论）。
- **第二层**：平台缓存目录下的 `metis/build/<pipeline_version>/<key>/`（`$XDG_CACHE_HOME/metis` / `%LOCALAPPDATA%\metis`），只存可部署包；`build-info` 若存在 = 派生诊断，不作权威。key = SHA-256（规范化编码）over：
  - 源码树 hash：**全量源码工程减排除集**（相对路径排序 + 内容 hash；排除 `target/`、`.git/`、`.metis/`、`dist/`、编辑器临时文件）；符号链接记录 target；这样 build script / proc-macro 读取的工程内文件也会进 key。
  - 解析后依赖图：registry 版本取 lockfile，path 依赖递归 hash（工程外 path 依赖同样纳入，避免 stale 命中）。
  - 工具链指纹：`rustc -vV` 的 commit / host / LLVM、`cargo -V`、`wasm-component-ld`、`wasm-tools`、wit-bindgen / SDK 版本。
  - target / profile / 解析后的 features / `RUSTFLAGS` / 影响构建的声明 env。
  - key 带 schema 版本前缀（`v1`）；`v1` = 缓存 key 编码格式版本，`pipeline_version` = 构建管线版本，两者独立 bump；namespace 的 `pipeline_version` 与产物元数据同源。
- **命中与写入**：命中后校验产物 hash；原子写（temp + rename）；按 key 加锁。mtime 不作 key。
- **工程外输入**：工程外文件、未声明 env、网络/时钟等非 hermetic 输入不在本地缓存保证内；`dist` / 远端构建场要求 `--locked --offline` 与 hermetic 输入。
- **清理**：默认上限 **5 GiB**、最大年龄 **30 天**（实现期配置可改）；每次写入与启动时按 `last_used` 标记（不依赖 atime）做 LRU 淘汰；`metis cache clean --max-size / --older-than` 是手动覆写。T10.15 加一条容量测试。
- **sccache** = 可选扩展位：只能缓存依赖 rlib，不能缓存最终 `cdylib` / component link；启用时关闭 Cargo incremental。v1 不默认启用。
- `.cwasm` 预编译产物 = 运行时缓存（wasmtime 版本/Engine 配置绑定），不作构建缓存。
- **F8 区分**：产物 hash 校验失败 = 缓存损坏（重建并覆盖）；工程外/未声明输入导致的旧产物 = stale hit（不在本地缓存保证内；靠 `dist` / farm 的 hermetic 构建排除）。

## 8. D112 工具链、版本与产物元数据（2026-10-18 定案）

**D112 定案：统一预检 + 脚手架版本单一来源 + dev/dist 同 profile + 确定性 `metis.build` section。**

- **预检**（`metis check` / `build` / `dev` 共同）：rustc / cargo、`wasm-component-ld`、`wasm-tools`、wit-bindgen 版本、`wasm32-wasip3` target 在场（WASI 0.3；2026-10-19 维护者确认）、插件 manifest 声明的 WIT package version vs 宿主 ABI 版本（判定语义 = W10 D187）、发布级构建的 lockfile 在场。SDK SemVer 只作诊断，不作 ABI 兼容判定。失败 = miette 三段式（code + 说明 + help，如 `rustup target add wasm32-wasip3`）；**不自动安装**。
- **脚手架版本来源**：`metis new` 的 `rust-toolchain.toml`、SDK 依赖版本、`.cargo/config.toml` 从 metis 二进制自身的构建常量生成（单一来源），不在模板里手写版本号。
- **profile**：dev 与发布级构建共用优化 profile `metis`（继承 `release`）；不做 dev-only debug/未优化分叉，避免「dev 能跑、发布件行为不同」与缓存分叉。具体 `debug` / `strip` / `codegen-units` / `incremental` 值在 T10.15 以可复现测试钉定并写入脚手架。
- **产物元数据**：构建完成后用 `wasm-encoder` 在 component 层追加 `metis.build` custom section；内容 = canonical JSON（键排序、无空白、无 timestamp、无绝对路径）：`pipeline_version`、`source_sha256`、`sdk_version`、`wit_package_version`、`wit_sha256`、`target`、`profile`、`toolchain_version`（仅版本，不含 host / path）。追加前剥离所有同名 section，校验恰好一条；`metis.build` 是诊断 / 溯源面，不是信任锚，签名 / attestation 单独绑定产物。若后续有 strip/optimize 步骤，重新追加并校验在场。复现比较 = 剥离该 section 后的规范化 digest（D114）。
- **可复现姿态**：本地构建**不承诺**字节级可复现（Rust 对 wasm target 的可复现性仍在演进）；为远端构建场预留三件：路径重映射（`--remap-path-prefix`）、`--locked --offline`、钉版工具链镜像。源代码 hash 与构建指纹现在就稳定输出，供将来对账。
- **UX**：成功 = 一行摘要（插件名 / 耗时 / 警告数）；失败 = 完整 miette 诊断；`--json` 输出 NDJSON 事件（构建开始 / 成功 / 失败 / 安装 / 重载）；`--verbose` 才透出 cargo 原始输出。构建事件不写 journal。

## 9. D113 部署内工具链服务（扩展位，2026-10-18 定案）

**D113 定案：部署内工具链服务不进 v1；只冻结安全边界与触发器。** 它是 ADR-0034 §6 的「可选件，Creator 模式的体面退场形态」，也是 D115 自扩展快车道在「agent 没有本机工具链」时的实现形态。

- **硬约束**（不可协商）：
  - 编译器跑在**独立沙箱进程/容器**（microVM 或容器），不跑在宿主进程内，更不跑在 wasm 沙箱内（WASI 不能 spawn 进程/线程，rustc/cargo 无法在 guest 内运行）；
  - 只读 rootfs；默认无网络；CPU / RAM / 时间 / 磁盘配额；构建环境无长期密钥；
  - 源码提交走 ADR-0008 审批门（v1 接口预留）；高风险能力（global fs / process / network / 核心配置写入）按部署者策略：v1 = 预授权 / 策略允许；人工审批 = ADR-0008 审批门（v1 不实现）；既未预授权又不自动允许的能力 fail-closed；
  - 产物走与本地相同的 D108 校验 + D109 安装事务 + D112 元数据管线；
  - 来源 / artifact hash / 能力清单 / 审批结果由 T10.12 安装记录承载（「安装记录」= T10.12 事务的可查产物，载体由 T10.12 开工定：journal 条目或事务状态文件）；若需进 journal，作为 T10.12/T10.5 的 journal kind / schema 扩展输入，按 [ADR-0029](../decisions/0029-journal.md) 演进纪律处理。
- **接口**：`BuildRequest` / `BuildAttestation` schema 与 D114 共用；传输形态（本地 socket / CLI / WIT import）随 W3 章与实现期定，本文件只冻结安全边界。
- **触发器**：首个「agent 在无工具链部署内自写插件」需求，或 Creator 模式复活。

## 10. D114 远端构建场（扩展位，2026-10-18 定案）

**D114 定案：远端构建场不进 v1；现在只预留接口 schema、信任链与触发器。** 它是 ADR-0034 §6 的 marketplace 配套，与 ADR-0024 §6 防线 3 的「确定性构建 + 结构校验 + 签名」同管线。

- **BuildRequest**：`pipeline_version`、source URI + commit、manifest / package digest、lockfile digest、toolchain image digest、target / features、profile。
- **BuildAttestation**：`pipeline_version`、artifact digest（完整产物）、`reproduction_digest`（剥离 `metis.build` 后的规范化 digest）、`builder.id`、status、`reproduction` result（未复现 / 单方复现 / 双方复现）；`externalParameters` 只放不可变输入，`resolvedDependencies` 放 manifest / lockfile / rustc / 镜像 digest。
- **复现定义**：比较 `reproduction_digest`（规范化 = 剥离 `metis.build` section，按 canonical encoding 重算），不是完整产物字节；比较范围 = 可部署包内的组件产物（manifest / package digest 单独记入 `resolvedDependencies`）；完整 artifact digest 用于安装 / 签名。这样两个 builder 的 host 差异不会伪造「未复现」。
- **发布路径**：产物上传 + 发布者/CI 签名 + 结构校验（D108 的检查面共享）+ 可选重建；未重建或重建不一致标「未复现」，**不阻断安装**（与 ADR-0024 闭源降级一致）。
- **结构校验**（v1 的本地构建也执行，marketplace 侧复用）：组件合法性、固定 feature 集合、import/export allowlist、ABI 指纹、资源上限、禁用 threads / shared memory / cm-threading、拒绝 start 函数与命令类导出（`wasi:cli/run` / `_start`；[wit-abi](wit-abi.md) W1.7 D122）。结构校验不证明无恶意逻辑；运行时能力沙箱仍是最终防线。
- **构建场实现约束**（将来）：microVM / 容器 + 网络拒绝 + 钉版镜像；至少两个独立 builder 才能标「双方复现」。
- **触发器**：marketplace 接受 source-only 提交 / 需要「已复现」标 / 法规要求。

## 11. D115 自扩展快车道（2026-10-18 定案）

**D115 定案：release ≠ build/install；分发 lane 与自扩展 lane 共用同一条构建管线。** 自写自用的插件不经过 git / CI / marketplace / 签名 / semver；但仍必须经过 compile → validate → capability check → install transaction → activate。ADR-0024 §6 防线 4「自写自用零审批」是本 lane 的信任地基；零审批不等于零能力门。

|        | 分发 lane                       | 自扩展 lane                                            |
| ------ | ------------------------------- | ------------------------------------------------------ |
| 触发者 | 插件作者 / 第三方               | agent / Creator / 作者本人                             |
| 源码   | 独立 git 仓（或 monorepo 成员） | 任意可写目录；git 可选，甚至没有                       |
| 版本   | semver / tag / changelog        | 不需要                                                 |
| 构建   | CI / 本地 `metis build`         | 一次 `metis build --install`（同机）或构建服务（D113） |
| 信任   | 签名 + 结构校验 + 可选重建      | 本地信任档（self-authored local）                      |
| 分发   | marketplace / 部署仓            | 当前部署根                                             |
| 审批   | 发布审核 / 安装时能力清单       | 能力门 fail-closed；高风险能力按部署者策略             |

- **源码位置**：agent 可写进自己的 private data（`plugins/<agent>/data/fs/...`），或宿主管理的临时 / 缓存目录（不落在部署根内）；这些不是 `plugins/<new-name>/` 的包内容区。构建服务把源码复制进隔离沙箱编译；只有产物进入新插件包内容。源码可选保留内容哈希副本供审计/回滚；来源 / artifact hash / 能力清单 / 审批结果由 T10.12 安装记录承载（若需入 journal，按 D113 的 schema 扩展输入处理）。
- **构建期信任边界**：`cargo build` 会执行 `build.rs` 与 proc-macro，它们在宿主上拥有调用者的完整权限——这是 wasm 沙箱**之前**的代码执行面。因此：同机快车道只适用于本来就有主机代码执行权限的作者/外部 agent（能调用 `metis build` 的主体本就能运行命令，本 lane 不额外授权）；宿主内的 wasm agent 没有 process 能力，不能直接调用 `metis build`，必须走 D113 的沙箱构建服务。想限制 agent 的构建期权限，唯一形态 = D113 沙箱。
- **能力与审批**：新插件能力由 manifest 声明 + 宿主能力门 fail-closed 执行；agent 不能给自己加未声明的 import。零/低风险能力（纯计算、事件处理、私有数据）按本地信任档自动激活；高风险能力（global fs / process / network / 核心配置写入）的 v1 行为 = 部署者预授权 / 策略允许，人工审批 = ADR-0008 审批门（v1 接口预留，不实现）；既未预授权又不自动允许的能力必须 fail-closed。
- **不可修改面**：宿主核心、审批门、恢复路径不是插件；**运行期** agent 写插件不能改这些（构建期边界见上一条：同机快车道不在 wasm 沙箱内）。自替换失败时，宿主保留 last-known-good + 可禁用/回滚，避免「自我修改把恢复路径也弄坏」（ADR-0008 的安全前提）。
- **替换语义**：自替换复用 D109 的安装事务与 W4.5 卸载时序；agent 需把跨实例状态写入自己的持久化存储，不能依赖旧实例内存态。
- **不做**：运行时 eval / 解释执行 / 热注入代码。wasm-first 不提供这条通道；ADR-0008 的教训是「faulty self-modification can disable the very process needed to recover」。
- **v1 面**：同机快车道（机器上有 Rust 工具链）由 T10.15 覆盖；宿主内构建服务 = D113 扩展位。
- **git 门槛**：`metis new` 默认 git init 只是正常工程习惯；快车道可用 `--vcs none`，或在任意目录写源码后直接 `metis build --install --config <path/to/metis.yml>`。构建管线不依赖 git。

## 12. 脚手架与输出形态（D106 / D112 的落地细节）

`metis new <name>` 生成（v1）：

```text
<name>/
  Cargo.toml              # crate-type = ["cdylib"]；metis-plugin 精确钉版；[profile.metis] inherits = "release"
  Cargo.lock              # 首次构建生成
  src/lib.rs              # struct + impl Plugin 骨架（一个示例钩子）
  manifest.yml            # 最小骨架；字段形状 = wit-abi W2 D125–D134
  rust-toolchain.toml     # channel + targets = ["wasm32-wasip3"]，版本由 metis 构建常量生成
  .cargo/config.toml      # [build] target = "wasm32-wasip3"
  .gitignore              # target/、.metis/、dist/
  README.md               # 最小插件 + 构建/运行说明
```

- `metis dev` 的成功输出 = 一行（插件名 / 构建耗时 / 警告数 / 重载结果）；失败 = 完整诊断；`--json` = NDJSON。构建 / 安装事件不写 journal。
- 默认临时部署根 = `<project>/.metis/dev/`（根有 `metis.yml`；`plugins/<name>/` 内有 `manifest.yml`、空 `config.yml`、`data/`）；`--config <path/to/metis.yml>` 指向真实部署根，只写包内容，不动 `config.yml` / `data/`。
- `metis new` 向上查找 `metis.yml`；目标落在 `<deployment>/plugins/` 下时拒绝创建（不是仅警告）；当前目录是部署根时要求显式 `--in <source-dir>`。
- v1 一个工程一个插件；多插件 workspace、watch globs 配置、构建 profile 自定义 = 扩展位。

## 13. 扩展位登记

| 扩展位                                   | 归属                                          | 触发器                                                    |
| ---------------------------------------- | --------------------------------------------- | --------------------------------------------------------- |
| attach 到已运行宿主                      | D107                                          | 首次「对着共享/远端部署开发」需求                         |
| Cargo workspace / 多插件构建             | D106 / D108                                   | 首个真实多插件工程需求                                    |
| 自定义 `build.command` / 非 Rust 适配器  | D108                                          | 真实自定义工具链或语言梯队进站                            |
| 全局共享 Cargo target dir                | D111                                          | 不立；若未来立，须先证明锁/磁盘/未建模输入可控            |
| sccache 可选加速                         | D111                                          | 依赖编译成为主导耗时，且 wasm target 命中率实测可接受     |
| 缓存默认值调优                           | D111                                          | 实测 5 GiB / 30 天不合适                                  |
| 增量指纹（Merkle / 受影响路径失效）      | D110                                          | 大工程的全量指纹成为主导耗时                              |
| 完整 `build-info` sidecar / 更丰富元数据 | D112                                          | 首个需要离线诊断产物来源的需求                            |
| 源码映射 / 更丰富 debug info             | D112                                          | 首个 wasm trap 调试痛点                                   |
| prepare-new-before-stop 热换             | D109（T10.12 面）                             | 可用性需求进入安装事务站                                  |
| 部署内工具链服务                         | D113                                          | 首个「agent 在无工具链部署内自写插件」需求 / Creator 复活 |
| 远端构建场                               | D114                                          | marketplace source-only 提交 / 「已复现」标 / 法规要求    |
| `config.yml` fs watch                    | D110（[config-format](config-format.md) D37） | 既有触发器不变                                            |

## 14. 联动登记汇总

| 去向                  | 登记内容                                                                                                                                                                                                                                                                                                                                                                         |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T10.15（构建管线站）  | 开工输入 = [build-pipeline](build-pipeline.md) + [build-pipeline-survey](../research/build-pipeline-survey.md)；v1 实现面 = D105–D112 + D115 同机形态；F 类窗口 = §15（收官时迁入 [testing-strategy](testing-strategy.md) §3.F）；D11 / cov-floor 若新增内核 crate 则按 station-run 义务；新增依赖（`notify`、`wasmparser` / `wasm-encoder`、hash 库等）走 `adding-dependencies` |
| T10.14（插件 SDK）    | 脚手架 SDK 依赖来源（v1 path 到 metis 检出 + 版本/WIT hash）；版本常量由 metis 构建提供；D99/S2 的「生成物住 SDK crate」不被构建管线破坏                                                                                                                                                                                                                                         |
| T10.13（metis-codec） | 无直接依赖；若 `metis.build` 将来改用 TLV，形态随 codec                                                                                                                                                                                                                                                                                                                          |
| T10.12（安装事务）    | D109 调用其安装事务；事务边界（激活在事务内 vs 补偿重装）由 T10.12 开工拍板；D65/D70 挂载联动；所有安装/激活路径共用事务锁；`prepare-new-before-stop` 作为评审输入登记（不归本件）                                                                                                                                                                                               |
| T10.9 / T10.10        | 部署索引扫描与组件运行时共享 D108 的纯校验 / import 提取面；该面住共享 crate（`metis-component` 或 `metis-validate`），运行时 crate 不依赖 `metis-build`                                                                                                                                                                                                                         |
| W1 章（T13-h）        | **已兑现（2026-10-18）**：可部署包布局与组件文件名 = D116；start / 命令组件拒绝 = D122；WIT 包与 world = D117                                                                                                                                                                                                                                                                    |
| W2 章（T13-i）        | **已兑现（2026-10-19）= wit-abi W2 D125–D134**：manifest 形状与脚手架骨架的 SSOT；合成最小 manifest = `version` + `abi`（无 `name`）；**不引入任意 `build.command`**（D108 约束）                                                                                                                                                                                                |
| W3 章（T13-j）        | D113 构建服务的传输形态（若走 WIT import）                                                                                                                                                                                                                                                                                                                                       |
| W10 章（T13-p）       | **已兑现（2026-10-19）= wit-abi W10 D186–D193**：ABI 版本接受语义（D187）与工具面判定落点（D193）；D112 预检消费同一比较实现；命令面 SSOT（D193）                                                                                                                                                                                                                                |
| W9 章（T13-o）        | **已兑现（2026-10-19）= wit-abi W9 D180–D185**：`metis check` 单插件 / 部署根模式的五格范围 = D185；预检与 loader 启用期同一实现；WASI 0.3 基线逐条清单 = T10.10 宿主注册集（SSOT），D112 只预检 toolchain / target，`metis check` 消费 T10.10 的记录                                                                                                                            |
| T13-f                 | 清写交接（2026-10-18）：[config-format](config-format.md) D29/D35 的旧入口与布线文件已改为组件布局，[seam-capabilities](seam-capabilities.md) 已按 wasm 能力面重写；命令面 = 本文件 D106 的 `metis new/check/build/dev`。D35 = 源码侧 manifest 可选，包内始终合成最小 manifest                                                                                                   |
| journal               | 构建事件不写 journal；安装/卸载/重载条目由 T10.12 安装记录承载；来源/artifact hash/能力清单/审批结果入 journal = T10.12/T10.5 的 schema 扩展输入（ADR-0029）                                                                                                                                                                                                                     |
| AGENTS.md             | 文档地图登记本文件与调研报告                                                                                                                                                                                                                                                                                                                                                     |

## 15. T10.15 的 F 类窗口预登记

本表 = T10.15 开工输入，不是 F 类 SSOT；T10.15 收官时按 [testing-strategy](testing-strategy.md) §3.F 的登记位落位，迁移后本表压缩为指针。

| #   | 故障窗口                             | 恢复承诺                                                                                                           |
| --- | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| F1  | 编译失败                             | 不安装；旧包与旧实例不动；错误可读                                                                                 |
| F2  | 结构校验 / 能力对账失败              | 不安装；旧包与旧实例不动；错误指明 import / ABI / feature / start 函数                                             |
| F3  | staging 写入失败                     | 不进入安装事务；清理临时文件；旧包不动                                                                             |
| F4  | 原子替换失败（跨平台 rename / 权限） | 事务回滚；旧包可用；错误可读                                                                                       |
| F5  | 安装事务回滚失败                     | 响亮错误 + 保留备份路径；宿主进入可诊断状态                                                                        |
| F6  | reload / 新实例 setup 失败           | 按 T10.12 事务边界：提交前回滚；提交后补偿重装备份包 + 重启旧版本；`metis dev` 继续 watch                          |
| F7  | watcher 丢事件 / 构建循环            | 源码 hash 门兜底；`--verbose` 可诊断；不产生无限构建                                                               |
| F8  | 缓存损坏 / stale hit                 | 产物 hash 失败 = 重建；工程外 / 未声明输入 = stale hit，不在本地缓存保证内，dist / farm 用 hermetic 构建排除       |
| F9  | 工具链缺失 / 版本不匹配              | 预检失败；给出精确安装/升级命令；不自动安装                                                                        |
| F10 | 并发 `metis dev`                     | 部署根级锁拒绝第二个实例；不互相覆盖产物                                                                           |
| F11 | 自扩展绕过能力声明                   | 能力门 fail-closed；安装前对账；安装记录可查                                                                       |
| F12 | 元数据 section 丢失 / 伪造           | 追加前剥离同名 section、校验恰好一条；缺失 = 重建或拒绝安装；不把 `metis.build` 当信任锚                           |
| F13 | 构建期 host 代码执行                 | 同机快车道只对已有 host 代码执行权的主体；in-host agent 必须走 D113 沙箱；`build.rs` / proc-macro 不在 wasm 沙箱内 |
| F14 | `build --install` 与 `dev` 并发      | 共用 T10.12 安装事务锁；第二个操作等待或明确拒绝                                                                   |
| F15 | 已有宿主在跑                         | 部署根级宿主/开发锁；拒绝第二个写入者                                                                              |
| F16 | stale lock                           | OS advisory lock 或 PID + 启动时间检测；过期锁可回收                                                               |
| F17 | provider reload 波及挂载实例         | D65：已有挂载持旧代码快照、之后的新挂载拿新快照；D70：epoch 级联消费者；恢复归 T10.12/T10.9                        |
| F18 | manifest-less / SDK 依赖缺失         | 构建时合成最小 manifest；脚手架 SDK path 缺失 = 预检失败，不生成不可构建工程                                       |

## 16. 决策索引

| 编号 | 定案                                                                                                       |
| ---- | ---------------------------------------------------------------------------------------------------------- |
| D105 | v1 = 本地构建闭环；两条服务化通道为扩展位；F 类登记迁入 testing-strategy §3.F                              |
| D106 | 源码工程与部署根两源分离；顶层命令面；git 拓扑三形态                                                       |
| D107 | `metis dev` 单进程内嵌宿主；无 attach（v1）                                                                |
| D108 | Rust 内置适配器；无任意 `build.command`；可部署包；manifest-less 合成；校验 SSOT；拒绝 start               |
| D109 | staging → 校验 → T10.12 事务（边界由 T10.12 拍板）；失败旧版本恢复；D65/D70；并发锁                        |
| D110 | 全量源码监听 + 200 ms debounce + latest-wins + hash 门；sanitized build env                                |
| D111 | 每工程 target dir + 内容寻址缓存；全量源码 hash；5 GiB / 30 天；pipeline_version 命名空间                  |
| D112 | 全工具链预检；WIT package version 比较；同 profile；`metis.build` 非信任锚；复现比较剥离该 section         |
| D113 | 部署内工具链服务 = 扩展位；沙箱/无网络/配额/审批硬约束；安装记录承载 provenance                            |
| D114 | 远端构建场 = 扩展位；BuildRequest / BuildAttestation；`reproduction_digest`；未复现分级                    |
| D115 | 分发 lane 与自扩展 lane 共用构建管线；构建期信任边界；v1 高风险能力 = 预授权/策略；release ≠ build/install |
