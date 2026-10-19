# Metis — 配置格式设计（living doc）

> 状态：living design doc——任务 4（配置格式设计）工作站。**已冻结 = [ADR-0027](../decisions/0027-config-format.md)**；保持 living：细则澄清与教学条继续沉淀，语义修订走 ADR-0027 演进纪律。
> 来源：任务 4。前提文档：[seam-capabilities](seam-capabilities.md)（能力面）、[instance-model](instance-model.md)（实例模型）、[wit-abi](wit-abi.md)（ABI 章）。
> 相关 ADR：[0027](../decisions/0027-config-format.md)（冻结）/ [0002](../decisions/0002-config-as-pure-data.md) / [0004](../decisions/0004-plugin-forms.md) / [0006](../decisions/0006-config-schema-ssot.md) / [0007](../decisions/0007-yaml-config-subset.md) / [0008](../decisions/0008-creator-mode-self-modification.md) / [0023](../decisions/0023-core-restart-semantics.md) / [0036](../decisions/0036-fs-scope-three-layers.md) / [0039](../decisions/0039-fs-capability-two-tier.md) / [0040](../decisions/0040-config-shared-variables.md)。
> Last reviewed: 2026-10

---

## 0. 术语锚点与既定地基

### 0.1 术语锚点（本文档新增；共享术语见 [wit-abi](wit-abi.md) §0 / [seam-capabilities](seam-capabilities.md) §0）

| 术语     | 含义                                                                                                               |
| -------- | ------------------------------------------------------------------------------------------------------------------ |
| 部署根   | 一个 Metis 部署的全部内容所住的目录 = 配置树文件所在目录（D29）                                                    |
| 部署区   | 插件目录内部署者/agent 手写的文件（`config.yml`）：升级保留、分发排除（D36）                                       |
| 运行期区 | 插件目录内运行期产物（`data/`）：升级保留、分发排除、gitignore（D29/D35）                                          |
| 休眠件   | 树里有 entry 但 `enabled: false` 的插件：部署索引收录、不加载不对账不激活（D32；W9 章语义）                        |
| 未装配件 | `plugins/` 目录存在但树里无 entry 的插件（D34）                                                                    |
| 残迹     | 卸载 keep 档后的插件目录：包内容已移除、`config.yml`/`data/` 保留；索引收录、运行期不吵（D48/D49）                 |
| 生效树   | base（`metis.yml`）∪ overlay 按 key 字段级合并的产物 = loader 实际消费的装配单（D38）                              |
| 墓碑     | overlay 里的 `enabled: false` 行：卸载/禁用的显式留痕，审批 diff 可见、可逆（D38）                                 |
| 重挂载   | 单插件热更新路径 = 卸载（[wit-abi](wit-abi.md) W4.5）+ 重启用（W9 章判定重过）；与核心重启同一清理契约（ADR-0023） |

### 0.2 既定地基（本章只承接，不重审）

- ADR-0007：YAML 1.2 受限子集；来源两层 = 人类手写层 + agent overlay；机器写面 = overlay + `config.yml`（经审批，D36/D40）；`--config` 选人类层文件。
- ADR-0002：配置纯数据（不内嵌可执行表达式）；keyed diff 相等性平凡（配置值 diff / volatile 快路径机器由 D36/D37 退役）。
- ADR-0006：config schema SSOT = manifest `config:` 节；校验先于插件代码运行（值文件住处由 D36 修订）。
- ADR-0008：审批事务管线（审批单位 = 插件目录 diff，D36 后含 `config.yml`）。
- ADR-0004：插件名解析（两形态由 D35 合并为目录单形态）。
- W2 章（T13-i）承接：插件身份 = 路径名；config 撞名按构造免疫（值挂各插件自己子树）；对账时机 = 启用期。
- [instance-model](instance-model.md) §0.2：来源分层 × 作用范围两根轴；机器写入约束。

## C1 部署根布局与配置树物理组织

### C1.0 边界与前提

本题定**一切路径的锚点**：部署根选定方式、目录骨架、配置树文件面。不归本题：entry 树字段集（C2）、补丁分层与 diff 协调（C3/C4）、fs 两档（C5）、secrets store 内部形态（C6）、数据生命周期（C7）。

### C1.1 D29 部署根布局

- **部署根 = 配置树文件所在目录**：`--config path/to/metis.yml` 选定文件即选定根（ADR-0007），`plugins/`、`lib/` 全部相对它解析。淘汰专门 `--deploy-root` 旗标（零新概念）。`--config` = **单租户模式**；多租户托管 = `--fleet-dir` 舰队目录模式，两旗标互斥（[instance-model](instance-model.md) §S2.1 D56）。
- **目录骨架**：

```text
<部署根>/
  metis.yml                 # 配置树正本（人类手写层；惯例名，--config 可指他处）
  overlay.yml               # agent overlay 层（机器唯一可写的配置树文件，C3 详谈）
  secrets.yml               # secrets store（豁免①；形态与纪律 = D46/D47）
  values.yml                # 共享变量（非秘密；C6.4 D135）
  lib/                      # 部署级契约包（contract.yml + 文档；W7 章 = T13-m）
  journal/                  # journal 段文件（豁免②；细节 = journal.md J4 = D90/D91）
  plugins/
    foo/
      manifest.yml          # 可选：无 = 零仪式插件（D35）
      config.yml            # 部署区：配置值稀疏覆盖（D36）
      plugin.wasm           # 组件产物（文件名 = W1 章 SSOT）
      lib/                  # 随包契约（contract.yml + 文档）
      data/                 # 运行期区
        fs/                 # private scope 根（ctx.fs 挂载点；插件自带 DB 也住这里）
```

- **插件私有数据同居插件目录**：fs 私域根 = `data/fs/`；插件自带 DB（SQLite / redb / sled 等）由插件自己在 private fs 内管理，核心不代持 DB 句柄（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）。淘汰集中数据根（`data/plugins/<name>/` 分居）：同居让「一个插件的全部」= 一个目录，卸载处置（C7）与备份/迁移粒度顺手。
- **global scope 豁免清单物理落点** = `secrets.yml` + `journal/` 两处（[seam-capabilities](seam-capabilities.md) §5.2 两条豁免）；其余部署根 global 正常可达（维护/调试正道不变）。
- **版本管理纪律**：部署根可整体入库（部署即代码）；gitignore = `secrets.yml`、`journal/`、`plugins/*/data/`；工具面脚手架生成 `.gitignore`。
- **契约包无运行期布线文件**：`lib/` 与插件随包契约包由 loader 扫描；不存在运行期模块别名文件（跨插件代码共享 = 编译期依赖，[service-registry](service-registry.md) §7）。

### C1.2 D30 配置树单文件制

v1 = 单文件 `metis.yml` + overlay 一层，**不立 include 机制**。Cordis 的 Include（外部文件挂载 + 运行时写回）主要服务 bundle/profile 组合分发——bundle/profile 全套已判「包分发时代 machinery」（ADR-0004 / [instance-model](instance-model.md) §0.2），多来源问题已由 overlay 分层解决。登记扩展位，触发器 = 真实的文件拆分/分发需求（任务 9 时代复审）。

### C1.3 联动登记（C1）

