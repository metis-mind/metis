# Metis — 配置格式设计（living doc）

> 状态：living design doc——任务 4（配置格式设计）工作站。**未冻结**：各章随讨论逐站收官；冻结时走新 ADR（候选 0027），各章登记的 ADR 局部 supersede 批次届时一并吸收（登记行 = SSOT）。
> 来源：任务 4。前提文档：[luau-abi](luau-abi.md)（已冻结 = ADR-0026）、[seam-capabilities](seam-capabilities.md)（已冻结 = ADR-0025）、[session-model](session-model.md)（种子稿，任务 5 输入）。
> 相关 ADR：[0002](../decisions/0002-config-as-pure-data.md) / [0004](../decisions/0004-plugin-forms.md) / [0006](../decisions/0006-config-schema-ssot.md) / [0007](../decisions/0007-yaml-config-subset.md) / [0008](../decisions/0008-creator-mode-self-modification.md) / [0023](../decisions/0023-core-restart-semantics.md)。

---

## 0. 术语锚点与既定地基

### 0.1 术语锚点（本文档新增；共享术语见 [luau-abi](luau-abi.md) §0 / [seam-capabilities](seam-capabilities.md) §0）

| 术语     | 含义                                                                                             |
| -------- | ------------------------------------------------------------------------------------------------ |
| 部署根   | 一个 Metis 部署的全部内容所住的目录 = 配置树文件所在目录（D29）                                  |
| 部署区   | 插件目录内部署者/agent 手写的文件（`config.yml`）：升级保留、分发排除（D36）                     |
| 运行期区 | 插件目录内运行期产物（`data/`）：升级保留、分发排除、gitignore（D29/D35）                        |
| 休眠件   | 树里有 entry 但 `enabled: false` 的插件：部署索引收录、不加载不对账不激活（D32；§A9.1 D24 原位） |
| 未装配件 | `plugins/` 目录存在但树里无 entry 的插件（D34）                                                  |

### 0.2 既定地基（本章只承接，不重审）

- ADR-0007：YAML 1.2 受限子集；来源两层 = 人类手写层 + agent overlay；机器只写 overlay；`--config` 选人类层文件
- ADR-0002：配置纯数据（无 `!!luau`）；keyed diff 相等性平凡（配置值 diff / volatile 快路径机器由 D36/D37 退役——Status 行 interim 注已同步）
- ADR-0006：config schema SSOT = manifest `config:` 节；校验先于插件代码运行（值文件住处由 D36 修订）
- ADR-0008：审批事务管线（审批单位 = 插件目录 diff，D36 后含 `config.yml`）
- ADR-0004：插件名解析（两形态由 D35 合并为目录单形态——Status 行 interim 注已同步）
- luau-abi §A2.1：插件身份 = 路径名；§A2.3：config 撞名按构造免疫（值挂各插件自己子树）；§A9.1 D24：对账时机 = 启用期
- session-model §3：来源分层 × 作用范围两根轴；机器写入约束（随 D36 平移，修订注已同步）

## C1 部署根布局与配置树物理组织（2026-10-11 收官）

### C1.0 边界与前提

本题定**一切路径的锚点**：部署根选定方式、目录骨架、配置树文件面。不归本题：entry 树字段集（C2）、补丁分层与 diff 协调（C3/C4）、workspace 绑定语法（C5）、secrets store 内部形态（C6）、数据生命周期（C7）。

### C1.1 D29 部署根布局（2026-10-11 用户拍板）

- **部署根 = 配置树文件所在目录**：`--config path/to/metis.yml` 选定文件即选定根（ADR-0007 已立 `--config` 机制），`plugins/`、`lib/`、`.luaurc` 全部相对它解析。淘汰专门 `--deploy-root` 旗标（零新概念）。
- **目录骨架**：

```text
<部署根>/
  metis.yml                 # 配置树正本（人类手写层；惯例名，--config 可指他处）
  overlay.yml               # agent overlay 层（机器唯一可写的配置树文件，C3 详谈）
  .luaurc                   # @lib → ./lib/（见下）
  secrets.yml               # secrets store（豁免①；内部形态归 C6）
  lib/                      # 部署级契约包（luau-abi §A8.2 三来源之二）
  journal/                  # journal 段文件 + blob 预留（豁免②；细节归任务 6）
  plugins/
    foo/
      manifest.yml          # 可选：无 = 零仪式插件（D35）
      config.yml            # 部署区：配置值稀疏覆盖（D36）
      main.luau             # 入口（manifest entry: 可改）
      lib/                  # 随包契约（既定）
      boxes/                # 盒制品（既定，wasm 里程碑）
      data/                 # 运行期区
        fs/                 # private scope 根（ctx.fs 挂载点）
        db.sqlite           # 私有 db（luau-abi §A3.9）
```

