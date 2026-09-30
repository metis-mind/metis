# 0001. inject 运行期检查纪律

- Status: accepted
- Date: 2026-09-29

## Context

Luau 无编译期类型系统，静态契约无从谈起，运行期注册表是唯一能落地的响亮失败点（Harness 教训："误配置响亮失败"）。论文也承认 DSU 式前向状态迁移是 future work。

被淘汰的选项：TS interface 式静态契约（Luau 不存在编译期）；核心零停机热更（复杂度爆炸，且与"状态全在配置树"的设计前提冲突）。

## Decision

- 服务注册表记录 `key → provider + 版本 + 类型`；加载时校验 inject 依赖，缺失/重复**响亮失败**
- 依赖指纹（epoch）驱动：服务提供方更新时，消费者 fiber 按拓扑序重启
- Rust 核心自身更新 = **进程干净重启**（状态全在配置树 + 显式落盘）；零停机核心热更是后期优化，v1 不做

## Consequences

加载期运行期校验成为唯一契约强制点；epoch 指纹与拓扑序重启由 [ADR-0010](0010-fiber-core.md) 的 fiber 设计具体化。