| 去向                                           | 内容                                                                                                                |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| [seam-capabilities](seam-capabilities.md) §5.2 | private 根物理落点 + 豁免清单具体化；fs = private 隐式 / global 声明（ADR-0039；命名根撤回；单数 workspace 仍不立） |
| [seam-capabilities](seam-capabilities.md) §5.2 | 插件自带 DB 住 private fs；卸载保留政策 = §C7.1 D48                                                                 |
| [service-registry](service-registry.md) §7     | 部署根布局草图 SSOT 移交本节                                                                                        |
| C3 / C4                                        | overlay 文件形状与补丁语义；`config.yml` 机器写回的注释保留 / 格式稳定纪律                                          |
| C6 / C6.4 / C7                                 | `secrets.yml` 内部形态与纪律；`values.yml` 共享变量（D135）；卸载处置（同居目录的处置粒度）                         |
| [journal](journal.md) J4                       | `journal/` 段文件布局 / 轮转 / 总量封顶（D90）与生命周期（D91）                                                     |
| 工具面                                         | `.gitignore` 脚手架                                                                                                 |
| 扩展位                                         | include 机制（D30）                                                                                                 |

## C2 entry 树语法

### C2.0 边界与前提

本题定**装配单的形状**：顶层形状、entry 字段集、休眠/未装配语义、插件形态单化、配置通道新形态、reload 原则。不归本题：补丁分层与写回纪律（C3）、diff 协调细则与 D37 机制展开（C4）。

### C2.1 D31 顶层形状 = `plugins` map，key 即身份

```yaml
# metis.yml —— 纯装配单
plugins:
    memory: # null ≡ {} ≡ 全部缺省（启用、无字段）
    scheduler: {}
    llm-openai:
        enabled: false
```

- **map 形**，淘汰 Cordis 的 list + `id` 形（`cordis.yml` = entry 列表，每行带自动生成 id 供 keyed diff 定位）：① 身份 = 路径名且部署内唯一（W2 章）——key 天然可作 diff 键，`id` 机制整个不需要；② key 即身份，「name 显式值必须等于目录名」的报错面按构造消失（无 name 字段可写错）；③ keyed diff 相等性平凡（ADR-0002 预言兑现）；④ 重复声明 = YAML map 重复键 = parser/校验层响亮。
- **归一规则**：entry 值 `null` ≡ `{}`（全部缺省）。校验器归一；机器写 overlay 用 `{}`（diff 可读性）。
- **加载顺序不靠声明序**（inject 拓扑驱动，ADR-0001）；受限 YAML 子集里 map 键序无语义。
- key 必须是合法目录名（禁 `/`、`.`/`..` 等）；校验进 loader/`metis check` 同一实现（校验器单一实现、多面暴露哲学）。
- **已知牺牲**：Cordis `id` 的真实职能 = 同插件多实例（同一份代码、两份配置、各挂一份）；v1 部署级答案 = 复制目录（`llm-openai-a/`/`llm-openai-b/`），代码重复与升级改 N 份的代价明记；实例面答案 = 声明 list 形同 key 多实例各带 overlay（[instance-model](instance-model.md) D61）。部署级复制目录的缓解维持不移除（D69）。

### C2.2 D32 entry 字段集 = `{ enabled }` 封闭集

| 字段      | 缺省   | 一句                                                        |
| --------- | ------ | ----------------------------------------------------------- |
| `enabled` | `true` | `false` = 休眠件：部署索引收录、不加载不对账不激活（W9 章） |

- **正面词 `enabled`（缺省启用）**，淘汰 Cordis 的 `disabled`（其 detached 行是准入失败产物，我们无此概念）；翻转 = 拓扑变更走 C4 diff。
- **未知字段 = 响亮报错**（与 manifest 政策同款 typo 防线，W2 章）。
- **config 不在 entry 树**：值住处 = 插件目录 `config.yml`（D36）。
- **不立项与触发器**：`isolate` / 部署侧重命名 / `intercept`（Cordis：给某服务名下所有后代合并配置）/ `group` / 嵌套子树——四者都依赖**子树**概念，而子树 = 实例模型议题。v1 撞名本地解 = 部署者二选一（`enabled` 足够表达），全程不改源码。四件维持不立：isolate 语义由实例作用域 + parent 链遮蔽天然给出、重命名维持不立（实例作用域已缓解撞名）、intercept 职能由声明 `config:` overlay 覆盖、group 由子树实例天然嵌套覆盖（[instance-model](instance-model.md) §S4.2/§S4.3 D68/D69）。entry 封闭集 `{ enabled }` 不动；仅重命名的「首个真实撞名需求」触发器存活。

### C2.3 D33 顶层节封闭集

顶层节 = 封闭集，**未知节 = 响亮报错**（typo 防线同 manifest）。v1 已知节：`plugins`、`core`（核心节 = §C8.1）。配置树是部署者文件，演进 = 新节 additive + 老核心未知节响亮自然兼容，不立版本号机制。

### C2.4 D34 休眠件与未装配提醒面

- **休眠件**（树里有、`enabled: false`）：显式声明态、审批 diff 可见——**运行期不提醒**（不是 W9 章那种「可能无期限的人为 Pending」）。禁用不丢配置：`config.yml` 住插件目录（D36），重新启用配置还在。
- **未装配件**（目录在、树里没有）：**运行期不吵**（配置树是唯一声明面，目录里多个目录不是核心的管辖事项）；`metis check` 提示级列出未装配目录（写码期教具，不进运行期日志面）。
- **部署索引覆盖 = 目录扫描全量**：启动/安装扫描 `plugins/` 全量目录解析登记（休眠件与未装配件带状态标记，纯数据非判定）；依赖方 inject 未装配件 = 启用期响亮报错，与「查无」区分、指路「目录存在但树无 entry」；未装配件解析失败同休眠件政策（索引标记损坏 + host warn 一条，登记启用时响亮报错）。

### C2.5 D35 插件形态 = 目录单形态

- **淘汰单文件形态**（单文件组件直接放 `plugins/`）：一切插件 = 目录 `plugins/foo/`。ADR-0004 名字解析规则缩为 1 条（目录存在即可）；歧义格（单文件组件与目录共存）按构造消失；成长路径从「搬家」变「加 `manifest.yml`」，组件文件不动（零改写标准原样成立）。
- **manifest 保持可选**：无 manifest = 零仪式插件（无 config schema、无能力门资格、无 provide/inject）。零仪式产出 = mkdir + 放一个组件文件，仪式不增。
- **目录内三类文件规则**：包内容（manifest/组件/`lib/`——升级替换、分发内容）/ 部署区（`config.yml`——升级保留、分发排除）/ 运行期区（`data/`——升级保留、分发排除、gitignore）。Creator 审批事务 diff 视野排除 `data/` 子树（运行期产物不进审批面）。

### C2.6 D36 配置通道 = `config.yml` + 注入签名不变

