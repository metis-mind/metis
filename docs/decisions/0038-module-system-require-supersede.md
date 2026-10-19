# 0038. 模块系统与 require 纪律 supersede（wasm-first 承接）

- Status: accepted（整篇 supersede [ADR-0003](0003-module-require-discipline.md)；载体前提 = [ADR-0034](0034-wasm-first-carrier.md)；`lib/` 契约包的载体措辞由 [ADR-0044](0044-contract-package-type-sources.md) 更新为 `contract.yml` + 文档（2026-10-19），模块系统消亡的决策本体不变）
- Date: 2026-10-19

## Context

ADR-0003 在 Luau 载体下冻结了运行期模块系统与 require 纪律：require 包装器、`(importer, specifier)` 依赖边账本、失效集 = 反向传递闭包、插件内相对路径自由、跨插件 require 拒绝、共享 `lib/` 纯代码库与别名机制。其约束前提是「Luau VM 无模块自省、模块解析发生在运行期」。

wasm-first 转向后，guest 是编译产物：模块解析在编译期完成（Rust / Cargo 或对应语言工具链），运行期不存在模块加载、模块缓存、循环 require 与按边的失效传播。跨插件代码 import 在组件边界上结构性不存在；跨插件运行期组合仍只走 inject / events。`lib/` 的存活职能是契约包（纯数据：schema + 文档），不是代码模块（[wit-abi](../design/wit-abi.md) W1.1 D116；[config-format](../design/config-format.md) D35）。

design-review A2 已把 require 体系全链列为随载体消亡项；ADR-0003 是冻结 ADR，机制死亡须走 supersede，不能以修订注覆盖。

## Decision

**整篇 supersede ADR-0003。**

- **ADR-0003 正文条款死亡**：require 包装器、`(importer, specifier)` 依赖边账本、反向传递闭包失效、插件内相对路径 import 规则、跨插件文件 require 禁令的路径逃逸判定、共享 `lib/` 纯代码库、`@lib` 别名。
- **A1.5/A2 承接项死亡**：`.luaurc`、模块缓存、循环 require 报错、加载期「编译入口 chunk」步骤（组件编译 / 实例化由 wasmtime 承担）。
- **存活原则**：跨插件运行期组合只走 inject / events；装配层组合走 entry 树（`metis.yml` 装配单）（ADR-0003 的 rationale 存活，机制换载体）。同一插件内部的组织由 guest 语言的编译期模块系统承担；契约变更的级联重启用是数据驱动的反向索引（config-format D43 / W7），不是模块边失效。
- **`lib/` 定位**：部署级与随包契约包（WIT / 数据），由 loader 扫描进契约命名空间，不被 import 为代码。跨插件代码共享 = 编译期依赖（guest 包生态），不产生运行期耦合。
- **实现面无承接物**：不建模块解析器、依赖边账本、模块缓存或别名系统。

## Consequences

- ADR-0003 Status 行记 superseded by ADR-0038；正文冻结。
- [ADR-0019](0019-crate-layout.md) C3 的 `@lib` 别名与 lib 两层机制同属死亡；其正式 supersede 按站表归 [wit-abi](../design/wit-abi.md) W7（T13-m），本 ADR 只记录归属。
- 热更新语义不变：插件重载 = 组件实例重换（[wit-abi](../design/wit-abi.md) W4.5），不依赖运行期模块失效。
- `metis check` / `metis build` 校验组件 import / export 与 world 结构，不校验模块边。
- [luau-abi](../design/luau-abi.md) 保持归档，正文不动；A1.5 的 require 考古只在 git 史与归档中。
- 实现期无新增 crate；原模块系统相关实现项全部消失。