- **插件私有数据同居插件目录**：fs 私域根 = `data/fs/`、`db.sqlite` 与其平级——满足 luau-abi §A3.9「db 不在 fs 私域子树内」。淘汰集中数据根（`data/plugins/<name>/` 分居）：同居让「一个插件的全部」= 一个目录，卸载处置（C7）与备份/迁移粒度顺手。
- **global scope 豁免清单物理落点** = `secrets.yml` + `journal/` 两处（luau-abi §A3.7 两条豁免）；其余部署根 global 正常可达（维护/调试正道不变）。
- **版本管理纪律**：部署根可整体入库（部署即代码）；gitignore = `secrets.yml`、`journal/`、`plugins/*/data/`；sdk 脚手架生成 `.gitignore`。
- **`.luaurc` 布线落位**（兑现 luau-abi §A10.5 D28.5）：核心启动时检查部署根 `.luaurc`——不存在则自动生成（内容固定 = 含 `@lib` → `./lib/` alias，幂等）；存在则校验该 alias，不符 = 响亮报错指路 `metis sdk`。自动生成的理由：Creator 模式零仪式精神（文本即插件不该被工具链布线文件卡住）；`.luaurc` 是工具链布线不是配置层，不违反「机器只写 overlay」纪律。

### C1.2 D30 配置树单文件制（2026-10-11 用户拍板）

v1 = 单文件 `metis.yml` + overlay 一层，**不立 include 机制**。Cordis 的 Include（外部文件挂载 + 运行时写回）主要服务 bundle/profile 组合分发——bundle/profile 全套已判「包分发时代 machinery」（ADR-0004 / session-model §3），多来源问题已由 overlay 分层解决。登记扩展位，触发器 = 真实的文件拆分/分发需求（任务 9 时代复审）。

### C1.3 联动登记汇总（C1）

| 去向                              | 内容                                                                                   |
| --------------------------------- | -------------------------------------------------------------------------------------- |
| luau-abi §A10.5                   | D28.5 `.luaurc` 布线——**已兑现 = D29**（本 change 修订注同步）                         |
| luau-abi §A3.7                    | private 根物理落点 + 豁免清单具体化（本 change 修订注同步）；workspace 绑定机制仍归 C5 |
| luau-abi §A3.9                    | db 物理位置落定（本 change 修订注同步）；卸载保留政策仍归 C7                           |
| service-registry §7               | 部署根布局草图 SSOT 移交本节（本 change 修订注同步）                                   |
| C3                                | overlay 文件形状与补丁语义；`config.yml` 机器写回的注释保留/格式稳定纪律               |
| C6                                | `secrets.yml` 内部形态；轮转感知（D37 登记：显式 reload / boot）                       |
| C7                                | 卸载处置（同居目录的处置粒度）                                                         |
| 任务 6（journal/replay 正式设计） | `journal/` 段文件与 blob 细节                                                          |
| sdk 工具面                        | `.gitignore` 脚手架；`.luaurc` 生成/校验（D28.5 既定）                                 |
| 扩展位                            | include 机制（D30）                                                                    |

## C2 entry 树语法（2026-10-11 收官）

### C2.0 边界与前提

本题定**装配单的形状**：顶层形状、entry 字段集、休眠/未装配语义、插件形态单化、配置通道新形态、reload 原则。不归本题：补丁分层与写回纪律（C3）、diff 协调细则与 D37 机制展开（C4）。

### C2.1 D31 顶层形状 = `plugins` map，key 即身份（2026-10-11 用户拍板）

```yaml
# metis.yml —— 纯装配单
plugins:
  memory:        # null ≡ {} ≡ 全部缺省（启用、无字段）
  scheduler: {}
  llm-openai:
    enabled: false
```