- **值住处**：`plugins/<name>/config.yml`（部署区，目录顶层）。private scope 根是 `data/fs/`——**private scope 结构性够不到自己的 `config.yml`**；持 `global` 的插件读得到文件，但读到的只是**引用而非真值**（秘密真值住豁免的 secrets store；非秘密共享引用真值住 `values.yml`，不是豁免、global 可达，[seam-capabilities](seam-capabilities.md) §5 fs 通道条）——任何 scope 下旁路都拿不到秘密真值（`secrets.yml` 与 `journal/` 两条豁免对一切 fs scope 生效），校验与秘密解析只能过注入通道。
- **`setup` 钩子语义不动**（作者面形态 = W1 章）：核心启用期读 `config.yml` → 按 manifest schema 识别引用形状与 `secret` 标记（缺 required/类型错/多给键 = 响亮失败，**插件代码一行未跑**——ADR-0006 前置原样保全）→ 共享变量解析（C6.4 D135：`{ $value: "name" }` 从 `values.yml` / `secrets.yml` 对应 store 解析；秘密写明文 = 校验拒绝；真值只进该插件实例）→ 最终类型校验 → 默认值合成（稀疏覆盖叠加 schema default）→ 冻结表注入。
- **变化面**：审批单位 = 插件目录 diff（`config.yml` 与代码同单位，ADR-0008 管线不动）；overlay 收缩为装配补丁层（装/卸/启用/绑定，不再管配置值）；`volatile` 词汇退役（哪些键可热应用 = 作者文档 + handler 自行判断）。
- **两档分工**：声明式配置（`config.yml` + manifest schema：部署者可写、审批可见、工具面可验）vs 完全自管（无 `config:` 节 = 核心零参与；运行期状态/偏好走 `data/fs` 或插件自带 DB——判别两问之②，[instance-model](instance-model.md) §0.2）。私域自管配置不进审批面；文档引导部署决策走声明式通道。
- **休眠件的 `config.yml` 不校验**：不使用的插件不检查；required 缺失 = 启用期响亮失败，报错指路 `config.yml` 路径 + 键名。
- **无 `config:` 节 ≡ 空 schema**：`config.yml` 存在且有键 = 一切键按「多给键」响亮拒绝（ADR-0004 旧格「配值 → 响亮失败」精神平移；零仪式插件同规则）。
- **工具面联动**：从 schema 生成 `config.yml` 模板（脚手架）；多插件配置全景聚合查看命令。

### C2.7 D37 reload 权利归插件（原则；机制归 C4）

- `config.yml` 变更（审批事务提交 / boot 检测 / 显式 reload 命令）→ **默认重挂载**（安全缺省）。
- 插件订阅配置更新通知（带新值推送）→ **live-apply 或否决后重挂载，权利在插件**（Cordis `internal/update` waterfall 否决先例：插件可参与配置更新决策）。
- 人直接编辑文件 = 下次 boot 生效；fs watch 实时感知 = 扩展位，v1 不立。纯轮转 secrets store（引用不变的值轮换）不产生 `config.yml` 变更——感知 = 显式 reload / boot（v1），fs watch 同扩展位。
- 机制细则（通知形状、否决通道、与 C4 diff 协调的衔接）= D42。

### C2.8 概念地基

- **依赖为什么不住配置树**：依赖 = 作者意图，唯一住处 = manifest（`inject` 双列表 + `events` 双名单，W2 D131 / W8 D173–D179）；配置树重复声明 = 重复税 + 漂移源（部署者声明的依赖与作者代码需求必然漂移）。**事件 listen ≠ 依赖**——listen 是弱耦合被动面（没人 publish 不是错误，观测类插件「听一切」是一等用法）；`inject` required 才是硬依赖（缺失 = 启用期 Pending/拒，W9 章）。两条通道互补不替代。部署者在依赖面的正当角色 = 审批可见面（capabilities + inject + events listen 流入栏；events publish 出流另列）+ 撞名时选实现（v1 = 二选一，D32）。
- **config 为什么住插件目录**：审批单位一体（agent 动配置 = 动插件，目录 diff = 代码 + 配置一体审批）；core agent-agnostic（配置值 = 插件的领域事务，核心不当保管人）；部署根自包含（一个目录 = 一个插件的全部）。平移而非退役的机制清单：schema SSOT（manifest）、校验前置（启用期、代码未跑）、秘密引用（写处平移）、默认值合成、类型生成（W7 章）。
- **配置树 ≠ 配置值仓库**：配置树（装配单）只剩「装哪些、开不开」；插件运行期攒的状态/偏好从来就不进任何 YAML（判别两问之②，[instance-model](instance-model.md) §0.2）。「绑什么」随单数绑定机制不立撤除（D44）；命名根亦已撤回（ADR-0039），fs 只余 private 隐式 / global 声明两档（C5）。

### C2.9 联动登记（C2）

| 去向                                | 内容                                                                                                                                                                                                          |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W2 章（T13-i）                      | **已兑现（2026-10-19）** = wit-abi W2 D125–D133：manifest 字段全集 / 身份 = 目录名 / config schema / 共享变量引用（`$value`）/ fs 两档；entry 启用禁用 = D32；配置树边界、秘密解析点、setup config 来源 = D36 |
| W1 章（T13-h）                      | 零仪式插件平移为无 manifest 目录插件（D35）——**已兑现 = wit-abi W1.8 D123**                                                                                                                                   |
| W9 章（T13-o）                      | 部署索引覆盖（目录扫描全量、含未装配件与解析失败政策）与缺失三分行三收窄（= 休眠件；未装配件走同族响亮）= D34                                                                                                 |
| C3/C4                               | overlay 装配补丁语义、keyed diff 装配面协调、D37 机制展开、部署 `lib/` 契约重对账时机 = D38–D43                                                                                                               |
| C5                                  | fs 两档（private 隐式 / global 声明；命名根撤回）；env 不立收紧；cwd 缺省 = 家目录（D44/D45 + ADR-0039）                                                                                                      |
| [instance-model](instance-model.md) | per-实例/preset 实例级配置通道 = D67（base ∪ 声明 overlay 字段级合并 + D36 管线复用）；isolate/重命名同批复审 = D69                                                                                           |
| 任务 9（marketplace）               | 包内容 vs 部署区/运行期区三分随包形态定稿（D35）；升级后配置 vs 新 schema 对账报告                                                                                                                            |
| 工具面                              | `config.yml` 模板脚手架；配置全景聚合；未装配件提示（D34）；key 合法目录名校验（D31）                                                                                                                         |
| 扩展位                              | fs watch 实时感知（D37）；仅重命名的「首个真实撞名需求」触发器存活                                                                                                                                            |
| 实现期清单                          | `config.yml` 读取/校验/共享变量解析/默认值合成管线（D36/D135）；gitignore 脚手架（D29）；部署索引目录扫描（D34）                                                                                              |

## C3 补丁分层

### C3.0 边界与前提

本题定**机器写面的补丁语义与落盘纪律**：overlay 形状与合并、写回时机与原子性、`config.yml` 双作者文件的注释保真。不归本题：diff 协调与 reload 机制（C4）；secrets store 内部形态（C6）。前提：overlay 收缩为装配补丁层（D36）；map 形 + `{ enabled }` 封闭集（D31/D32）；机器唯一可写的配置树文件 = `overlay.yml`（ADR-0007）。

### C3.1 D38 overlay 形状与合并语义

