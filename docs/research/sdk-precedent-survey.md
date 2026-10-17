# SDK 先例调研：宏藏 ABI 四先例机制（2026-10-17）

> 性质：point-in-time 调研报告——T13-d（SDK 形态议题）开工输入执行：[ADR-0034](../decisions/0034-wasm-first-carrier.md) §4 点名的四先例（proxy-wasm SDK / Zed `zed_extension_api` / Spin / Extism PDK）逐一调研「宏藏 ABI」机制与作者面形态。执行 = 4 路并行子代理网络核查（2026-10-17 访问，出处 URL 随节在案）；消费去向 = [plugin-sdk](../design/plugin-sdk.md) S1–S7（D98–D104 的拍板地基）。勘误与更新走补遗，不改写正文结论。
> 标记惯例：「未确认」= 未核实项（附确认方法）；代码块逐字摘自官方源，不改动。
> 与既有研究的关系：产业案例层（版本矩阵地狱、生态活力、ABI 版本化纪律）= [wasm-research](wasm-research.md) §4.1/§4.3/§5.5 已在案；本报告只补宏机制层，不重复。

## 0. 结论速览

| 先例 | 宏形态一句话 | 对 metis-plugin 的落点 |
| --- | --- | --- |
| proxy-wasm Rust SDK 0.2.5 | `macro_rules!` 的 `main!` 注册块 + 三层库代码（hostcalls/dispatcher/trait 族），零 proc-macro | 分层而非堆宏；全默认 trait 钩子；手写 dispatcher 长尾 bug 与 unwrap→trap 为戒（→ S1/S5） |
| Zed `zed_extension_api` 0.7.0 | `register_extension!` + `Extension` trait；wit-bindgen 生成/导出全住 SDK crate 内，作者零接触 WIT | 绑定住 SDK crate、WIT SSOT 同树两视角、trait 默认方法演进术（→ S2/S3）；`static mut` 单例为戒（→ S1） |
| Spin `spin-sdk` 7.0.0 | 属性宏挂 async fn；宏 crate vendor WIT；wasip3 原生 async 后删除内置 executor | async 直挂起步（→ S4）；能力 API 实体性分流与运行期类型化错误（→ S6）；宏名漂移与接口版本倍增为戒 |
| Extism `extism-pdk` 1.4.1 | `#[plugin_fn]` 挂 pub fn，极简；私有 ABI 字节过内存，serde 直通 | 极简作者面为上限基准（→ S1）；零跨边界类型对账 + XTP Bindgen 外挂为反例（→ S7） |

共同判决：组件模型路线的 SDK 价值点 = 把 wit-bindgen 生成面包成符合人体工学的作者 trait/函数面 + 版本与钉版收口；四先例无一让作者接触 ABI 层，目标形态一致。

## 1. proxy-wasm Rust SDK

调研基线：`proxy-wasm` crate 0.2.5 / main 0.3.0-dev；出处 = docs.rs、GitHub main 分支源码、proxy-wasm/spec v0.2.1。

### 1.1 前置更正

不存在 `#[proxy_wasm::main]` 属性宏。`proxy_wasm::main!` 是 `macro_rules!` 函数式宏，接收一个 `$code:block`，不挂在任何 struct/impl/fn 语法节点上；整个 SDK 没有 proc-macro crate。（ADR-0034 §4 先例清单的措辞按此更正理解。）
出处：https://github.com/proxy-wasm/proxy-wasm-rust-sdk/blob/main/src/lib.rs 、https://docs.rs/proxy-wasm/latest/proxy_wasm/macro.main.html

### 1.2 作者面形态

最小插件 = 注册块 + 一个 struct + trait impl（逐字摘自 hello_world 示例）：

```rust
proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> { Box::new(HelloWorld) });
}}

struct HelloWorld;
impl Context for HelloWorld {}
impl RootContext for HelloWorld {
    fn on_vm_start(&mut self, _: usize) -> bool { /* ... */ true }
    fn on_tick(&mut self) { /* ... */ }
}
```

HTTP 插件再加一层：root 实现 `get_type() -> Some(ContextType::HttpContext)` 与 `create_http_context()`，子上下文实现 `HttpContext`。作者写的全是普通 Rust：无 `extern`、无 `no_mangle`、无 unsafe、无指针。
出处：https://github.com/proxy-wasm/proxy-wasm-rust-sdk/blob/main/examples/hello_world/src/lib.rs

### 1.3 宏藏 ABI 机制（宏 + 三层库代码，宏只占很小一部分）