- **map 形**，淘汰 Cordis 的 list + `id` 形（`cordis.yml` = entry 列表，每行带自动生成 id 供 keyed diff 定位）：① 身份 = 路径名且部署内唯一（§A2.1）——key 天然可作 diff 键，`id` 机制整个不需要；② key 即身份，「name 显式值必须等于目录名」的报错面按构造消失（无 name 字段可写错）；③ keyed diff 相等性平凡（ADR-0002 预言兑现）；④ 重复声明 = YAML map 重复键 = parser/校验层响亮。
- **归一规则**：entry 值 `null` ≡ `{}`（全部缺省）。校验器归一；机器写 overlay 用 `{}`（diff 可读性）。
- **加载顺序不靠声明序**（inject 拓扑驱动，ADR-0001）；受限 YAML 子集里 map 键序无语义。
- key 必须是合法目录名（禁 `/`、`.`/`..` 等）；校验进 loader/`sdk check` 同一实现（§A2.2 校验器单一实现、多面暴露哲学）。
- **已知牺牲**：Cordis `id` 的真实职能 = 同插件多实例（同一份代码、两份配置、各挂一份）；v1 答案 = 复制目录（`llm-openai-a/`/`llm-openai-b/`），代码重复与升级改 N 份的代价明记；复审触发器同 D32（任务 5 子树）。

### C2.2 D32 entry 字段集 = `{ enabled }` 封闭集（2026-10-11 用户拍板）

| 字段      | 缺省   | 一句                                                                  |
| --------- | ------ | --------------------------------------------------------------------- |
| `enabled` | `true` | `false` = 休眠件：部署索引收录、不加载不对账不激活（兑现 §A9.4 登记） |

- **正面词 `enabled`（缺省启用）**，淘汰 Cordis 的 `disabled`（其 detached 行是准入失败产物，我们无此概念）；翻转 = 拓扑变更走 C4 diff。
- **未知字段 = 响亮报错**（与 manifest 政策 §A2.1 同款 typo 防线）。
- **config 不在 entry 树**：值住处 = 插件目录 `config.yml`（D36）。
- **不立并登记触发器**：`isolate` / 部署侧重命名（§A2.3 撞名本地解后两条）、`intercept`（Cordis：给某服务名下所有后代合并配置）、`group`/嵌套子树——四者都依赖**子树**概念，而子树 = 任务 5（session 模型）preset 议题；v1 撞名本地解 = 部署者二选一（`enabled` 足够表达），全程不改源码。触发器 = 首个真实撞名需求或任务 5 子树落地，同批复审。§A2.3 登记就此结案。
- C5 绑定字段若立 = additive 增补，不受封闭集阻碍。

### C2.3 D33 顶层节封闭集（2026-10-11 用户拍板）

顶层节 = 封闭集，**未知节 = 响亮报错**（typo 防线同 manifest）。v1 已知节：`plugins`；登记位：绑定节（C5）、核心节（C8，audit 降档键住处）。配置树是部署者文件，演进 = 新节 additive + 老核心未知节响亮自然兼容，不立版本号机制。

### C2.4 D34 休眠件与未装配提醒面（2026-10-11 用户拍板）

- **休眠件**（树里有、`enabled: false`）：显式声明态、审批 diff 可见——**运行期不提醒**（不是 §A9.2 D25.1 那种「可能无期限的人为 Pending」）。禁用不丢配置：`config.yml` 住插件目录（D36），重新启用配置还在。
- **未装配件**（目录在、树里没有）：**运行期不吵**（配置树是唯一声明面，目录里多个目录不是核心的管辖事项）；`sdk check` 提示级列出未装配目录（写码期教具，不进运行期日志面）。
- **部署索引覆盖 = 目录扫描全量**：启动/安装扫描 `plugins/` 全量目录解析登记（休眠件与未装配件带状态标记，纯数据非判定——D24 原位）；依赖方 inject 未装配件 = 启用期响亮报错，与「查无」区分、指路「目录存在但树无 entry」；未装配件解析失败同休眠件政策（索引标记损坏 + host warn 一条，登记启用时响亮报错）。

### C2.5 D35 插件形态 = 目录单形态（2026-10-11 用户拍板）