- **overlay.yml 与 metis.yml 同 schema**：顶层 `plugins` map、entry 字段集同一封闭集；同 parser 同校验器（校验器单一实现哲学）。机器写 overlay 用规范形（`{}` 不写 `null`，D31 归一规则既定）。
- **合并 = 按 key 字段级稀疏覆盖**：生效树 = base ∪ overlay；overlay 提到的 key 按字段赢，未提到的零意见。不立 Cordis 补丁词汇（`insert:` 行 / 按 id 定位整体替换）——那是 list 形 entry 树的税，map 形合并语义平凡（ADR-0002「keyed diff 相等性平凡」同根）。
- **卸载语义 = `enabled: false` 墓碑 + 审批事务处置目录**（包内容随事务移除；部署区 `config.yml` 与运行期区 `data/` 的处置粒度 = §C7.1 D48：keep/purge 两档可选、缺省保留），不立删除语法：① `null` 已被 D31 占用（≡ `{}`），兼作墓碑 = 同语法两义；② Cordis 先例正是「删除 = `disabled: true`」；③ 墓碑留痕审批 diff 可见、操作可逆（重新启用 = 机器删掉自己的 overlay 行，base 声明自然复活）；④ 目录处置后 + entry 休眠 = 一致静默（D34 休眠件政策原样覆盖）。残留写实：base 的 `foo:` 行与 overlay 的禁用行可能共存，人整理 metis.yml 时顺手清；指向查无目录的禁用行无害（合并产物 = 休眠件）。
- **机器对 overlay.yml 整文件所有权**（规范形重写自由），永不碰 metis.yml（ADR-0007「人类手写层永不被机器重写」不变）。
- **淘汰**：overlay 专用删除语法（两义冲突）；顶层 `removed:` 节（重开 D33 封闭集零收益）；整树替换形 overlay（机器须全知 base 才能写，破坏稀疏分层本意）。

### C3.2 D39 配置树文件写回纪律

- **机器写 overlay 的唯一时机 = 审批事务提交**（ADR-0008 管线：暂存 → 校验 → 审批门 → 原子提交）。v1 无自由运行期写回——Cordis 的 debounced write 服务运行时高频自写，我们的写源 = 离散事务，**debounce 不立**。
- **落盘 = tmp+rename 原子替换 + 串行写队列**（归实现期清单；Windows 句柄占用退避重试同归实现期）。
- **读-改-写在事务内**：overlay.yml 入库 git（D29）、人可手改——机器提交前重读现文件再施加补丁，不拿内存旧快照直接覆盖。
- **解析失败两分**：boot 时 metis.yml/overlay.yml 解析失败 = **响亮拒启动**、指路文件与行号（配置树 = 装配单，读不出不猜着跑）；运行期 v1 不重读树文件（无 fs watch，D37 扩展位），审批事务内 overlay 内容在暂存阶段已校验，提交后无解析失败窗口。Cordis「解析失败保留运行中树」的平移落点 = `config.yml` reload 路径（D42）。
- **多租户细化**：fleet 模式下单租户配置解析失败 = 该租户 Failed、其余照常 boot，不拒整个进程；单租户模式维持本拒启动语义原样（[instance-model](instance-model.md) §S2.3 D58）。
- **淘汰**：运行期免校验热读树文件（与 D37「人改文件 = 下次 boot 生效」矛盾）；boot 解析失败降级为「忽略 overlay 继续跑」（静默丢 agent 装的件 = 装配被篡改却不响）。

### C3.3 D40 `config.yml` 机器写纪律

- **问题定性**：D36 后 `config.yml` = 双作者文件（人手写带注释 + 机器经审批写），ADR-0007「注释保留问题架构性消解」的解法对它失效，纪律须重立。
- **最小 diff 纪律**：机器写 `config.yml` 时**未触碰区域字节级保留**（注释、格式、键序），只改动事务所涉键块；机器新写/重写的值走规范序列化。
- **可行性**：ADR-0007 受限子集（禁锚点/别名/自定义标签/多文档）使行块级补丁器可实现，不需要全保真 YAML round-trip parser。实现归实现期清单。
- **退档路径**：实现期补丁器质量不达标时退档 = **顶层键块整体替换**（该键块内注释丢失；审批 diff 事先显形损失，人审批时知情）。退档只影响注释保真度，不影响语义。
- 工具面模板脚手架（D36 登记）产物 = 规范形，同纪律。
- **淘汰**：机器整文件规范形重写（人的注释全灭、审批 diff 变噪音——ADR-0007 当年避开的问题原地回归）；`config.yml` 再分双层（D36 刚杀掉的 layering creep）。

### C3.4 联动登记（C3）

| 去向       | 内容                                                                            |
| ---------- | ------------------------------------------------------------------------------- |
| ADR-0007   | 「注释保留/格式稳定纪律归 C3」= D40                                             |
| ADR-0008   | 审批管线落盘纪律具体化（D39 原子写 + 事务内读-改-写）                           |
| C4         | 生效树 diff 的消费侧（D38 合并语义 = D41 diff 的输入）                          |
| C5         | fs 两档按 ADR-0039；单数绑定字段不立（D44）                                     |
| C7         | 卸载处置粒度（D38 不预决）；`data/` 保留形态与部署索引扫描/休眠件政策 = D48/D49 |
| 工具面     | `config.yml` 模板规范形（D40）；overlay 状态可视化（墓碑残留提示）              |
| 实现期清单 | tmp+rename 原子写 + 串行写队列（D39）；行块级 YAML 补丁器（D40）                |

## C4 diff 协调与 reload 机制

### C4.0 边界与前提

本题定**变更的协调语义**：生效树 diff 的动作表、D37 reload 原则的机制展开（通知形状/否决通道）、部署 `lib/` 契约重对账时机。不归本题：fs 两档形状（C5）；插件目录内容变更的安装事务细则（实现期；ADR-0008 管线既定）。前提：ADR-0002 keyed diff 机器随 D36/D37 退役（配置值不再住配置树）；D37 原则（默认重挂载 + 插件否决通道）；契约重对账归 W7 章。

### C4.1 D41 装配 diff 协调语义

- **生效树 diff 的两个时机**：① boot = 全量加载，无 diff 概念；② 审批事务提交 = 新旧生效树 diff（生效树 = base ∪ overlay，D38）。map 形 diff = 按键三分，平凡。
- **逐 key 动作表**：

| 变更                                              | 动作                                       |
| ------------------------------------------------- | ------------------------------------------ |
| key 增                                            | 安装 + 启用流程（W9 章判定细则）           |
| key 减（机器装的件撤 overlay 行且 base 无此 key） | 卸载流程（ADR-0018 S4 四段论，依赖者先卸） |
| `enabled` 翻转                                    | 卸载 / 启用                                |

- **key 减幂等**：墓碑日后被清理（工具面提示引导）时，key 减可作用于目录已不存在的件——卸载流程对此幂等（索引查无 → 仅摘树）。
- **失败隔离 = 逐 entry 非事务**（Cordis 先例平移：单行激活失败不影响兄弟行）：失败者落 Failed 显式状态 + `internal/plugin` 事件；依赖方走 W9 章 Pending + warn 既有机器。
- **同批多变更顺序**：先拆卸半相、后建设半相；建设半相内 inject 拓扑驱动（ADR-0001；声明序无语义，D31 既定）。
- **边界（树 diff 不管什么）**：`config.yml` 变更走 D42 通道；插件目录内容变更（代码/manifest/契约）= 安装事务协调（ADR-0008 管线既定：审批提交 → 重挂载该插件，热更新 = 重新启用 = W7 章契约级联机器原样），本章不为其立新规。
- **淘汰**：装配变更事务化（整批全成或全滚——插件激活涉及真实效应，假回滚比显式 Failed 更撒谎；Cordis 明确选非事务，理由平移成立）。

