# 0005. VM 拓扑：每插件实例独立 `lua_State`

- Status: accepted
- Date: 2026-09-29

## Context

- **卸载零残留**：`lua_close` drop 一切引用，语言层面保证 temporal composability。共享 VM 下共享模块的单例状态会跨插件泄漏（A 改模块全局态 B 可见；A 卸载状态残留）——Cordis 在 Node 上一直吞着这类泄漏，我们有机会从结构上消灭
- **模块缓存 = fiber**：缓存随 VM 生灭，插件内 HMR 无需闭包计算；仅共享 `lib/` 变更需要反向闭包定位
- **故障遏制**：每插件独立内存上限 + 指令中断，跑飞的插件饿死不了别人，与 fiber 错误遏制哲学对齐
- **字节码全局共享**：`luau_compile` 产物是字节数据，编译一次可加载进任意多个 state；内存大头（代码）不重复，回滚保留旧 chunk 也成立

被淘汰的选项：全局共享单 VM + 每插件独立全局表（Node 式；卸载残留风险、模块单例泄漏、内存无法按插件限额）。

## Decision

每个插件实例一个独立的 Luau VM（`lua_State`：解释器状态实例，含独立堆/全局表/GC，非重量级虚拟机）。

## Consequences

已接受的代价：

- 事件 payload 必须可序列化（serde 数据，不能传 Luau table 引用）——视为强制干净缝的优点；值类型设计见 `../design/context-events.md` §3
- 每实例经验估算几十 ~ 一百 KB（原型期实测），百级插件约 10–30 MB，长驻 server 可接受
- 跨插件 Luau 级数据共享不可能——本就该走服务，纪律被强制执行

执行模型（actor 语义、邮箱、串行循环）在此之上展开，见 [ADR-0011](0011-actor-execution-model.md)。