- `main!` 只展开成入口：cdylib 模式生成 `#[no_mangle] pub extern "C" fn _initialize()`（内调 `__wasm_call_ctors` 后执行注册块）；reactor 模式生成 `fn main() -> Result<...>`。
- hostcalls 层：每个宿主函数一个 `#[link(wasm_import_module = "env")] unsafe extern "C"` 声明，外包安全 `pub fn ... -> Result<_, Status>`，内部做 `(ptr,len)` 传递、自定义小端序列化、`Vec::from_raw_parts` 回收宿主经 guest 分配器写入的内存。
- allocator 层：`proxy_on_memory_allocate(size) -> *mut u8`——宿主要往 guest 写数据时反向调它要内存，私有 ABI 的内存约定核心。
- dispatcher 层：约 25 个 `#[no_mangle] pub extern "C" fn proxy_on_*` 导出，全部转给 `thread_local!` 单例 `Dispatcher`。
- 版本标记：`#[no_mangle] pub extern "C" fn proxy_abi_version_0_2_1() {}`——空函数，函数名即握手，规范明确「This function is never called」。

作者完全不需要知道 ABI 存在；唯一可见的 ABI 残影是构造闭包签名里的 `context_id: u32`。
出处：https://github.com/proxy-wasm/proxy-wasm-rust-sdk/blob/main/src/hostcalls.rs 、…/src/allocator.rs 、…/src/dispatcher.rs ；规范 https://github.com/proxy-wasm/spec/blob/master/abi-versions/v0.2.1/README.md

### 1.4 trait 族设计

- 继承链：`Context`（基座）← `RootContext` / `StreamContext` / `HttpContext`。
- 风格：全同步、事件回调式，`on_*(&mut self, ...)`；无 async/await。「异步」靠 token 关联：`dispatch_http_call(...) -> Result<u32, Status>` 拿 token，宿主事后回调 `on_http_call_response(token_id, ...)`，dispatcher 用 `callouts: HashMap<token_id, context_id>` 路由回发起上下文。
- 默认实现全覆盖：所有 `on_*` 默认为空操作或安全默认值；host 服务方法默认实现直接转调 hostcalls——作者只覆写关心的钩子。
- 分发不靠宏：注册时存 `type NewRootContext = fn(u32) -> Box<dyn RootContext>` 函数指针；子上下文经全局构造器、root 工厂方法或 `get_type()` 判别三选一。

### 1.5 crate 拓扑与版本钉

- 单 crate `proxy-wasm`，依赖仅 `hashbrown`、`log`、可选 `mockalloc`；公开模块 `hostcalls`/`traits`/`types`，私有 `allocator`/`dispatcher`/`logger`。docs.rs 显示「0% of the crate is documented」——文档靠 examples 而非 rustdoc。
- 双版本轨：crate 版本（0.2.5）≠ ABI 版本；对账方式 = 导出符号名 `proxy_abi_version_0_2_1`。规范独立成库（proxy-wasm/spec），宿主与多语言 SDK 共同实现同一 ABI。

### 1.6 错误与状态面

- 行动返回：`#[repr(u32)] #[non_exhaustive] enum Action { Continue=0, Pause=1 }`；生命周期用 `bool`。
- host 调用错误：`Result<_, Status>`；但 SDK 内部大量 `.unwrap()` / panic——预期内失败才给 Result，其余直接 panic（wasm 下 panic=abort = 杀 VM，规范为此要求宿主实现崩溃重启与限流）。
- 跨调用状态：作者 struct 被 `Box<dyn XContext>` 收进 dispatcher 的 `RefCell<HashMap<u32, …>>`，键 = 宿主分配的 `context_id`；每次回调按 id 取回 `&mut` 调用；作者只管往 struct 字段塞状态，取回机制不可见。

### 1.7 对 metis-plugin 的教训

1. 值得抄：分层而非堆宏——薄 unsafe 包装层 → 持有 trait 对象的分发层 → 全默认实现的作者 trait 族；「全默认实现 + 只覆写所需钩子」让最小插件约 30 行。
2. 差别要认清：藏私有 C ABI ≠ 藏 canonical ABI。组件模型下 `(ptr,len)` 约定、反向分配、手搓序列化被 canonical ABI lift/lower、resource handle、WIT 包版本整体替代：胶水代码由 `wit_bindgen::generate!`/`export!` 生成而非手写，版本对账用 WIT world 的 `@version`——不要重新发明 `proxy_abi_version_x_y_z` 符号标记。SDK 的价值点上移为「把生成的 Guest trait/resource 类型包成符合人体工学的作者 trait」。
3. 要避的坑：panic 当错误通道 + 手写 dispatcher 长尾 bug（CHANGELOG 多处「invalid context_id/token_id」类修复）。metis 有 WIT `result<_, err>` 一等错误通道，host 失败应以 `Result` 贯穿作者面；分发逻辑尽量交给生成代码。

