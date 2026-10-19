# 0042. Value 增加 Bytes：wasm 二进制数据模型

- Status: accepted（局部取代 [ADR-0015](0015-payload-value-model.md) 的「扩展版」淘汰格 Bytes 半与 [ADR-0022](0022-value-int64.md) Consequences「`Bytes` 变体维持扩展位登记」行；两份 ADR 的决策本体不变）
- Date: 2026-10-19

## Context

[ADR-0015](0015-payload-value-model.md) 冻结最小 `Value` 时，把 `Bytes` 与 `Int(i64)` 一起留在「扩展版」淘汰格里，触发器 = 首个跨插件二进制消费者。当时的成本结构是 Luau：`Bytes` 提正需要 buffer 映射、schema 类型词与编码纪律，收益不足以抵消复杂度。[ADR-0022](0022-value-int64.md) 只提正了 `Int(i64)`，`Bytes` 继续留扩展位。

wasm-first 转向（[ADR-0034](0034-wasm-first-carrier.md)）改变了成本结构：WIT `list<u8>` 原生，跨插件二进制与文本同权（[wasm-first-design-review](../research/wasm-first-design-review.md) B6）。与此同时，跨插件二进制在数据模型里仍没有合法载体——`Value::String` 强 UTF-8，把二进制塞进字符串会破坏 UTF-8 契约。继续等「首个消费者」只会让形状在实现期仓促定，且触发器本身是按旧成本结构设的。

边界形态的完整设计 = [wit-abi](../design/wit-abi.md) W6 章（D157/D158/D162）；本 ADR 只冻结 `Value` 模型的变更面。

## Decision

- **`Value` 增加 `Bytes(Arc<[u8]>)` 变体**。`Value` 保持 `Send + Sync`；大二进制从第一天 `Arc` 持有，避免宿主侧多处引用时复制。
- **TLV 线格式增加 bytes tag**；动态解码优先借用（`ValueRef::Bytes(&[u8])`），需要持有时才 `Arc` 化；编码确定（同一 `Value` 树 → 同一字节序列）。
- **范围纪律**：`Bytes` = 数据模型内的二进制（事件 / 服务参数 / business data），受 TLV 条目 / 深度 / 总字节预算约束；**大二进制走 W6 的 `blob` 车道**，不塞 `Bytes`。
- **journal**：JSONL 落盘表示 = base64 包裹对象（与 D77 Int 包裹同族）；实现归 T10.5。
- **config**：YAML 没有二进制类型，config 解析不产生 `Bytes`；`Bytes` 只从程序 / 事件 / 服务路径产生。
- **局部取代**：[ADR-0015](0015-payload-value-model.md) 的「扩展版（v1 即含 `Int(i64)` / `Bytes`）」淘汰格 Bytes 半，以及 [ADR-0022](0022-value-int64.md) Consequences「`Bytes` 变体维持扩展位登记，未被本次驱动」行。两份 ADR 的 Decision 本体与 `Int(i64)` 结论不变。

## Consequences

- [ADR-0015](0015-payload-value-model.md) / [ADR-0022](0022-value-int64.md) Status 行记录 partial supersede；正文冻结。
- `context-events` §3 方案 C 行改为「Bytes 已提正 = ADR-0042」；T10.1 站表与遗留问题登记的「Bytes 扩展位」行删除。
- T10.13 TLV codec 增 bytes tag、借用解码与 round-trip 测试；T10.14 SDK 增 `Bytes` 类型；T10.5 journal 增 base64 包裹。
- 不新增能力、能力门或错误 kind；`Bytes` 预算超限按 W6 D162 的边界错误处理，kind 映射 = W5.2 的 W6 增补行（`core-contract-violation`）。
- 复审触发器：跨插件二进制出现超 TLV 预算的常态使用 → 应按 W6 引导改用 `blob`，而不是放宽 `Bytes` 预算。
