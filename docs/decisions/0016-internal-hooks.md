# 0016. internal/* 钩子处置：v1 开二、留二、删四、转一

- Status: accepted
- Date: 2026-10-02

## Context

`internal/*` 钩子 = 框架自身操作时派发的事件——钩子在 Metis 不是独立机制，就是事件（同一事件表、同一派发循环、同一邮箱路由），其特殊性仅在于"派发方是框架 + 占用保留前缀"。它是 everything-is-a-plugin 的关键一环：工具面板、调试器、agent 自我观测都只是订阅框架事件的普通插件，框架对 agent 与第三方插件一碗水端平。Cordis 留下 9 个钩子，其中 3 个是 Node Proxy 时代的遗物（属性拦截管道），按调研"不照搬 Node 特有部分"清单执行。讨论见 `../design/context-events.md` §4。

被淘汰的选项：

- **全盘保留 8 个**：含 Proxy 依赖钩子（显式 API 下无挂接基础）与零用户钩子
- **全删**：失去工具插件与自我观测的最小观测面
- **`internal/dispatch` 保留为钩子**："每次派发都触发事件"自身也触发派发，递归悖论需特判；且每事件一钩子是 hot path 开销

## Decision

| Cordis 钩子                                           | v1 处置                               | 理由                                                                                              |
| ----------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `internal/plugin` / `internal/status`                 | ✅ **开**（emit，只观测不介入）       | fiber 生命周期事件；工具插件/状态观测第一需求，成本≈零                                            |
| `internal/config` / `internal/update`                 | 💤 **机制留、钩子不开**               | 唯一内建用户是惰性求值（[ADR-0002](0002-config-as-pure-data.md) 已推迟）；`!!luau` 复活时同步启用 |
| `internal/get` / `internal/set` / `internal/listener` | ❌ **永久删除**                       | Proxy 属性拦截遗物；显式 API（[ADR-0001](0001-inject-runtime-checks.md)）下没有可拦截的"属性读写" |
| `internal/service`                                    | ❌ **删除**                           | inject 重激活由 epoch + 反向索引在 Rust 层驱动，不需插件可见钩子                                  |
| `internal/dispatch`                                   | 🔄 **替换为 tracing instrumentation** | 观测需求由日志平面满足；递归悖论与 hot path 整类消失；落实 "Model-visible ⟺ logged"               |

- **命名纪律**：`internal/` 前缀保留给框架；插件事件自由命名，文档建议 `plugin-name/event` 约定（不强制）
- v1 的 internal 钩子全是**观测型**（emit）；介入型机制留而未开

## Consequences

- v1 观测面小（仅生命周期），派发观测由 tracing 补；新增 internal 钩子永远是纯增量操作，不破兼容性
- 事件发现性开放点（manifest 是否静态声明 emits/listens + payload schema → 事件目录/校验）归 Luau ABI 设计（插件 manifest 格式）时定