- **淘汰单文件形态**（`plugins/foo.luau`）：一切插件 = 目录 `plugins/foo/`。ADR-0004 名字解析 4 条规则缩为 1 条（目录存在即可）；歧义格（`foo.luau` 与 `foo/` 共存）按构造消失；成长路径从「搬家」变「加 `manifest.yml`」，入口文件不动（§A1.3 零改写标准原样成立）。
- **manifest 保持可选**：无 manifest = 零仪式插件（原纯代码 niche 平移：无 config schema、无能力门资格、无 provide/inject——§A1.3 差异面原样，仅物理形态变）。Creator 模式产出 = mkdir + 写一个文件，仪式不增。
- **目录内三类文件规则**：包内容（manifest/代码/`lib/`/`boxes/`——升级替换、分发内容）/ 部署区（`config.yml`——升级保留、分发排除）/ 运行期区（`data/`——升级保留、分发排除、gitignore）。Creator 审批事务 diff 视野排除 `data/` 子树（运行期产物不进审批面）。
- **联动**：ADR-0004 局部 supersede（两形态格）归任务 4 冻结 ADR（Status 行 interim 注已同步）；§A1.3/§A2.1 修订注（本 change 已同步）。

### C2.6 D36 配置通道 = `config.yml` + 注入签名不变（2026-10-11 用户拍板）

- **值住处**：`plugins/<name>/config.yml`（部署区，目录顶层）。private scope 根是 `data/fs/`——**private scope 结构性够不到自己的 `config.yml`**；持 `global` 的插件读得到文件，但读到的只是**引用而非真值**（真值住豁免的 secrets store，§A3.7）——任何 scope 下旁路都拿不到秘密真值，校验与秘密解析只能过注入通道。
- **`setup(ctx, config, deps)` 签名不动**（§A3.3 原样）：核心启用期读 `config.yml` → 按 manifest schema 校验（缺 required/类型错/多给键 = 响亮失败，**插件代码一行未跑**——ADR-0006 前置原样保全）→ 秘密引用解析（§A2.5 机制原样，写处/解析源 = `config.yml`；写明文 = 校验拒绝；真值只进该插件 VM）→ 默认值合成（稀疏覆盖叠加 schema default）→ 冻结表注入。
- **变化面**：审批单位 = 插件目录 diff（`config.yml` 与代码同单位，ADR-0008 管线不动）；overlay 收缩为装配补丁层（装/卸/启用/绑定，不再管配置值）；`volatile` 词汇退役（其消费者 = 核心快路径机器，由 D37 替代——哪些键可热应用 = 作者文档 + handler 自行判断）。
- **两档分工**：声明式配置（`config.yml` + manifest schema：部署者可写、审批可见、sdk 可验）vs 完全自管（无 `config:` 节 = 核心零参与；运行期状态/偏好走 `data/fs` 或 db——判别两问之②，session-model §3）。db/私域自管配置不进审批面；文档引导部署决策走声明式通道。
- **休眠件的 `config.yml` 不校验**（D24：不使用的插件不检查）；required 缺失 = 启用期响亮失败，报错指路 `config.yml` 路径 + 键名。
- **无 `config:` 节 ≡ 空 schema**：`config.yml` 存在且有键 = 一切键按「多给键」响亮拒绝（ADR-0004 旧格「配值 → 响亮失败」精神平移；零仪式插件同规则）。
- **sdk 联动**：从 schema 生成 `config.yml` 模板（脚手架）；多插件配置全景聚合查看命令（登记 sdk 工具面）。
- **联动（supersede 批次归任务 4 冻结 ADR）**：ADR-0002（配置值 keyed diff / volatile 机器退役）、ADR-0006（值住处 + volatile 标注）、ADR-0007（overlay 收缩 + 机器写面扩张）、ADR-0008（模型配置写入面）、luau-abi §A2.0/§A2.1/§A2.5/§A2.7/§A3.3（措辞与解析点）——Status 行 interim 注与 doc 修订注本 change 已同步。

### C2.7 D37 reload 权利归插件（原则；机制归 C4）（2026-10-11 用户拍板）

- `config.yml` 变更（审批事务提交 / boot 检测 / 显式 reload 命令）→ **默认重挂载**（安全缺省）。
- 插件订阅配置更新通知（带新值推送）→ **live-apply 或否决后重挂载，权利在插件**（Cordis `internal/update` waterfall 否决先例：插件可参与配置更新决策）。
- 人直接编辑文件 = 下次 boot 生效；fs watch 实时感知 = 扩展位，v1 不立。纯轮转 secrets store（引用不变的值轮换）不产生 `config.yml` 变更——感知 = 显式 reload / boot（v1），fs watch 同扩展位。
- 机制细则（通知形状、否决通道、与 C4 diff 协调的衔接）归 C4。

