# 0022. Value 增加 Int(i64)：Luau 原生整数对接

- Status: accepted
- Date: 2026-10-03

## Context

[ADR-0015](0015-payload-value-model.md) 冻结最小 `Value`（`Null / Bool / Float(f64) / String / Array / Map`）。当时否决 `Int(i64)` 的两条论据：v1 无用户（YAGNI——被淘汰选项"扩展版（v1 即含 `Int(i64)` / `Bytes`）"一格）；`Int(i64)` 即便加入，进入 Luau 侧仍退化为 f64（Luau number 全 double——Consequences 末条）。

2026-02-12 Luau 上游合并 64 位整数 RFC（`luau-lang/rfcs#153`；修订系列至 2026-03-25）：原生 `integer` 类型 + 完整 `integer` 库 + C API（`lua_pushinteger64` / `lua_tointeger64`）+ buffer / string / 类型系统集成——**退化论据消失**。

用户拍板（2026-10-03）：everything-is-a-plugin 严格模型（[ADR-0021](0021-extension-language-runtime.md) 产品模型前提）下插件需自行实现真实功能（API 签名 / 哈希 / 大 ID / 时间戳），整数是必备能力，不接受绕行——**YAGNI 论据同步消失**，首个真实需求类别已明确。

出厂状态说明：RFC 合并 ≠ 随发布版出厂；实现状态由 mlua spike 实测确认（[ADR-0021](0021-extension-language-runtime.md) 运行时语义上限条）。

## Decision

局部 supersede [ADR-0015](0015-payload-value-model.md) 的四处（该 ADR 的 Status 行已注记）：

- **`Value` 增加 `Int(i64)` 变体**——取代被淘汰选项中"扩展版（v1 即含 `Int(i64)` / `Bytes`）"一格的 `Int(i64)` 半；`Bytes` 不动，仍留扩展位
- Decision 第一条整格（变体枚举 `Null / Bool / Float(f64) / String / Array / Map`、"数只有 f64"、"与 YAML core schema 无缝互落（int 进 Float，2⁵³ 以内精确）"）——`Value` 数改双轨：`Float(f64)` + `Int(i64)`；YAML core schema（[ADR-0007](0007-yaml-config-subset.md)）整数 → `Int(i64)`（不再进 `Float`）；小数 → `Float`；**超 i64 界 → 响亮失败**（不静默降精度，与响亮失败哲学一致）
- Decision"时间戳纪律"格——**纪律不变**（整数毫秒或带小数的秒，不用纳秒；纳秒连 i64 也撑不过 2262 年）；2⁵³ 精度 rationale 失效，由原生整数承接
- Consequences 末条整格（"2⁵³ 精度天花板双兜底"与"退化为 f64"论述）——Luau 边界映射 = 原生 `integer`（C API `lua_pushinteger64` / `lua_tointeger64` 已定义）；超 i64/u64 范围的大数仍走字符串惯例；纳秒级数据 host 侧处理的习惯不变

同时登记：

- **过渡纪律**：若 vendored Luau 当前版本未出厂整数，边界暂按 2⁵³ 内映射为 number、超界响亮失败；上游出厂后切换原生映射（ABI 版本纪律归 tooling 议题）

## Consequences

- config 与事件 payload 共用同一 `Value` 的"数据大一统"不变
- 边界转换细则（table ↔ `Value` 的 integer 映射规则）由 Luau ABI 设计吸收
- schema DSL 需含 int64 类型标注
- `Bytes` 变体维持扩展位登记，未被本次驱动