附注：示例工程的 `crate-type` 未逐一核对（未确认）；SDK 支持 cdylib 与 reactor bin 两种形态，见 src/lib.rs 的 cfg 分支。

## 2. Zed `zed_extension_api`

调研基线：zed 仓 main 分支源码 + docs.rs 0.7.0 + 官方文档（2026-10-17 访问）。四先例中与 metis 最同构：Zed 扩展同样是 wasm 组件 + WIT world + wit-bindgen 生成绑定，SDK 把 WIT 藏起来。

### 2.1 作者面形态

扩展 = Git 仓库 + `extension.toml` 清单 + 可选 Rust crate（`crate-type = ["cdylib"]`，target `wasm32-wasip2`）。最小 Rust 代码（逐字摘自官方文档与 README）：

```rust
use zed_extension_api as zed;

struct MyExtension {
    // ... state
}

impl zed::Extension for MyExtension {
    // ...
}

zed::register_extension!(MyExtension);
```

`register_extension!` 在 `lib.rs` 顶层、impl 之后调用一次，只收一个类型名（宏签名 `($extension_type:ty)`）。无状态的方法纯靠默认实现时 impl 体可为空。
出处：https://zed.dev/docs/extensions/developing-extensions ；https://github.com/zed-industries/zed/blob/main/crates/extension_api/README.md

### 2.2 宏藏 ABI 机制

wit-bindgen 完全藏在 SDK crate 内部，作者不接触 .wit 也不接触 wit-bindgen（它是 SDK 的私有依赖）：

- `extension_api.rs` 内私有 `mod wit { wit_bindgen::generate!({ skip: ["init-extension"], path: "./wit/since_v0.8.0" }) }`——只对最新 wit 快照生成。
- SDK 内部定义 `struct Component; impl wit::Guest for Component`，逐方法转发到全局单例 `static mut EXTENSION: Option<Box<dyn Extension>>`，并在此边界做 JSON String ↔ `serde_json::Value` 转换；`wit::export!(Component)` 也在 SDK 内。
- `register_extension!` 展开为：一个覆写 `chdir`（禁改 CWD）的 `wasi_ext` 模块 + `#[unsafe(export_name = "init-extension")] fn __init_extension()`，后者调用 `new()` 并存入全局单例。`generate!` 的 `skip: ["init-extension"]` 是刻意留白，让宏手写这个导出以夹带 wasi libc 修复。
- 宿主能力函数（`fetch`/`download_file` 等）是对生成符号的显式枚举 re-export（注释：「有些符号要 shadow 掉以提供更干净的 Rust API」）。

出处：https://github.com/zed-industries/zed/blob/main/crates/extension_api/src/extension_api.rs

### 2.3 trait 族设计

`pub trait Extension: Send + Sync`：全同步方法；唯一必需项 `fn new() -> Self`；其余约 19 个全是带默认实现的 provided methods，默认值分语义档位——「未实现即错」（`Err("`x` not implemented")`）vs「空即合理」（`Ok(None)` / `None` / `Ok(Vec::new())`）。演进术：WIT world 加 export → trait 加带默认体的方法 → 旧扩展照常编译运行；ABI 面用 JSON string 承载配置类数据，Rust 面用 `serde_json::Value`，ABI 因此可冻结而 Rust 类型自由演进。宿主侧对新功能 + 旧版本扩展返回显式错误「`run_slash_command` not available prior to v0.1.0」。
出处：https://docs.rs/zed_extension_api/latest/zed_extension_api/trait.Extension.html ；https://github.com/zed-industries/zed/blob/main/crates/extension_host/src/wasm_host/wit.rs

### 2.4 crate 拓扑与版本纪律