### C4.2 D42 `on_config` 通知通道（D37 机制展开）

- **触发点三处**（D37 既定）：审批事务提交 / 显式 reload 命令 / boot（boot = 新鲜加载，无 diff）。reload 命令操作面形态（CLI/控制接口）归实现期工具面。
- **管线先行**：新 `config.yml` 先过 D36 全管线（校验 → 秘密解析 → 默认值合成 → 冻结）。**校验失败 = 拒绝变更**：运行中实例持旧配置不动，响亮 surface（host 报错 + journal `config_rejected`）——「解析失败保留运行中树」在 config 面的平移（D39）。文件保持坏状态等人修；下次 boot 该件启用响亮失败（D36 既定路径）。
- **结构相等 → no-op**（规范化比较器归 W7 章）。
- **通道 = 固定 world 的 `on_config` 钩子**（W1 章；作者面 = SDK trait 钩子，默认实现 = 不吸收 → 重挂载，作者覆写即可参与 live-apply；ABI 演进纪律 = WIT package version，归 W10 章）：

```text
# 语义示意（ABI 签名 = wit-abi W1.3 D118）
on_config(new_config):
    if new_config.refresh_interval != current.refresh_interval:
        return false        # 吸收不了 → 核心重挂载
    current = new_config
    return true             # live-apply 成交
```

- **返回协议二元**：`true` = live-apply 成交（插件自负其责把新值用起来）；**其余一切 = 重挂载**——缺 handler / 返回非 `true` / 抛错（抛错附带日志，ADR-0014 fail-open 同族精神：handler 崩了不挡路，退安全缺省）。**不立 transform 返回**（config 以文件为真相，handler 改值 = 第三种 config 风味、文件与运行态漂移）；**不立永久否决**（文件已是真相，插件选择怎么吸收，不能拒绝现实）。
- **handler 可挂起**（fiber 邮箱回调同族，fuel/epoch 时间盒原样）；全量推送 + 插件自持旧快照对比（淘汰 Cordis volatile 式按路径通知——与 setup 注入同形，零新概念）。
- **重挂载** = 卸载（[wit-abi](wit-abi.md) W4.5 五段：关闸 → 优雅窗口 → `dispose` → 实例即弃 → drain；dispose 细节 = W1.6 D121）+ 重启用（新组件实例、新 setup 拿新 config）；提供方重挂载 → 消费者 epoch 重启（ADR-0001/0018 既有机器）。live-apply 的价值正在此：自己吸收 = 消费者零感知。
- **秘密纪律按构造成立**：handler 住属主实例，解析后真值定向投递属主——不存在广播事件携带真值的泄露面（W2 章「真值只进该插件实例」不破）。
- **竞态与排队**：handler 在飞或插件 Loading 中又来变更 → **合并取最新**（config 是状态不是事件流，跳过中间态合法）；在飞 handler 返回 `true` 只成交它所见的那一版，不抑制合并后最新版的再次投递（stale 成交不结案）。热更撞 Loading 中 setup = 惯性原则兑现（W1 章：进行中的加载跑完落定再响应）。重挂载后 setup 失败 → Failed 既有路径（W9 章 warn 机器接管可见性）。
- **观测面不立新事件**：`internal/plugin` 族 + journal（`config_committed`/`config_rejected`）够用；审批 diff 本身 = 变更的第一观测面。
- **零仪式插件**（无 schema）按构造永不收到通知（config 恒空表，无变更源）。
- **淘汰**：① 开 `internal/update` 为 waterfall 广播事件（ADR-0016 💤 转正）——广播携带解析后真值 = 秘密泄露面；携带未解析引用 = 属主拿不到可用值、还须新立解析 API；否决权散给所有监听者，而「我的配置我做主」本质是属主专属权利。② ctx 新方法 `ctx.config.on_update(fn)`——生命周期钩子住处历来在入口契约（setup/dispose），不在 ctx 注册轴；固定 world 的 `on_config` 钩子让「此插件会 live-apply」跑代码前静态可见（默认实现 = 重挂载）。

### C4.3 D43 部署 `lib/` 契约增删改的重对账时机

- **触发点**：boot / 显式 reload（`lib/` 增删改 = 部署者动作，W7 章既定框架）；fs watch 同 D37 扩展位。**agent 经审批提议部署根 `lib/` 变更 = 登记扩展位**——既定审批单位 = 插件目录 diff（ADR-0008），部署根契约提案的管道随契约分发时代定（任务 9 登记）。
- **语义 = W7 章契约级联机器原样复用**：重扫 `lib/` → diff 契约命名空间 → 新增条目 additive 吸收；已有名字形状变更（规范化后不等）= 破坏 → 反向索引级联重启用全部引用方。与「插件随包契约热更新」同一台级联机器，不新造。
- **workhorse 写实 = boot 路径**：`lib/` 是部署级维护动作，通常本就伴随核心重启（ADR-0023 干净重启便宜）；热对账是同一机器的 bonus 用法。
- **失败隔离同 D41**：级联中某引用方重启用失败 = 该件 Failed，不拖兄弟。
- **淘汰**：仅 boot 生效、不立热对账通道（级联机器反正要为随包契约造，触发点加一个是零头；「契约变了要重启核心才知道」对部署者是不必要的钝）。

### C4.4 联动登记（C4）

| 去向                  | 内容                                                                                                                                                               |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| W1 章（T13-h）        | 入口契约 additive 扩展 = `on_config`（D42）——**已兑现 = wit-abi W1.3 D118**（`on-config: async func(config: list<u8>) -> result<bool, error>`）；ABI 版本归 W10 章 |
| W7 章（T13-m）        | 部署 `lib/` 重对账时机 = D43；跨部署分发形态仍归任务 9                                                                                                             |
| ADR-0016              | `internal/update` 💤 钩子的否决功能以定向 hook 实现（D42）                                                                                                         |
| ADR-0002              | keyed diff 退役后的协调语义 = D41；volatile 快路径替代 = D42                                                                                                       |
| ADR-0024              | §5「在途调用排空归插件热更新设计」= 重挂载复用 W4.5 排空路径（D42）                                                                                                |
| 任务 9（marketplace） | agent 提议部署根 `lib/` 契约的审批通道（审批单位 = 插件目录 diff 的扩展，D43 登记）                                                                                |
| [journal](journal.md) | 配置变更拒绝/应用/重挂载的条目政策 = D78（`config_committed`/`config_rejected`；重挂载由 `fiber_state_changed` 族覆盖）                                            |
| 实现期工具面          | 显式 reload 命令形态（CLI/控制接口）                                                                                                                               |
| 实现期清单            | 生效树 diff 器（D41）；config 管线重入与结构相等比较（D42）；`lib/` 重扫 + 级联（D43，复用 W7 章反向索引）                                                         |

## C5 fs scope 与 process 环境政策（fs 两档 = E4 修订）

