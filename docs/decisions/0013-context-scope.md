# 0013. Context 的 Rust 形态与作用域裁剪

- Status: accepted
- Date: 2026-10-01

## Context

Cordis 的 Context 是"服务容器 + 事件总线 + 作用域树"三位一体的大对象，实现手法（Proxy 属性拦截、原型链遮蔽、声明合并）全是 TS 特有表达，不可移植。且 Cordis 每个插件本就持有 `extend` 出的子 context——context 树与 fiber 树几乎同构，是重复建模。actor 模型（[ADR-0011](0011-actor-execution-model.md)）下跨插件一切交互经 host 两跳路由，host 站在服务解析的正中间：作用域规则的增删不需要插件侧配合。讨论全文见 `../design/context-events.md` §5。

被淘汰的选项：

- 照搬三位一体大对象：隐式属性拦截与显式 API 纪律冲突，实现手法不可移植
- 另造独立 context 树：与 fiber 树重复建模，两套父子关系需保持一致

## Decision

- **Context 是 fiber 视角的 runtime 轻句柄**：`Context { fiber: FiberId, runtime: RuntimeHandle }`。FiberId 携带身份——effect 归属是结构性的（插件一切调用自带身份，不可伪造、不可匿名）；RuntimeHandle 路由到 host 的全局唯一状态；句柄不持有任何框架状态
- **作用域树 = fiber 树本身**：parent 链已在 Fiber 数据结构上（[ADR-0010](0010-fiber-core.md)，记账需要），不另造 context 树；v1 服务解析 = 全局注册表单级查找，parent 链预留
- **作用域特性 v1 全裁**：`extend` 的两半意义分开处置——对象分叉（Cordis 靠它让每插件持有自己的 context）被 per-fiber Context 天然覆盖，无需加回；子树遮蔽 v1 无用户，加回路径 = 服务查找沿 parent 链先查自己。`isolate`（服务隔离域）与 `intercept`（后代配置合并）是声明式 entry 特性，推迟到配置格式设计一并定，与 inject 是否静态声明同族
- 强制复审点：配置格式设计；`../design/session-model.md` §4 的 session 模型设计

## Consequences

- 心智模型收敛为一句：每插件一个 Context、服务全局唯一、host 路由一切；Cordis 新人最大认知负担（三个作用域机制 + Proxy 行为）整体消失
- v1 跑不了两套互不干扰的服务栈（多账号 / 多 session 服务隔离）；测试 mock 粒度粗——只能全局注册表替换 + epoch 重启
- 加回路径均不破插件 API：`extend` 遮蔽 = 服务查找沿 parent 链先查自己；`isolate` = host 路由加一层 scope 匹配；`intercept` = 配置层补丁合并
- Luau 侧 ctx 的具体形态（userdata 方法集、await 写法）归 Luau ABI 设计