- WIT SSOT 在 SDK crate 内：`crates/extension_api/wit/since_vX.Y.Z/`，每个 API 版本一份完整快照（约 12 个 wit 文件）。guest 侧只 `generate!` 最新目录。
- 宿主消费同一目录树：`crates/extension_host/src/wasm_host/wit/since_vX_Y_Z.rs` 每版一个模块，`wasmtime::component::bindgen!({ path: "../extension_api/wit/since_v0.6.0", … })`，并手写 `MIN_VERSION`/`MAX_VERSION` 常量与到 `latest` 的 `From` 桥接。
- 版本对账不靠扩展声明、靠二进制内嵌：SDK 的 `build.rs` 把 crate 版本写成 6 字节，`#[link_section = "zed:api-version"]` 嵌进 wasm custom section；宿主编译前用 wasmparser 解析（缺失即拒绝加载），实例化时按版本级联选择对应快照的绑定，`wasm_api_version_range` 按 release channel 圈定支持区间。
- 破坏性改动攒批：`PENDING_CHANGES.md` 记录待爆发的 breaking changes；README 维护 Zed 版本 ↔ api 版本兼容表。

出处：https://github.com/zed-industries/zed/tree/main/crates/extension_api/wit ；…/extension_api/build.rs ；…/extension_host/src/wasm_host.rs

### 2.5 异步与能力面

作者面零 async：http（`fetch`/`fetch_stream`，后者返回带 `next-chunk` 方法的 resource）、`download_file`、`Command::output()`、`Worktree::read_text_file` 等全是阻塞式同步签名。WIT import 就是普通 `func`（非 async），异步全部在宿主侧：wasmtime `bindgen!` 配 `imports/exports: { default: async }`，引擎开 epoch interruption 每 100 ms 让出。映射手法 = 生成符号 re-export + 手写 facade（`HttpRequest::builder()`、`Command::new().arg().env()`）。能力管控：宿主有 `CapabilityGranter` 按 manifest 门控部分 import，细节「未确认」（未读 capability_granter.rs）。
出处：https://github.com/zed-industries/zed/blob/main/crates/extension_api/wit/since_v0.6.0/http-client.wit ；…/src/http_client.rs ；…/wasm_host.rs

### 2.6 对 metis-plugin 的教训

值得抄：

1. WIT SSOT 住 SDK crate + since_快照分层：guest 只对最新快照 `generate!`，宿主对全部快照 `bindgen!` 并按 wasm 内嵌版本号分发。一套文件两个视角，旧插件永久可加载；build.rs 嵌版本 + custom section 的方案零清单依赖、不可伪造。
2. `export!`/`Guest` 整体藏 SDK 内：作者只见 trait + 注册宏；ABI 边界用 JSON-string 承载演进型数据，Rust 面自由升级——WIT world 变更频率被结构性压低。
3. trait 默认方法分语义档位（Err 档 vs None 档）+ 宿主侧「not available prior to vX」显式报错，演进零破坏且失败可诊断；`PENDING_CHANGES.md` 攒批 breaking change。

要避的坑：

1. 同步阻塞作者面把复杂度全推给宿主：效应调用阻塞 wasm 执行线程，Zed 靠 epoch interruption 缓解且自认是折衷。metis 插件有长任务/重入需求（S2 spike 方向），在 world 层就考虑异步，而非事后补丁。
2. 每版本手写 From 桥接成本线性增长（单 `since_v0_6_0.rs` 就数百行机械 impl），快照越多越痛；应尽早冻结 world 形状，或评估桥接代码生成。「未确认」：Zed 是否完全手写，未见生成工具。
3. `static mut EXTENSION` 全局单例（`#[expect(static_mut_refs)]` 硬压 lint）绑定「一组件一实例」模型；与 metis 的实例模型 + 同实例重入（W4 章）不兼容，注册机制需换成实例化工厂式。

另注：`wit/since_v0.6.0/settings.rs`（wit 目录里混着一个 .rs，经 `OUT_DIR` include）用途「未确认」，疑似 settings schema 的代码生成辅助。

## 3. Spin `spin-sdk`

调研基线：`spin-sdk 7.0.0`（2026-08-25 发布，配套 Spin v4 / wasip3 时代）+ 对照 `5.1.1/5.2.0`（Spin v3 / wasip1-p2 时代）。

### 3.1 作者面形态

宏挂在自由函数（fn）上，不是 impl。逐字形态（README main 分支）：

```rust
use spin_sdk::http::{IntoResponse, Request};
use spin_sdk::http_service;

#[http_service]
async fn hello_world(_req: Request) -> impl IntoResponse {
    "Hello, Spin"
}
```

宏名历经改名：v3 文档时代为 `#[spin_sdk::http_component]`（async 可选、兼容 sync fn），7.0 起为 `#[http_service]`（async 强制，非 async 直接编译错误）。返回值走 `IntoResponse` 特质（`&str`、`String`、`StatusCode`、`http::Response<T>`、`(StatusCode, body)` 元组、`Result<impl IntoResponse>` 均可）。
出处：https://github.com/spinframework/spin-rust-sdk （README）；https://docs.rs/spin-sdk/latest/spin_sdk/attr.http_service.html

