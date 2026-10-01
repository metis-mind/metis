# 0015. payload 与配置值的数据模型：自定义最小 Value

- Status: accepted
- Date: 2026-10-02

## Context

进程内派发不需要字节编解码——payload 以 Rust 值在 host 与各 VM 间传递，只在 VM 边界做 `Lua table ↔ Value` 转换。要定的是**数据模型**（哪些值合法），不是线路格式。约束来自三方对齐：Luau 类型系统、YAML 1.2 core schema（[ADR-0007](0007-yaml-config-subset.md)）、插件作者的 JSON 直觉。讨论见 `../design/context-events.md` §3。

被淘汰的选项：

- **`serde_json::Value`**：number 为 i64/u64/f64 三态，对 Luau（只有一种 number）无意义且令相等性变绕；他人的类型无法承载 config 侧约束；自定义成本本就极低（百行量级）
- **扩展版（v1 即含 `Int(i64)` / `Bytes`）**：真实需求但 v1 无用户，YAGNI——登记为兼容扩展位（加变体向后兼容），见首个真实用户再加

## Decision

- **自定义最小 `Value`**：`Null / Bool / Float(f64) / String / Array / Map<String, _>`——与 Luau 类型一一对应（数只有 f64、字符串 UTF-8），与 YAML core schema 无缝互落（int 进 Float，2⁵³ 以内精确）
- **config 值与事件 payload 共用同一 `Value`**（"数据大一统"）：schema 校验、审批 diff、volatile 快路径、事件日志全套基础设施只针对一种数据模型
- **payload 永远是纯数据**：函数、插件引用、协程永不进入——actor 隔离的地基；过 VM 边界即深拷贝，插件间无共享可变状态
- **时间戳纪律**：payload/config 中时间戳用整数毫秒或带小数的秒，不用纳秒（f64 整数精确上限 2⁵³ ≈ 9×10¹⁵，纳秒时间戳已超出；毫秒精确 ~28 万年）

## Consequences

- 闭环"事件 payload 值类型"这一遗留开放问题
- 边界转换细则（Lua table 的 Array/Map 判定、非字符串 key 处置）归 Luau ABI 设计
- 2⁵³ 精度天花板由双兜底承接：`Int(i64)` 扩展位 + "大整数走字符串"惯例；注意 `Int(i64)` 即便加入，进入 Luau 侧仍退化为 f64——纳秒级数据的正确姿势是 host 侧处理、插件看聚合结果