### C5.0 边界与前提

本题原题面 = workspace/命名根绑定机制 + env 收紧 + cwd 政策。收官形态：**单数 workspace 与单数绑定机制不立**；fs 后续收窄为**两档** = private 隐式 / global 声明（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)；命名根撤回）；process 环境政策落 defaults。不归本题：secrets store（C6）与能力面 fs 两档（[seam-capabilities](seam-capabilities.md) §5.2）。前提：能力面支配关系——process 继承核心进程 OS 全权，fs scope 分段/两条豁免/http 白名单均不经由它强制；secrets 走引用式不住 env。

### C5.1 fs scope = 两档（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）

- **private**：`plugins/<name>/data/fs/` 以 WASI preopen 授予；**隐式读写**，不声明、不可收窄；逃逸防护 = 运行时结构保证，自研两段式解析对 private 退役。
- **global**：减法语义（用户世界 − `secrets.yml` / `journal/` 两条豁免），preopen 表达不了 → host-mediated 自研逃逸舱；manifest 声明 `fs: read|write`（语义 = global；[wit-abi](wit-abi.md) W2.5 D129），缺省 = 无 global；TOCTOU 硬化与路径解析留在宿主实现。
- **命名根不立**：原 ADR-0036 的「manifest 声明 + 部署者绑定」中间档撤回（ADR-0039）——插件可能服务多个 project，静态绑定一次配不齐；需要用户世界访问走 global。
- **单数 workspace 仍不立**：理由 = 「这个部署的工作区是哪个目录」对通用部署无答案（core agent-agnostic 同族）；真要多目录，单根版本来就半残。
- **审批面**：private 不进门；global 最高档（碰用户文件 = 要么信要么不信）；无中间档；偏执部署走 OS 级沙箱——与 process 外墙同一答案。
- **插件无需知道配置目录在哪**：维护/调试走 global 可达部署根不变；动态 project 目录 = 插件自己的 config 值（D36/D45 同族）。
- D32 additive 位备注、D33 绑定节登记位、D41「绑定字段变更」动作行与各联动登记中的单数 workspace 项——结案不立；命名根相关登记随 ADR-0039 结案。

### C5.2 D45 process 环境政策（env 收紧结案 + cwd 政策）

- **env = 继承核心 env + 覆盖，不立收紧层**（W3 章「env 收紧政策归任务 4」结案 = 不立）：
  - 收紧挡不住同 OS 用户 `/proc/<pid>/environ` 读取——立了是假边界（严格制厌恶的模糊处理）；
  - secrets 不住 env（泄露面结构上小，既定）；
  - process 审批权重已按 ⊇ `fs: write`（global 写档）+ 任意域 http 展示（[seam-capabilities](seam-capabilities.md) §5 fs 通道条既定）。
  - 配套：审批/Creator 文档写明「子进程继承核心环境变量、secrets 不住 env」纪律。触发器 = 多租户/托管形态（core 高权、插件降权）到来时重议，与 OS 级沙箱外墙同批。
  - 淘汰：部署级 deny 清单（blocklist 永恒不完整 + 可绕过 = 假安全感）；默认清空 + 允许清单（无 PATH/HOME，ergonomics 死亡）。
- **cwd 政策**：缺省 = **用户家目录**（落选缺省 = 私域：子进程是任意代码、不受私域边界约束，拿私域当 cwd 只给出假 confinement 暗示；家目录 = 用户世界直觉缺省）；家目录不可解析（HOME/USERPROFILE 未设、服务账户）= **响亮报错**指路显式传 `cwd`（严格制：不猜、不静默退档）；显式 cwd = 任意绝对路径（非边界写实：cwd 不构成 confinement，与支配关系同源）；相对 cwd → 相对家目录解析（与缺省同一解析基，零特例）；cwd 不存在 = spawn 自然失败（`process.spawn_failed`），不预检。
- **插件域默认工作目录**（如 shell 插件的项目目录）= 插件自己 `config.yml` 的事（D36 通道），核心零参与；登记 Creator/插件作者文档引导。

### C5.3 联动登记（C5）

| 去向                                                | 内容                                                                                                                                          |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| W3/W9 章（T13-j/o）                                 | fs 两档（private 隐式 preopen / global 声明自研）——[ADR-0039](../decisions/0039-fs-capability-two-tier.md)；命名根撤回；单数 workspace 仍不立 |
| W3 章（T13-j）                                      | env 收紧结案 = 不立；cwd 缺省 = 家目录（D45）                                                                                                 |
| W10 章（T13-p）/ 写码期检查                         | scope 键↔命名空间规则 = private / global 两档（命名根撤回）                                                                                   |
| [seam-capabilities](seam-capabilities.md) §5.2/§5.3 | fs 两档与 process 支配关系已同步                                                                                                              |
| Creator/审批文档                                    | process 环境纪律（子进程继承核心 env、secrets 不住 env）；插件域默认工作目录走 `config.yml` 引导                                              |
| 扩展位                                              | env 收紧触发器（多租户/托管形态）                                                                                                             |

## C6 secrets store 与共享变量

### C6.0 边界与前提

本题定 secrets store 与共享变量的内部形态与运行纪律（W2 章「store 细节归任务 4」兑现；共享变量 = C6.4 D135）。既定地基（只承接不重审）：引用式秘密机制（W2 章——schema 标 `secret: true` → 只许写 `{ $value: "name" }` 引用、写明文 = 校验响亮拒绝、真值只进属主实例、人类手写层机器永不写）；物理落点 = 部署根 `secrets.yml`（D29）；共享变量（含非秘密值）= C6.4 D135；两条豁免 carve-out（[seam-capabilities](seam-capabilities.md) §5.2：一切 fs scope 读写同禁）；轮转感知 = 显式 reload / boot（D37）。

### C6.1 D46 `secrets.yml` 内部形态

```yaml
# secrets.yml —— 人类手写层；机器永不写；gitignore（D29）
openai-prod: sk-...
backup-ssh-key: |
    -----BEGIN OPENSSH PRIVATE KEY-----   # ← pr-guard:allow（文档示例占位，非真实秘密；marker = 仓库 secret-scan 的示例豁免约定）
    ...
```

- **形状 = 扁平 map `name: value`，值仅限字符串**；多行字符串 = PEM 类钥匙正例。结构化秘密（keypair 等）= 拆多个名字；嵌套 map/数组值 = 响亮拒绝（受限子集校验兜底，ADR-0007）。
- **名字 = 部署级共享命名空间**：`{ $value: "openai-prod" }` 引用全局解析；无 per-plugin 隔离（多插件引用同名 = 同值，无害；同一名字同时出现在 `values.yml` 与 `secrets.yml` = 响亮，C6.4）。
- **文件缺席 ≡ 空 store**；引用查无 = 启用期响亮失败（W2 章既定路径原样）。
- **淘汰静态加密 at rest**：主密钥住处无解（住 env → `/proc` 同源泄露；住文件 → 鸡生蛋）；v1 = 明文 + 权限锁 + gitignore（W2 章承诺原样兑现）；触发器 = 托管/合规需求，届时外挂 KMS/age 类形态再审。

### C6.2 D47 secrets 纪律（权限档 + 加载/轮转感知）