### 3.2 宏藏 ABI 机制

`spin-macro 7.0.0` 源码（lib.rs 逐字读过全文）揭示全部机制：

- WIT vendored 在宏 crate 内部：`const WIT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/wit")`——proc-macro 展开时的 `CARGO_MANIFEST_DIR` 指向宏 crate 自己，仓库内可见 `crates/spin-sdk-macro/wit/` 目录。作者从不接触 .wit。
- `#[http_service]` 展开 = 原 fn + `mod __spin_wasip3_http`：`struct Spin; ::spin_sdk::wasip3::http::service::export!(Spin);`，再 `impl wasip3::exports::http::handler::Guest for Spin`，在 trait 方法里做 `FromRequest` 转换、`.await` 用户函数、`IntoResponse` 回转。http 走 `wasip3` crate 自带的 wasi:http@0.3 绑定，不再本地 generate。
- `#[redis_subscriber]` 展开 = `::spin_sdk::wit_bindgen::generate!({ world: "spin-sdk-macro-redis-trigger", path: #WIT_PATH, generate_all })` + `export!(Spin)` + impl `exports::spin::redis::inbound_redis::Guest`。即 wit-bindgen 的 `generate!` 被推迟到用户 crate 编译期展开，WIT 路径由宏 crate 携带。
- 另有 `spin_sdk::dependencies!()`：若用户项目根存在 `spin-dependencies.wit` 则以 `world: "root"` 生成组件间依赖绑定。该 .wit 由谁生成（推断为 `spin build` 依 manifest 生成）——「未确认」。
- 多触发器一致性：内置触发器（http/redis）宏同构；cron 是外部触发器插件，用独立 crate `spin_cron_sdk` 的 `#[cron_component]`，形态照抄同一模式。

出处：https://docs.rs/spin-macro/latest/src/spin_macro/lib.rs.html ；https://docs.rs/spin-macro/5.1.1/src/spin_macro/lib.rs.html （旧版对照）

### 3.3 处理函数签名风格与 async 桥接

- 7.0 HTTP：`async fn(Request) -> impl IntoResponse`，入参任何 `FromRequest`、返回任何 `IntoResponse`；惯用 `anyhow::Result<impl IntoResponse>`。Redis：`async fn(Vec<u8>) -> anyhow::Result<()>`，宏内把 `Err` 映射成 WIT `Error::Other(e.to_string())` 并 eprintln。
- async 桥接两时代对比（最关键的历史教训）：
  - 5.x（wasip1/2）：组件模型 export 是阻塞式 `fn handle(request, response_out)`，宏内用 `::spin_sdk::http::run(async move { … })`（独立 crate `spin-executor`）把用户 future 跑在 SDK 内置 executor 上；用户 fn 的 async 是「可选装饰品」。
  - 7.0（wasip3）：Guest trait 方法本身就是 `async fn handle(request) -> Result<Response, ErrorCode>`——组件模型原生 async ABI，`wasip3::http::service::export!` 直接接受 async impl，SDK 内 executor 整个被删掉。另暴露 `spin_sdk::wasip3::spawn(Future<Output=()>)` 跑后台任务（可活过入口返回，用于流式响应），以及 `wit_future::new()`/`wit_stream::new()` 原语。
- 流式惯例：API 返回 `(StreamReader<T>, impl Future<Output = Result<(), Error>>)` 对——先消费流，后 await 结果 future。

出处：https://docs.rs/spin-macro/5.1.1/src/spin_macro/lib.rs.html ；https://spinframework.dev/v4/rust-components

### 3.4 crate 拓扑与版本对账

- 仓库 `spinframework/spin-rust-sdk`（独立于宿主仓），`crates/` 下 `spin-sdk` + `spin-sdk-macro`（发布名 `spin-macro`）。5.x 时代还拆出 `spin-executor`、`spin-wasip3-http(-macro)`，7.0 全部收敛、改用 bytecodealliance 的 `wasip3 ^0.7`（其 `wit_bindgen` 经 `spin_sdk::wit_bindgen` 再导出）。能力模块全部 feature-gated，默认全开。
- WIT world SSOT 在宿主仓：`spinframework/spin` 的 `wit/world.wit`，package `spin:up@4.1.0`，定义 `http-trigger`/`redis-trigger`/`platform` 三个 world；SDK 侧只 vendor 宏展开所需子集。
- 版本对账机制不是锁版本号，而是 WIT 接口版本化 + 宿主多版本并存：world.wit 同时 import `spin:postgres/postgres@3.0.0` 和 `@4.2.0`、`wasi:http` 0.2.6/0.3.0-rc/0.3.0 三版——旧 SDK 编的组件在新宿主照跑。代价落到 SDK 面：5.x 出现 `pg`/`pg3`/`pg4`、`sqlite`/`sqlite3` 并存模块（rustdoc 原文「for Spin 2 and earlier… should use `pg4` instead」）。
- 发布联动：SDK 发 tag → crates.io → 向 `spinframework/spin` 仓 dispatch `rust-sdk-release` 事件 → 自动更新模板/示例的 SDK 依赖。即模板 pin SDK 版本是新项目的实际对账点。

