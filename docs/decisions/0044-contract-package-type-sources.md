# 0044. 契约包与类型来源（wasm-first）

- Status: accepted（新设计；整篇 supersede [ADR-0019](0019-crate-layout.md) C3 的 lib 模块体系；局部 supersede [ADR-0018](0018-service-registry.md) S6 的 manifest 方法集与「插件服务文档约定起步」两格；ADR-0019 C1/C2、ADR-0018 其余条款、ADR-0006/0007 不动）
- Date: 2026-10-19

## Context

wasm-first 转向后，作者是编译型 guest，契约类型需要在两侧同时可用：宿主侧用于形状校验、对账与级联，作者侧用于写代码。A7 的契约体系建立在 Luau 载体的三个前提上：表达式串 DSL 需要表达 `buffer` / `function` 等 ctx 面词；`lib/` 是运行期模块系统；类型生成目标是 `.d.luau`。这三个前提随载体消亡。

E5 拍板保留瘦身形状 DSL 服务 Value 形状面（A7 半消融）；E1① 拍板开放数据面走 TLV `Value`。W7 需要回答：契约包是什么、类型从哪来、作者代码里的类型怎么得到、运行期如何校验、契约如何演化。另一个被重新审视的问题是 `Value` 的存废：WIT 是静态、非递归、无 `map` / `any`；宿主的动态路由、开放事件、contractless 数据、config 通用树与 journal 脱敏仍需要一个自描述的动态表示。结论 = `Value` 保留，但收窄为动态数据面 + 宿主交换格式；契约已知的作者面由生成类型遮蔽。

被淘汰的选项：

- **契约包 = WIT 包**：WIT 无 `map` / `any` / 递归；事件名不是 WIT 标识符；config 的 `required` / `default` / `secret` 无落点；线上仍是 TLV `Value`，WIT 只增加一层映射，不减少机器。
- **WIT + YAML sidecar**：两套来源（类型住 WIT、事件名 / config 属性住 YAML），加 WIT ↔ Value 编解码层；比单一 YAML 契约多，不推荐。
- **删除 `Value` / 全静态化**：动态服务路由、开放事件、contractless 服务 / 事件、config overlay / diff、journal 通用脱敏全部失去表示；且配置层与 journal 仍会需要一个内部通用树，`Value` 不会真正消失。
- **Rust 类型 → schema 反向生成**：违反 ADR-0006（宿主在跑插件代码前读 schema）与 Seam 三角（契约独立于提供方）。
- **check-in 生成类型**：SSOT 双写与生成物腐化面（plugin-sdk S7 D104 已淘汰）。

## Decision