- **权限锁落档**：boot 检查 `secrets.yml` 权限；POSIX 下 group/other 可读 → **host warn 一条 + 继续，不拒启动**（容器/orchestrator 挂载的 secret 常是 0644 root-owned，拒启动太脆；warn = W9 章人读面精神）；工具面脚手架生成时 chmod 0600；Windows ACL best-effort 归实现期。
- **加载 = boot 一次性读入内存**（core 自持 map）；解析点 = 启用期 config 管线（D36 既定）。
- **轮转感知 = 显式 reload / boot**（D37 既定，本条落位）：reload 重读 `secrets.yml`（共享非秘密值见 C6.4）→ 重跑受影响插件 config 管线 → 结构不等 → D42 通道（`on_config` / 重挂载）。引用不变值轮换对运行中插件零感知直到 reload——写实。
- **机器永不写**（ADR-0007 人类手写层延伸，W2 章既定）；agent 无提议秘密变更的通道（审批单位 = 插件目录 diff），不立。
- **解析失败两分**：boot 读到坏文件（YAML 解析失败 / 受限子集校验拒绝）= **响亮拒启动**、指路文件与行号（D39 同族：读不出不猜着跑——静默置空会让一切引用炸成「查无」，误导排查）；显式 reload 读到坏文件 = **拒绝本次 reload、运行中实例持旧 map 不动** + 响亮 surface（D42「校验失败 = 拒绝变更」在 secrets 面的平移）。fleet 多租户模式下 boot 读到坏文件 = 该租户 Failed、其余照常（D39 细化，[instance-model](instance-model.md) §S2.3 D58）；单租户模式维持拒启动原样。
- **联动**：秘密值按身份注册脱敏管线 = [journal](journal.md) D89（append 前统一精确匹配扫 + `redacted` 字段 + `redaction_applied` 紧随条目 + 短秘密护栏）；journal 加密密钥形态/住处——**消解**（生成型秘密加密随 replay 不立消解 = journal.md D75——无生成型秘密落盘则加密议题消失）。

### C6.4 D135 共享变量（`values.yml` + `secrets.yml` 统一引用）

**D135 定案：共享变量覆盖 secret 与非 secret；一条 `{ $value: "name" }` 引用，schema 的 `secret` 标记选 store。**

- 两个 store = `values.yml`（非秘密，git 追踪，人类手写，扁平 map `name → Value`）与 `secrets.yml`（C6.1，秘密，值仅字符串）。
- 统一命名空间：同名同时出现在两个 store = 响亮；文件缺席 ≡ 空 store；引用查无 = 响亮。
- `{ $value: "name" }` = 整值引用；`$` 前缀键保留给引用标签，字面量位置使用 = 响亮；嵌套字段引用 = 扩展位。
- `secret: true` 键：键存在时必须写引用且必须解析自 `secrets.yml`；明文 / 解析自 `values.yml` = 响亮。`required: false` 时键可缺席（缺席 = 无值，不做引用检查）；`required: true` 时缺席 = 响亮。
- 非 secret 键：字面量合法；引用必须解析自 `values.yml`；引用一个只存在于 `secrets.yml` 的名字 = 响亮（防秘密流入未声明 secret 的键）。
- `default` 是字面量；`{ $value }` 写在 `default` = 响亮。
- 解析后的值再按该插件 schema 做最终类型校验；秘密值仅字符串，`type` 限 `string` / `any`（[wit-abi](wit-abi.md) W2.4 D128）。
- 管线：识别引用形状与 `secret` 标记 → 从对应 store 解析 → 最终类型校验 → 默认值合成 → 冻结；插件代码一行未跑。
- `values.yml` 加载 = boot 一次性读入内存 map；reload 原子替换；变更感知 = boot / 显式 reload（无 fs watch，D37）。
- `values.yml` 解析失败：单租户 boot = 响亮拒启动；fleet 模式 = 该租户 Failed、其余照常；显式 reload = 拒绝本次 reload、运行中实例持旧 map 不动。单插件名字查无 / 类型不符 = 该插件启用期响亮，兄弟不受影响。
- `values.yml` 人类手写、机器永不写（v1）；agent 提议共享值变更 = 扩展位。`secrets.yml` 纪律 = C6.2 原样。
- 审批面：`config.yml` 的引用名进插件目录 diff；`values.yml` 进部署根 git diff；`secrets.yml` 不进审批（人类手写层）。

### C6.3 联动登记（C6）

| 去向                  | 内容                                                                                                                                             |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| W2 章（T13-i）        | **已兑现（2026-10-19）** = wit-abi W2 D128：秘密引用 = `{ $value: "name" }`（D135）、写明文拒绝、引用查无响亮；store 细节 = D46/D47              |
| 共享变量（C6.4 D135） | `values.yml` + `secrets.yml` 统一命名空间；`{ $value }` 引用；`secret` 标记选 store；[ADR-0040](../decisions/0040-config-shared-variables.md)    |
| [journal](journal.md) | 秘密值身份注册脱敏管线 = D89；journal 加密密钥形态/住处——消解（replay 不立 = D75）                                                               |
| 工具面                | 脚手架生成 `secrets.yml` 时 chmod 0600；权限提示；`values.yml` 模板（可选）                                                                      |
| 实现期清单            | boot 权限检查 warn；secrets 内存 map 与 reload 重读管线；解析失败两分（boot 拒启动 / reload 持旧，D47）；`values.yml` 读取与共享引用解析（D135） |
| 扩展位                | 静态加密 at rest（触发器 = 托管/合规需求）                                                                                                       |

## C7 数据生命周期：卸载处置与残迹政策

### C7.0 边界与前提

本题定**卸载后插件目录剩什么**：`config.yml` 与 `data/` 的处置粒度（D38 不预决项兑现；W3 章「db 卸载保留归任务 4」兑现）与保留形态的索引交互（C3.4 登记）。不归本题：升级处置（D35 已定保留部署区/运行期区）；数据备份/迁移/配额（无登记需求，不立）；journal 数据生命周期 = [journal](journal.md) D91。前提：目录单形态（D35）；私有数据同居插件目录（D29）；卸载语义 = 墓碑 + 审批事务处置目录（D38）；部署索引 = 目录扫描全量（D34）。

### C7.1 D48 卸载处置粒度 = 两档可选、缺省保留

- **卸载两档**：
  - **keep（缺省）**：包内容移除；`config.yml` + `data/` 保留 = 残迹目录（D49）。
  - **purge**：目录整体移除（含 `config.yml` + `data/`）。
- **档位选择归部署者**（卸载动作发起时选定；工具面旗标 / 审批面展示位）。**agent 提议卸载固定 keep 档**——删数据不进 agent 提议面（与 D47「agent 无提议秘密变更通道」同族：破坏性动作的提议权不交机器）；要全删 = 部署者自己动手。
- **残迹的事后 purge** = 工具面 purge（命令名归 T10.12）/ 手工 `rm -rf`（部署者专属；残迹经 D49 索引可见，不会遗忘）。
- **缺省保留的理由**：插件自带 DB（在 private fs 内）装着持久意图（schedule 插件的提醒记录、memory 的记忆库——W2 章原则 1「意图数据持真相」），`config.yml` 是部署者手写/审批产物；「卸载」的语义 = 不再参与运行，不是历史蒸发——**删数据是显式动作，不是移除代码的副作用**。与 D35 升级保留同向：禁用（休眠）→ 卸载（移除代码）→ purge（删数据）逐级加强。
- **淘汰**：全删唯一档（误卸 = 持久意图永久丢失）；「留 config 删 data」分界（插件自带 DB 里的持久意图恰恰不可再生，分界不一致反难记）。