出处：https://raw.githubusercontent.com/spinframework/spin/main/wit/world.wit ；https://raw.githubusercontent.com/spinframework/spin-rust-sdk/main/release-process.md

### 3.5 能力面映射与越权炸点

两种惯形态，按资源语义分流：

| 能力 | 形态 | 代表 API |
| --- | --- | --- |
| kv / sqlite / redis（有连接/存储实体） | 句柄类型 | `key_value::Store::open_default().await?` → `store.set(k,v).await?`；`sqlite::Connection` |
| llm / variables / outbound http（无状态调用） | 自由函数 | `llm::infer(model, prompt)`；`variables::get("name")`；`http::send(req).await?` |

模块是 WIT 生成绑定外的惯用包装层（re-export `http` crate 类型、`set_json`/`get_json` serde 糖），原始 WIT 绑定保留可达但标注「You should not normally need to use this module」。

授权分工：声明全在 manifest（spin.toml），SDK 面不做任何编译期检查。`allowed_outbound_hosts`、`key_value_stores = ["default"]` 等都是组件级 manifest 字段。越权在运行期以类型化 WIT 错误返回：未声明域名的出站请求 → `send()` 返回「Destination not allowed」错误；`Store::open` 未声明的 label → `error::no-such-store`。不 trap、不编译失败，就是普通 `Err`，作者用 `?` 自然传播。
出处：https://docs.rs/spin-sdk/latest/spin_sdk/key_value/struct.Store.html ；https://spinframework.dev/v4/rust-components

### 3.6 对 metis-plugin 的教训

值得抄：

1. 「宏挂 async fn + WIT vendor 在宏 crate 内」的整套机制可复刻：`CARGO_MANIFEST_DIR/wit` 定位技巧、`generate!` 推迟到用户 crate 展开、`FromRequest`/`IntoResponse` 式转换特质让签名宽容化。
2. 能力 API 按实体性分流：有生命周期实体用 `open()` 得句柄类型，无状态服务用自由函数；原始绑定保留 escape hatch 但明确标注不推荐。manifest 声明授权 + 运行时类型化错误返回（而非编译期或 trap），SDK 面保持统一 `Result` 风格。
3. 直接以组件模型原生 async 起步：Spin 的历史债是 wasip1/2 时代不得不在 SDK 内置 executor 桥接，wasip3 后整套删掉。

要避的坑：

1. 接口版本演进别在 SDK 面倍增：`pg/pg3/pg4`、`sqlite/sqlite3` 并存是作者面可见的疮疤。版本兼容应收进 WIT package 版本化 + 宿主多版本并存层。
2. 宏名随 ABI 换代漂移的断裂成本：`http_component`→`http_service`、`redis_component`→`redis_subscriber` 的全量改名让所有旧文档/教程/用户代码作废。触发器宏名应尽早冻结，把 ABI 演进藏在宏内部。
3. 文档与 crate 版本不同步：v4 文档手写 `spin-sdk = "6.0"` 时 crates.io 已是 7.0.0——版本号在文档里出现时即腐化点，文档引用 SDK 版本应交给工具链注入而非手写。

注：`spin-dependencies.wit` 的确切生成方、spin-sdk 7.0 与 Spin 宿主次版本的精确对应表为「未确认」。

## 4. Extism `extism-pdk`

调研基线：`extism-pdk` 1.4.1 + derive/converter 源码（2026-10-17 访问）。定位重申：ADR-0034 已判决 Extism 路线 = 设计参照非技术选项（私有 ABI / 字节过内存 / 单厂商治理），本章只吸取其作者面设计的优点与教训，不重开选型。

### 4.1 作者面形态

最小插件（逐字摘自 README）：

```rust
use extism_pdk::*;

#[plugin_fn]
pub fn greet(name: String) -> FnResult<String> {
    Ok(format!("Hello, {}!", name))
}
```

