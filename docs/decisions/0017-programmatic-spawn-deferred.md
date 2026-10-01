# 0017. 编程式 spawn v1 推迟

- Status: accepted
- Date: 2026-10-02

## Context

Cordis 允许插件运行时编程式挂载子插件（`ctx:spawn`/`ctx.plugin`）。这构成"插件如何存在"的第二条路径——第一条是声明式 entry 树。每多一条存在路径，创建/配置/热更/卸载/持久化/审计/审批整套生命周期问题就要重新回答一遍，且两路径的交互是组合爆炸。Harness 的模型运行时 eval 定义插件正是这样一条第二路径，已被 Harness 自己退役（`../research/cordis-research.md` 第四部分 §4(d)：进程内临时定义造成"第二条插件生命周期"）；编程式 spawn 与该教训同族。讨论见 `../design/context-events.md` §6。

被淘汰的选项：

- **v1 开放通用 spawn API**：插件集合在运行时绕过配置、绕过审批自行变化——对自我进化 agent，这正是最需关进笼子的能力；且机制已备（子 fiber = 父的 effect，[ADR-0010](0010-fiber-core.md)），推迟零成本

## Decision

- **v1 插件存在路径唯一**：声明式 entry 树；不提供 `ctx:spawn` API
- **动态性的边界划在"是否产生新的生命周期单元"**：插件内部动态需求由内部任务覆盖（fiber effect 账本挂后台任务/定时器，随卸载自动取消）；动态造**插件**（新 VM、新生命周期单元）v1 不做
- **汇合点已登记**：preset 的 session 子树挂载 = spawn + 子树作用域的交集，在 session 模型设计（`../design/session-model.md`）时与 isolate/intercept（[ADR-0013](0013-context-scope.md) 推迟项）一起回来——spawn 的第一个真实用户到来时在它的设计场合专门定

## Consequences

- 生命周期路径唯一，插件集合永远可审查、可 diff、可审批；爆炸半径收在配置闸门内
- **会话子树挂载不违反本 ADR**：preset 是声明（与 entry 树同语法），由 host 在 session 创建时按声明实例化——不是"插件创建插件"
- agent 自我进化的正当出口 = preset revision 通道（机器可写、走审批），见 `../design/session-model.md` §6；这比运行时 eval 造插件安全得多，是 Harness 教训的正面答案
- 将来真开放通用 spawn，是纯增量（fiber 父子关系在 arena 里本就有），不破本 ADR 确立的默认路径