### C2.8 概念地基（2026-10-11 用户问答沉淀）

- **依赖为什么不住配置树**：依赖 = 作者意图，唯一住处 = manifest（`inject` 双列表 + `events` 双名单，§A2.5/§A8 已冻结）；配置树重复声明 = 重复税 + 漂移源（部署者声明的依赖与作者代码需求必然漂移）。**事件 listen ≠ 依赖**——listen 是弱耦合被动面（没人 publish 不是错误，观测类插件「听一切」是一等用法，§A10.3 D28.3 #5）；`inject` required 才是硬依赖（缺失 = 启用期 Pending/拒，§A9）。两条通道互补不替代。部署者在依赖面的正当角色 = 审批可见三栏（capabilities + inject + events listen，§A9.3 D26.2）+ 撞名时选实现（v1 = 二选一，D32）。
- **config 为什么住插件目录**：审批单位一体（agent 动配置 = 动插件，目录 diff = 代码 + 配置一体审批）；core agent-agnostic（配置值 = 插件的领域事务，核心不当保管人）；部署根自包含（一个目录 = 一个插件的全部）。平移而非退役的机制清单：schema SSOT（manifest）、校验前置（启用期、代码未跑）、秘密引用（写处平移）、默认值合成、类型生成（§A10.4 D19.3 原样）。
- **配置树 ≠ 配置值仓库**：配置树（装配单）只剩「装哪些、开不开、绑什么」；插件运行期攒的状态/偏好从来就不进任何 YAML（判别两问之②，session-model §3）。

### C2.9 联动登记汇总（C2）

| 去向                                   | 内容                                                                                                                                           |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| luau-abi §A9.4                         | entry 启用/禁用字段形态——**已兑现 = D32**（本 change 修订注同步）                                                                              |
| luau-abi §A2.3                         | isolate/重命名结案（不立 + 触发器，D32）——修订注同步                                                                                           |
| luau-abi §A2.0/§A2.1/§A2.5/§A2.7/§A3.3 | 配置树边界、身份规则、秘密解析点、config 键约定、setup config 来源——修订注同步（D36）                                                          |
| luau-abi §A1.3                         | 纯代码 niche 平移为无 manifest 目录插件（D35）——修订注同步                                                                                     |
| ADR-0002/0004/0006/0007/0008           | 局部 supersede 意向（Status 行 interim 注，本 change 已同步）；正式批次归任务 4 冻结 ADR                                                       |
| session-model §3                       | 机器写入约束随 overlay 收缩——修订注同步                                                                                                        |
| luau-abi §A9.1/§A9.2                   | 部署索引覆盖（目录扫描全量、含未装配件与解析失败政策）与 D25.1 行三收窄（= 休眠件；未装配件走行四同族响亮）——修订注同步（D34）；措辞归冻结批次 |
| C3/C4                                  | overlay 装配补丁语义；keyed diff 只管装配面（增/删/enabled/绑定）；D37 机制展开；部署 `lib/` 契约增删改重对账时机（§A8.5 D22.6）归 C4          |
| C5                                     | 绑定字段 additive 增补位（entry 树）；env 收紧 / cwd 政策（§A3.10/§A3.13 登记）                                                                |
| 任务 5（session 模型）                 | per-session/preset 实例级配置通道（`config.yml` 是部署级单份，张力登记）；子树落地时 isolate/重命名同批复审（D32）                             |
| 任务 9（marketplace）                  | 包内容 vs 部署区/运行期区三分随包形态定稿（D35）；升级后配置 vs 新 schema 对账报告                                                             |
| sdk 工具面                             | `config.yml` 模板脚手架；配置全景聚合；未装配件提示（D34）；key 合法目录名校验（D31）                                                          |
| 扩展位                                 | fs watch 实时感知（D37）；isolate/重命名/intercept/group（D32）                                                                                |
| 实现期清单                             | `config.yml` 读取/校验/秘密解析/默认值合成管线（D36）；`.luaurc` 自动生成与校验（D29）；gitignore 脚手架（D29）；部署索引目录扫描（D34）       |