- 宏挂在普通 `pub fn` 上（attribute macro），签名风格 = 普通 Rust 函数：输入最多一个参数（实现 `FromBytes`），输出必须是字面返回 `FnResult<T>`（T 实现 `ToBytes`）。
- 宏有硬性约束（违反即编译期 panic）：必须 `pub`、不得命名 `main`、返回类型标识符必须是 `FnResult`；`()` 参数或无参均可。
- crate-type 必须为 `cdylib`，目标 `wasm32-unknown-unknown`（无系统访问）或 `wasm32-wasip1`。

出处：https://github.com/extism/rust-pdk （README）；https://docs.rs/extism-pdk/latest/extism_pdk/attr.plugin_fn.html

### 4.2 宏藏 ABI 机制

逐字核对 `derive/src/lib.rs` 源码：

- `#[plugin_fn]` 把作者函数改名为 `inner`，外层生成 `#[no_mangle] pub extern "C" fn name() -> i32`。导出函数无参数、返回 i32 退出码：输入经 `extism_pdk::input()`（内核 import `input_length` + 循环 `input_load_u64/u8` 逐 8 字节拷贝）读入再 `FromBytes` 解码；输出经 `output()`（`ToBytes` → 分配内存 → `output_set(offset, len)`）；错误时把错误文本写入内存、`error_set(offset)`，并返回退出码；成功返回 0。
- 私有 ABI 全貌：一组固定 `#[link(wasm_import_module = "extism:host/env")]` import——`alloc/free/length/store_u64/load_u64/output_set/error_set/config_get/var_get/var_set/http_request/log_*`。内存以 u64 offset 指涉。作者全程不见这些符号。
- `host_fn!` 挂在 `extern "ExtismHost"` 块上：

```rust
#[host_fn]
extern "ExtismHost" {
    fn a_python_func(input: String) -> String;
}
```

  展开为 `#[link(wasm_import_module = "extism:host/user")] extern "C" fn name_impl(u64...) -> u64` + 同名包装函数：每个参数 `ToMemory::to_memory` 转 u64 offset，返回值 `FromBytes` 解码并包成 `Result<T, extism_pdk::Error>`。命名空间可用 `#[host_fn("extism:host/user")]` 参数化。

出处：https://raw.githubusercontent.com/extism/rust-pdk/main/derive/src/lib.rs ；https://raw.githubusercontent.com/extism/rust-pdk/main/src/extism.rs

### 4.3 类型与编码面

- 编码层独立 crate `extism-convert`，两侧共用同一编码词汇表。
- 结构化数据靠 serde 直通 + axum-extractor 风格 newtype：`Json<T>` / `Msgpack<T>` / `Prost<T>` / `Base64<T>`，模式匹配解构（`Json(add): Json<Add>`）；或 `#[derive(FromBytes/ToBytes)] #[encoding(Json)]` 把编码做成类型级注解。
- 编码错误在运行期、guest 边界炸：`input::<T>()` 内 serde decode 失败 → 错误字符串 `error_set` 后返回 -1，宿主只收到通用插件错误 + 文本消息。两侧编码/类型选错无任何编译期信号。
- 官方自认此缺口：带外 IDL「XTP Bindgen」（OpenAPI 风格 YAML schema → 生成各 PDK 绑定）是后补方案，非默认路径。

出处：https://docs.rs/extism-convert/latest/extism_convert/ ；https://raw.githubusercontent.com/extism/rust-pdk/main/src/macros.rs ；https://github.com/dylibso/xtp-bindgen

### 4.4 crate 拓扑与版本纪律

- 拓扑：`extism-pdk` 1.4.1 → `extism-pdk-derive`（同号锁步）、`extism-convert`（两侧共享）、`extism-manifest`（optional，`http` feature 引入 `HttpRequest` 类型）；feature：default = http + msgpack，protobuf 可选。
- 版本对账：各 crate 独立演进，靠 major=1 的 semver 范围对齐；未见任何 ABI 版本握手——契约就是 `extism:host/env` 符号集，PDK 与 runtime 错位会在实例化/调用期炸，编译期无对账。runtime 是否主动拒绝旧 ABI 插件：「未确认」（未发现相关机制文档）。
- manifest 分工：纯宿主面部署/策略描述（wasm 来源 + sha256、memory max_pages、allowed_hosts/allowed_paths、config k/v），JSON schema 导出；guest 不见 manifest，只经 `config_get` 读 config。

出处：https://raw.githubusercontent.com/extism/rust-pdk/main/Cargo.toml ；https://extism.org/docs/concepts/manifest