1. **类型来源三分**（W7 D163）：平台封闭面 = WIT 富类型；开放数据面 = TLV `Value` + 瘦身 DSL + 生成类型；大负载 = blob / reader。`Value` 保留为动态数据面 + 宿主交换格式；作者面契约类型 = 生成物；`any` 是唯一回落。契约类型非递归（引用成环 = 响亮）。
2. **契约包形态**（D164）：`lib/<contract-name>/contract.yml` + 可选 `README.md`；纯数据、无 WIT 面；身份 = 目录名；不设 `name:`；部署根与插件包同形；一个插件可带多份随包契约。**编译期期望的载体 = 随包契约**：使用 `types!()` 的插件必须把所用契约放入包的 `lib/`；该副本同时是运行期「二进制按哪个形状编译」的声明，同名不同形状（两个具体形状不等）= 启用期响亮。
3. **三来源与版本**（D165）：平台内嵌（版本 ≡ 核心）/ 部署级生态契约（`version:` 必填，引用 `name@N`）/ 插件随包（禁写 version，裸名）；版本 = 单整数、破坏性变更（含删除条目）才 bump；版本整数是声明与 `@N` 门，兼容性判定 = 规范化形状比较；同名规范化相等共存、两个具体形状不等 = 启用期响亮；按契约名引用，永不按插件名。
4. **瘦身 DSL**（D166 / D167）：YAML 结构 + 引号表达式串；词汇 = `nil` / `boolean` / `string` / `integer` / `number` / `bytes` / `array<T>` / `map<T>` / record / union / `T?` / `any` / `Contract.Type`；删除 `buffer` / `function` 与 ctx 签名面；`bytes` 直接或经引用展开后禁入 config `type:`；`nil` 恢复合法；字符串字面量 = 精确值；record 字段归一化含 `nil`（`T?` / `T | nil`）时键缺席或 Null 同义；任何含 `any` 的 union 归一化为 `any`（`nil` 除外；`any | nil` 保持 optional）；四节 `types` / `methods` / `errors` / `events`；方法集住契约；`provides.contract` 缺省 = contractless；business data 形状不进运行期校验。
5. **规范化与解析**（D168）：A7 D18.3 四条 + 类型引用展开（A8 D22.2）+ `any` 吸收（`nil` 除外）/ 递归相容 / 具体胜出；显式引用查无 = 响亮，缺省 `contract:` = contractless；缺失两分（查无 = 响亮 / 未启用 = Pending）；判定在启用期；解析冻结。
6. **运行期校验**（D169）：服务 args / returns 与事件 payload 在契约已知时 walk；违约 = `core-contract-violation` Err + journal；blob 车道不受契约约束（opaque，形状校验跳过）；contractless 不校验；方法存在性以契约为 SSOT；record 可选字段 / 字符串字面量按 D166 语义校验。
7. **作者类型投影**（D170）：`types!()` 编译期只读包内 `lib/<contract-name>/contract.yml`，加 manifest config schema，生成 Rust 类型；不立外部路径 / registry / 网络；typed decode 直走 TLV；无 check-in；反向生成与 WIT 投影不立（后者 = 扩展位）。
8. **生命周期与级联**（D171）：引用冻结；新增吸收；删除 / 形状变更 = 破坏 → 契约版本 bump；**两个兼容面** = 动态消费者走反向索引级联重启用，编译期消费者（随包旧契约）启用期响亮、必须重编译（重启用不能重建二进制）；部署 `lib/` 走 boot / reload；随包契约随插件更新；卸载降级写实；冻结形状图按部署共享；blocking-uninstall = 扩展位。
9. **ADR 处置**（D172）：ADR-0019 C3 整篇 supersede；ADR-0018 S6 局部 supersede；Seam 三角、框架级契约必须有包、加方法兼容 / 删改破坏规则存活；ADR-0006 / 0007 不动。

## Consequences

- [wit-abi](../design/wit-abi.md) W7 章（D163–D172）冻结完整设计；联动清单 = W7.12。
- 实现站：T10.3（契约解析 / DSL / 规范化 / walk / 合并 / 反向索引）、T10.14（`types!()`）、T10.7 / T10.12（对账 / 级联执行）、T10.9（索引与检测）。
- 契约包不再包含 WIT；`lib/` 只承载数据。WIT 只住 `crates/metis-plugin/wit/`（平台固定 world）。
- 作者面：写 YAML 契约与 manifest config schema，用 `types!()` 生成类型；不需要写 WIT，不需要手写 `Value`（`any` 除外）。契约副本随包，既是类型来源也是编译期期望声明。
- 成本账：自研 parser + 生成器 + 规范化比较器；换来单一来源、`map` / `any` 表达力、事件动态名与 config 属性同文件。代价明写：契约破坏性变更对编译期消费者 = 重编译，不是热重启用。
- 扩展位：契约 → WIT 投影；per-method / per-event blob 声明；blocking-uninstall；嵌套 `secret`；README lint；构建期服务名 ↔ 契约绑定检查；`metis sdk export`（平台契约副本导出）；跨部署契约发现性（任务 9）。
- 复审触发器：契约 DSL 摩擦成串（作者要求标准 IDL / 外部工具互操作）→ 评估 WIT 投影或重新审视契约语言；`Value` 动态面无真实消费者 → 重新审视存废。
