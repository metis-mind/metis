# 0002. 配置即纯数据（v1 跳过 `!!luau`）

- Status: accepted
- Date: 2026-09-29

## Context

- 审批 diff 保持数据级可审（Creator 模式的审批门依赖这一点，见 [ADR-0008](0008-creator-mode-self-modification.md)）
- keyed diff 的相等性语义平凡；`volatile` 快路径干净
- 攻击面不扩大；能力不丢只丢便利
- Luau 沙箱已在，将来加回成本低

被淘汰的选项：v1 支持 `!!luau`（配置补丁从"一行数据"变成代码片段，审批 diff 变代码评审；表达式相等性/重评估时机使 keyed diff 语义复杂化）。

## Decision

v1 不实现配置内嵌 Luau 惰性求值表达式（对应 Cordis 的 `!!js` + `internal/config` 钩子）。配置树保持纯数据；运行时才确定的值写在插件 setup 代码里，需要共享时经 service 暴露。

## Consequences

`!!luau` 复活时，配套机制（惰性求值与 `internal/config` / `internal/update` 钩子）需同步设计启用。