### 4.5 错误与异步面

- 错误：`FnResult<T> = Result<T, WithReturnCode<Error>>`；`WithReturnCode<T>(pub T, pub i32)`（错误 + 导出退出码），`Error` 是动态错误包装（anyhow 系），`From<E: Into<Error>>` 使 `?` 直通。
- 异步：无异步形态。宏只接受同步 fn（生成同步 `extern "C" fn`，阻塞式逐字节拷贝）；宿主面 `plugin.call` 同步阻塞；近期是否有实验性 async host function：「未确认」。

出处：https://docs.rs/extism-pdk/latest/extism_pdk/type.FnResult.html

### 4.6 对 metis-plugin 的教训

值得抄：

1. 极简作者面成立：「一个属性宏 + 普通 Rust 签名」覆盖绝大多数插件（README 全部示例无一样板代码）。metis 的生成面应以此为上限基准——若作者需要手写 trait impl/生命周期注解才能导出函数，就是倒退。
2. 编码做成可换 newtype 注解（`Json<T>` extractor / `#[encoding(Json)]` derive）而非签名语法：作者改编码只动一处类型注解。serde 直通的 ergonomics 是真实优势。
3. `host_fn` 声明面：extern 块 + 命名空间参数化，作者不碰 `#[link]`/指针；返回自动包 `Result`——「声明即 import」形态可对照 WIT import 生成代码设计。

WIT 路线必须避免的（Extism 的结构性缺点）：

1. 零跨边界类型对账：两侧各自声明类型与编码，错位 = 运行期 guest 边界 `error_set` + -1，宿主只得字符串。Extism 自己用带外 IDL（XTP Bindgen）补丁式追认——WIT 的「同一 .wit 生成两侧、编译期对账」正是根因解药，IDL 不应做成可选外挂。
2. 无 ABI 版本握手：契约隐于 `extism:host/env` 符号存在性，版本对齐靠 semver 约定，失败推迟到实例化期。WIT/component-model 的 world 名与接口版本是显式契约，版本错配应挡在编译/加载前。
3. ABI 形状泄漏到宏能力：`plugin_fn` 只支持 0/1 参数（单输入缓冲区），多参数要降级到指针语义——「宏藏 ABI」的代价是 ABI 限制以魔法约束形式反弹给作者；WIT 的多参数/record 在 IDL 层一等表达，无此泄漏。

## 5. 横向对照与判词

| 维度 | proxy-wasm | Zed | Spin | Extism |
| --- | --- | --- | --- | --- |
| 宏挂点 | `macro_rules!` 注册块 | `macro_rules!` 注册宏 | 属性宏挂 async fn | 属性宏挂 pub fn |
| 状态承载 | struct → `Box<dyn Context>` 注册表 | struct → `static mut` 全局单例 | 无状态 fn（实体走句柄类型） | 无状态 fn |
| ABI 类型 | 私有 C ABI（ptr/len） | WIT/canonical ABI | WIT/canonical ABI | 私有 ABI（字节过内存） |
| 绑定生成位置 | 手写 hostcalls | SDK crate 内一次生成 | 宏 crate vendor WIT，用户 crate 展开 | 宏生成 extern 包装 |
| 异步 | 无（token 回调） | 无（宿主 epoch 缓解阻塞） | 原生 async（7.0 起） | 无 |
| 版本对账 | 导出符号名握手 | wasm custom section 内嵌版本 | WIT 接口多版本并存 | 无握手（semver 约定） |
| 越权炸点 | —（宿主 ABI 层） | 运行期 CapabilityGranter | 运行期类型化错误 | 无能力门概念 |

判词：① 藏私有 ABI（proxy-wasm/Extism）与藏 canonical ABI（Zed/Spin）共享「作者只见 Rust 表面」的目标形态，但组件模型路线下胶水代码应由 wit-bindgen 生成而非手写，SDK 价值点上移为「生成面的人体工学包装 + 版本钉版收口」。② 实例状态承载上，struct + trait 对象注册表（proxy-wasm/Zed 谱系）与 metis 实例模型对齐；Zed 的 `static mut` 单例与 Spin/Extism 的自由 fn 形态均不兼容 W4 同实例重入。③ 异步面只有 Spin 7.0 与 metis 同代（组件模型原生 async），其 executor 删除史直接支撑「跳过 executor 时代」。④ 版本对账四先例四解，metis 取 WIT package version 承接（ADR-0034 §3，W10 章细化），Zed 的 custom section 内嵌版本为最近参照。
