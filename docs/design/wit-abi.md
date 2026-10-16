# Metis — WIT ABI 设计（living doc）

> 状态：living design doc——2026-10-15 开章（[ADR-0034](../decisions/0034-wasm-first-carrier.md) wasm-first 转向同批）。语义地基平移自 [luau-abi](luau-abi.md)（已归档，仅供考古）；W 系列章节随设计批逐章收官，章末联动登记汇总惯例同 A 系列。
> 前提：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）；接缝能力集 = [seam-capabilities](seam-capabilities.md)；遏制四约束 = [ADR-0005](../decisions/0005-vm-topology.md)；边界拷贝语义四层 = [wasm-research](../research/wasm-research.md) 补遗 5 §5.4。
> 拍板锚（2026-10-16 两轮）：接口数据三分法 / E3 并发投递 / E4 fs 三层——SSOT = 台账 T13 状态行，议题背景 = [wasm-first-design-review](../research/wasm-first-design-review.md) §6/§7；W 各章收官时吸收进正文，此行随吸收删除。
> Last reviewed: 2026-10

---

## 0. 术语锚点

（待首章填充；共享术语见 [seam-capabilities](seam-capabilities.md) §0）

## 章节目录（W 系列，随设计批逐章开章）

| 章  | 题                                                                                                                                                                             | 前身（A 系列 = 语义平移源） | 状态   |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------- | ------ |
| W1  | 插件包形态与入口契约（world 定义 / export setup·dispose）                                                                                                                      | A1                          | 未开始 |
| W2  | manifest 格式                                                                                                                                                                  | A2                          | 未开始 |
| W3  | ctx 形态：WIT import 接口集与 SDK 映射（作者不见 WASI/WIT = ADR-0034 §4）                                                                                                      | A3                          | 未开始 |
| W4  | async 与交错语义（组件模型 async 映射；**最高风险章**；开工输入 = S1 spike 结果 + E3 拍板（最大化并发投递、宿主零同步原语）+ 同实例重入 spike（开工首项，补遗 6 papercut 3）） | A4                          | 未开始 |
| W5  | 错误与 result 习语（`result<T,E>` 原生）                                                                                                                                       | A5                          | 未开始 |
| W6  | 边界转换：canonical ABI / 递归 Value 线编码（TLV 主线化 = E1 选项① 拍板）/ resource 句柄族                                                                                     | A6                          | 未开始 |
| W7  | 契约与类型来源（① 平台封闭面 = WIT 富类型原生；A7 半消融 = E5 拍板：瘦身形状 DSL 继承 Value 形状面）                                                                           | A7                          | 未开始 |
| W8  | 事件静态声明                                                                                                                                                                   | A8                          | 未开始 |
| W9  | deps/provide 与能力门（进口对账机械化）                                                                                                                                        | A9                          | 未开始 |
| W10 | 工具形态与 ABI 版本纪律（WIT package version 承接 abi_rev）                                                                                                                    | A10                         | 未开始 |
