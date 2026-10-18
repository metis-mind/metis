# 0040. 配置共享变量：values.yml + secrets.yml 统一引用

- Status: accepted
- Date: 2026-10-19

## Context

配置值住各插件自己的 `config.yml`（[ADR-0027](0027-config-format.md) D36），审批单位 = 插件目录 diff。代价是同一部署内多个插件共用的值（base URL、默认模型、同一 API key）需要重复填写，修改时逐处发散。

Cordis / DSH 的先例仍是 per-entry config：Cordis 的共享手段是 `intercept` 按服务名合并后代配置与 `!!js` 表达式；DSH 用 profile / bundle 层叠 patch。两条都依赖子树合并或可执行表达式，本项目已在 [ADR-0002](0002-config-as-pure-data.md)（配置纯数据）与 config-format D32（intercept 不立）淘汰。

`secrets.yml` 已是部署级共享命名空间（[ADR-0027](0027-config-format.md) C6）；非秘密共享值没有对应机制。W2 的初步登记只覆盖非秘密值；维护者 2026-10-19 明确：共享变量必须覆盖全部值，不论是否 secret。

## Decision

**共享变量 = 一个部署级命名空间 + 两个物理 store + 一条引用语法。**

### 1. 两个 store

- **`values.yml`**：非秘密共享值；部署根文件；git 追踪；人类手写；扁平 map `name → Value`（scalar / record / array）。
- **`secrets.yml`**：秘密共享值；现有 C6 形态不变（扁平 map，值仅字符串；gitignored；机器永不写；权限锁与脱敏纪律不变）。
- **统一命名空间**：名字部署级唯一；同名同时出现在两个 store = 响亮错误（无法判定意图）。
- 文件缺席 ≡ 空 store；引用查无 = 响亮。

### 2. 引用语法

- 配置值写 `{ $value: "name" }`，表示整值引用一个共享变量。
- `$` 前缀键保留给引用标签；在字面量位置使用 `$` 前缀键 = 响亮（`$` 不是 DSL record 标识符，正常 schema 不会产生这种字段）。
- v1 只支持**整值引用**；嵌套字段引用 = 扩展位（触发器 = 首个真实嵌套复用需求）。

### 3. schema `secret` 标记选 store

- `secret: true` 的配置键：键存在时必须是 `{ $value: "name" }`，且名字必须解析自 `secrets.yml`；写明文 / 解析自 `values.yml` = 响亮。`required: false` 时键可缺席（缺席 = 无值，不做引用检查）；`required: true` 时缺席 = 响亮。
- 非 secret 的配置键：字面量合法；写 `{ $value: "name" }` 时名字必须解析自 `values.yml`；引用一个只存在于 `secrets.yml` 的名字 = 响亮（防止秘密流入未声明 secret 的键）。
- `default` 是字面量；`{ $value }` 写在 `default` = 响亮（否则会绕过 store 选择）。
- 解析后的值再按该插件 schema 做最终类型校验；secret 值仅字符串（C6.1），`type` 限 `string` / `any`（[wit-abi](../design/wit-abi.md) W2.4 D128）。
- 秘密值仍按身份注册脱敏管线（[journal](../design/journal.md) D89），只进属主实例；引用机制不改变这两条。

### 4. 管线与生命周期

- 顺序：读 `config.yml` → 按 schema 识别引用形状与 `secret` 标记 → 从对应 store 解析 → 最终类型校验 → 缺省值合成 → 冻结 → TLV → `setup`。插件代码一行未跑。
- `values.yml` 加载 = boot 一次性读入内存 map；reload 原子替换。变更感知 = boot / 显式 reload（无 fs watch，D37）。
- `values.yml` 解析失败：单租户 boot = 响亮拒启动（部署根文件读不出不猜着跑）；fleet 模式 = 该租户 Failed、其余照常；显式 reload = 拒绝本次 reload、运行中实例持旧 map 不动。单插件名字查无 / 类型不符 = 该插件启用期响亮，兄弟不受影响。
- `secrets.yml` 的轮转与解析失败两分 = C6.2 原样；共享引用不新增感知通道。
- `values.yml` 人类手写、机器永不写（v1）；agent 提议共享值变更 = 扩展位（审批单位扩展），不立机器写面。
- 审批面：`config.yml` 的引用名进插件目录 diff；`values.yml` 进部署根 git diff；`secrets.yml` 不进审批（人类手写层）。

## Consequences

- 共享变量覆盖 secret 与非 secret，不再有「只做非秘密」的半套机制。
- [wit-abi](../design/wit-abi.md) W2 D128 的秘密引用语法更新为 `{ $value: "name" }`；[config-format](../design/config-format.md) C1.1 / C2.6 / C6 同步，新增 C6.4 D135。
- 同步面：[journal](../design/journal.md) D89 的引用语法与 J2 的持久化表述、[instance-model](../design/instance-model.md) S2.3 fleet 配置文件清单与 S3.3/S4.1 配置管线、[build-pipeline](../design/build-pipeline.md) 部署根跟踪文件清单。
- 实现归 T10.8 配置读写管线（`values.yml` 读取、统一解析、类型校验、reload 触发）；无新增 crate。
- 代价写实：两个 store + 一个 `secret` 标记的匹配规则；引用错 store = 响亮（比静默注入安全，但作者要理解两档）。`values.yml` 暂无 agent 提议通道，共享值变更走人类编辑 + 审批外部署根 diff。
- 扩展位：嵌套字段引用；agent 提议 `values.yml` 变更；`values.yml` 的按租户覆盖（fleet 模式下每租户部署根天然独立，暂不立额外机制）。
