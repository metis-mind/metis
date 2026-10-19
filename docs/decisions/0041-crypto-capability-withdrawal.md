# 0041. 纯算件归 guest 包生态：crypto 能力撤回

- Status: accepted（局部 supersede [ADR-0025](0025-seam-capabilities.md) §4 的 crypto 内建件项与 [ADR-0024](0024-carrier-layering.md) §4 中 crypto 行的「必须内建」结论；技术事实「wasm 无 AES/SHA 指令直通」保留）
- Date: 2026-10-19

## Context

crypto 内建件的原始论据有两条：Lua 解释器慢，高频纯算需要宿主内建件；wasm 无 AES-NI/SHA-NI 直通，加解密密集差 3–10×（[ADR-0024](0024-carrier-layering.md) §4 物理约束行；[ADR-0025](0025-seam-capabilities.md) §4 v1 清单）。

wasm-first 转向后，guest 是编译语言，基础计算性能可信。维护者拍板（2026-10-19）：**减少插件执行过程中与内核的通信；纯算件让插件自己执行。** 盘点后 crypto 是唯一剩下的纯算 host import——json / yaml / regex / diff / uuid / hex / base64 / timefmt 已随 [wasm-first-design-review](../research/wasm-first-design-review.md) B2 消解，sqlite 已随 [ADR-0039](0039-fs-capability-two-tier.md) 撤回。技术事实不变：wasm 仍无 AES-NI/SHA-NI，guest crypto 比原生慢约 3–10×。

## Decision

### 1. crypto 不进 ABI

v1 WIT 包不立 crypto interface；sha256 / sha512 / hmac / AES-GCM 由 guest crate 提供（RustCrypto 等）。插件作者按自身需求选 crate、自行钉版与升级，宿主不代持加密实现。

### 2. CSPRNG 走 WASI 标准件

`wasi:random` 是随机性的唯一来源（宿主提供熵；标准件复用，不是 metis 能力面）。SDK 可以 re-export 便利函数（feature 面，v1 不做）；v1 作者直接用 `getrandom` / `rand`。不设能力门。

### 3. 能力面判据固化

**只有 guest 不能自做的事进 metis import 面**：OS 访问（fs）、网络（http）、时钟唤醒（timer）、子进程（process）、跨实例编排（events / services / instance）。纯算件一律归 guest 包生态。v1 metis 功能 import = 7 个：`events` / `services` / `timer` / `http` / `fs`（global）/ `process` / `instance`；private fs = WASI `wasi:filesystem`（作者面 `ctx.plugin_fs()`，标准件，不入本表）；纯算 import 为零。

### 4. 成本接受与复审

- wasm 无 AES-NI/SHA-NI，guest crypto 比原生慢约 3–10×；高吞吐 crypto 走外部服务或 T2 进程插件。
- 宿主不再提供单一审计实现：crypto 的供应链、CVE 跟进与二进制体积归插件作者。
- 复审触发器 = 批量插件因 crypto 性能 / 审计问题成串；届时重新评估宿主内建或共享 guest crate 两种形态。

## Consequences

- 局部 supersede [ADR-0025](0025-seam-capabilities.md) §4 crypto 项、[ADR-0024](0024-carrier-layering.md) §4 crypto 行的结论面；两条 ADR 的 Status 行记本 ADR。技术事实（指令缺口）保留，[ADR-0034](0034-wasm-first-carrier.md) §3 的 WASI 标准件复用不变。
- 同步面：[seam-capabilities](../design/seam-capabilities.md) §1 / §4.2 / §6.2；[wit-abi](../design/wit-abi.md) W3（import 集 = D142 / D143）；台账 T10.11 站行（crypto 撤回）；[plugin-sdk](../design/plugin-sdk.md) S3（crate 依赖自由）；[wasm-first-design-review](../research/wasm-first-design-review.md) 补遗 1；[wasm-research](../research/wasm-research.md) 补遗 8。
- T10.11 不再实现 crypto；T10.14 可提供便利 re-export（feature 面，v1 不做）；T10.15 脚手架不加 crypto。
- 代价明记：单点审计红利消失；插件二进制体积与依赖树增大；性能取决于 guest crate 实现与编译优化。
