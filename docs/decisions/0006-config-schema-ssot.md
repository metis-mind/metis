# 0006. 配置 schema SSOT：纯数据 manifest

- Status: accepted
- Date: 2026-09-29

## Context

Rust 必须读懂 schema 的三理由——加载前校验（安装事务/审批门前置）、`volatile` 等价标注（loader 快路径）、审批展示（人类要知道每个 key 的含义/类型/默认值）。

被淘汰的选项：Luau 侧定义 schema（双端定义必然漂移）；Rust 不懂 schema（校验与审批无从下手）。

## Decision

config schema 是纯数据（`manifest.yml` 的 `config:` 节），**Rust 解析校验，Luau 只消费校验后注入的配置值**。Luau 侧不存在任何 schema 定义，双端漂移在结构上不可能。

manifest v1 字段：

```yaml
name: foo # 缺省 = 目录名
version: 0.1.0 # 可选，为将来包分发预留
entry: main.luau # 缺省 main.luau
description: ... # 审批展示用
config: # config schema
    api_key:
        type: string
        required: true
    refresh_interval:
        type: number
        default: 60
        volatile: true # volatile 标注 → loader 快路径就地提交
```

## Consequences

schema 校验成为 loader 与安装事务的公共前置；`volatile` 快路径由 loader 消费（配置协调设计见后续任务）。