### C7.2 D49 残迹目录的索引与扫描交互

- **残迹定义**：目录内无 manifest 且无入口文件（无包内容），但有 `config.yml` 或 `data/` 残留。部署索引标记「残迹」状态——不立此状态则按 D34 政策误归「解析失败」（索引标损坏 + host warn 一条，每 boot 白吵）。
- **运行期不吵**（同 D34 未装配件政策）；`metis check` 提示级列出残迹 + purge 指路。
- **inject 面无特殊 casing**：残迹无 manifest 不提供任何 provides；依赖方走「查无」同族响亮（W9 章）；实现期可在报错措辞层加残迹指路（「该 key 曾由残迹目录提供」），不立机制。
- **同名重装 = 复活**：包内容落回残迹目录，`config.yml` + `data/` 自然接上（name = 身份，D31）；同名新插件继承旧数据 = 工具面/审批面提示「目录含前任残迹」（展示位，不立核心机制）。新插件 schema 与残留 `config.yml` 不符 = 启用期校验响亮（D36 管线原样）；db schema 演化 = 插件自家事（库住 private fs）。
- **空目录**（无包内容、无 config、无 data）：索引不收录（纯噪音源）。
- **残迹的 `config.yml` 不校验**（无 schema 可校，同 D36 休眠件政策）。

### C7.3 联动登记（C7）

| 去向                | 内容                                                                                           |
| ------------------- | ---------------------------------------------------------------------------------------------- |
| W3/W9 章（T13-j/o） | db 卸载保留政策 = D48                                                                          |
| C1.3/C3.4 登记行    | 卸载处置粒度 + 索引交互 = D48/D49                                                              |
| 工具面              | 卸载档位旗标与审批展示；purge；`metis check` 残迹列出与 purge 指路；同名新插件「前任残迹」提示 |
| 实现期清单          | 卸载两档处置（包内容移除 / 目录整体移除）；部署索引残迹状态识别                                |

## C8 核心节

### C8.0 边界与前提

本题定**核心节容器形状**与**一等 cron 复审结案**。不归本题：journal 记录格式与降档语义本体（正式设计 = [journal](journal.md)，已冻结 = [ADR-0029](../decisions/0029-journal.md)；v1 = 写死族骨架 D84，可配审计键面不立）；schedule 插件立项（插件层议题，默认插件集批次）。前提：顶层节封闭集（D33）；overlay 与 metis.yml 同 schema + 按 key 稀疏合并（D38）。

### C8.1 D50 核心节 = `core:`（容器保留，键面不立）

- **节名 `core:`**；顶层节封闭集 = `{plugins, core}`（未知节响亮报错不变，D33 机制原位）。
- **v1 无已定义键**：可配审计键面与 `off` / `summary` / `full` 三档随 journal v1 写死族骨架整体不立（[journal](journal.md) D84）。`core:` 节保留为未来 additive 键的容器；未知键 = 响亮报错（typo 防线同 manifest）。
- **overlay 可补丁 `core:` 节**（D38「同 schema + 按 key 字段级稀疏合并」不豁免）；未来键的审批可见性由同一机制承接。
- **生效路径**：无键面则无换档动作；未来键若落，按 D41 的 host 侧换档语义重议。

### C8.2 D51 一等 cron 复审：结案 = 维持不立

- **复审结论 = 维持不立**（v1 = config 键约定，不立 manifest 一等 `schedule` 字段）：原始淘汰理由不随配置格式变化动摇——一等字段 = 第三台调度机器（与梯度一易失 timer、梯度二持久调度插件重叠），且「多久跑一次」是部署政策不该由作者写死进包。config 键约定在 `config.yml` 时代完整保留（schema 校验/默认值/审批 diff 全在原位，D36）；写码期教具 = 硬编码间隔告警（journal D84 v1 骨架化后 `timer_scheduled` 条目不立）。
- **梯度二持久调度 = 插件层议题**：schedule 插件立项归默认插件集批次（四件套原则已备）；持久意图载体 = 插件自带 DB（private fs 内），与 manifest 字段无关。
- **触发器**：schedule 插件立项时出现 config 键表达不了的声明式调度需求 → 走 WIT package version additive 通道（W10 章）重议。

### C8.3 联动登记（C8）

| 去向                                           | 内容                                                                                         |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------- |
| [journal](journal.md) J3                       | v1 可配审计面不立（D84）；`core:` 容器与封闭集保留                                           |
| W2/W8 章（T13-i/n）                            | **W2/W8 已兑现（2026-10-19）**：一等 cron 维持不立（D51）；事件声明面 = wit-abi W8 D173–D179 |
| [seam-capabilities](seam-capabilities.md) §5.4 | 一等声明式调度复审——维持不立（D51）                                                          |
| W10 章（T13-p）                                | 未来声明式调度需求 → WIT package version additive 通道重议                                   |
| 实现期清单                                     | `core` 节解析与校验（键面不立——剩节容器面 = 未知节报错原位）                                 |

## C9 联动收口

### C9.0 边界与前提

本题兑现**判环预检后补项**并收口全站登记。前提：依赖唯一住处 = manifest（C2.8）；判环运行期语义 = 启用期依赖图（节点 = 已启用 ∪ Pending ∪ 本次启用者，required 边；W9 章）；部署索引 = 目录扫描全量（D34）。

### C9.1 D52 判环预检（后补落位）

- **数据源精确化**：「读部署配置树」精确化为——读部署根树文件（`metis.yml` ∪ `overlay.yml` = 生效树，D38 合并语义）拿 **enabled 集**（overlay 独揽的 agent 装机与墓碑都在生效树里，只读 base 会漏）；**边 = manifest `inject` required 列表**（唯一住处）经 provides key 解析到提供方插件。
- **图构造**：节点 = 树内 enabled 插件（**休眠件/未装配件不入图**——不跑的插件造不出激活死锁，「休眠件撞名不判定」同逻辑）；provides key 查无/提供方休眠 = 边悬空不参与判环（悬空依赖 = 启用期 Pending/响亮既定面，W9 章，非本预检职责）。
- **命令形态**：`metis check` 在部署根运行自动含本项（读得到 `metis.yml` 即跑部署级检查；命令面 = [build-pipeline](build-pipeline.md) D106）；单插件目录模式跳过部署级项。级别维持**炸**（环 = 结构性错误）。

### C9.2 联动登记（C9）

| 去向                 | 内容                                                                            |
| -------------------- | ------------------------------------------------------------------------------- |
| W9/W10 章（T13-o/p） | 判环预检后补 = D52                                                              |
| 任务 4 冻结 ADR      | 九章全收官；supersede 批次吸收 = [ADR-0027](../decisions/0027-config-format.md) |
| 实现期清单           | `metis check` 部署级模式（读树拿 enabled 集 + manifest inject 图判环）          |
