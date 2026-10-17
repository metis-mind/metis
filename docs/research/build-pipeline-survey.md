# 构建管线先例调研：dev loop / 构建缓存 / 远端构建场（2026-10-18）

> 性质：point-in-time 调研报告——T13-e（构建管线方案）开工输入执行：[ADR-0034](../decisions/0034-wasm-first-carrier.md) §6 四件（`metis dev` watch / 构建缓存 / 部署内工具链服务 / 远端构建场）的机制先例。执行 = 3 路并行子代理网络核查（2026-10-18 访问，出处 URL 随节在案）；消费去向 = [build-pipeline](../design/build-pipeline.md) D105–D115。勘误与更新走补遗，不改写正文结论。
> 标记惯例：「未确认」= 未核实项（附确认方法）；引用的官方数字与机制描述以来源页为准。
> 与既有研究的关系：wasm 组件模型 / 版本矩阵 / 生态层 = [wasm-research](wasm-research.md) §4/§5；SDK 宏藏 ABI 机制 = [sdk-precedent-survey](sdk-precedent-survey.md)；本报告只补构建与开发循环层，不重复。

## 0. 结论速览

| 主题 | 关键结论 | 对 metis 的落点 |
| --- | --- | --- |
| dev loop | 先例分裂为「整进程重启」（Spin）与「进程内 reload」（Zed）；共同底线 = 构建成功才发布、失败保留 last-known-good | `metis dev` 单进程 + 既有热换路径（D107/D109） |
| 构建适配 | manifest 任意 `build.command` 最灵活，但破坏缓存 key、结构校验与远端可复现 | v1 内置 Rust 适配器，无任意命令（D108） |
| 监听 | 100–500 ms debounce、排除 `target/` / `.git/`、构建中合并事件是共同做法 | 200 ms + latest-wins + hash 门（D110） |
| 缓存 | Cargo 官方未建议跨项目共享 target dir；跨 workspace 的正式建议是 sccache；sccache 缓存不了最终 cdylib/component link | 每工程 target + 内容寻址产物缓存（D111） |
| 可复现 | 路径、debug info、时间戳、custom section 是主要噪声源；Rust 对 wasm target 未给全量保证 | 本地不承诺字节级；farm 预留 remap/locked/offline（D112/D114） |
| 远端构建场 | 最小可信形态 = 钉版镜像 + 源码坐标 + lockfile + 产物 digest + 重建对账 + 未复现分级 | v1 只留 schema 与触发器（D114） |
| 部署内构建 | WASI 不能 spawn 进程/线程，rustc/cargo 必须在沙箱外跑 | 独立沙箱/容器 + 无网络 + 配额 + 审批（D113） |

## 1. dev loop 先例

| 先例 | 进程拓扑 | 监听 / debounce | 构建适配 | 重载 / 失败 | 缓存 |
| --- | --- | --- | --- | --- | --- |
| **Spin** | `spin watch` 单进程编排；`spin build` / `spin up` 为子进程；产物变化后重启整个 `spin up` | watchexec 监听 manifest 目录，按 `build.watch` / `files` / `source` 过滤；约 100 ms throttle；忽略 `*.swp`；元数据变更不算 | manifest 的 `[component.<id>.build]` 提供 `command` + `workdir`，任意命令；模板只给 cargo 示例 | 首次构建失败则不启动；已有成功版本时构建失败不重启，旧进程继续服务；成功才重启整个 host；无自动回滚 | 不管理 cargo target；仅对远程 wasm / registry 做 sha256 内容寻址缓存 |
| **wasmCloud `wash dev`** | v2 单进程内嵌 wash-runtime Host + HTTP ingress；v1 另起 host / NATS / wadm | v2 文档称 watch；当前 `dev.rs` 未见 watcher（**未确认**，文档/源码冲突）；v1 文档明确 watch + reload | `.wash/config.yaml` 的 `dev.command` 覆盖 `build.command`，以 `sh -c` / `cmd /c` 执行；注入 `WASH_COMPONENT_PATH` | `reload_component` 先 `workload_stop` 再 `workload_start`；新 workload 失败时旧实例已停，无 last-known-good；build 失败退出会话 | 2.9.0 起按组件字节 digest 缓存编译结果 |
| **Zed dev extension** | Zed 进程内 wasm host；dev extension 以 symlink 装入 extensions dir，指向源码目录 | 监听 installed extensions dir（约 100 ms）+ 200 ms reload debounce；只 reload，不自动 `cargo build`；`rebuild_dev_extension` 是显式动作（自动重编**未确认**） | 固定 Rust：`cargo build --target wasm32-wasip2 [--release] --target-dir <ext>/target`；`RUSTC_WRAPPER=""`；无任意 build 命令 | cargo 成功后才写 `extension.wasm`，随后卸载/重载扩展；失败保留旧 wasm、不重载；不重启 Zed | target dir 在扩展源码内；dev 默认 debug；sccache 被禁用（WASI build 会卡） |
| **cargo-watch + cargo-component / wasm-pack** | 用户侧 watcher + 一次性 build，无宿主 | cargo-watch 默认监听整个项目，排除 `target/`、`.git/`，遵守 `.gitignore` / `.ignore`；默认 `--delay 0.5s`；命令运行中默认重启 | `cargo watch -x 'component build'` 等；wasm-pack 无 watch flag | 无部署/重载语义；失败只是命令退出；旧产物是否保留取决于工具 | cargo incremental target；wasm-pack 缓存 wasm-bindgen / wasm-opt 二进制 |
| **jco / componentize-js / Extism** | 一次性 CLI，无内置宿主融合 | 未发现内置 watch / dev 模式（**未确认**：`--help` 可确认） | `jco componentize src.js --wit wit -o component.wasm`；`jco scaffold` 生成 npm scripts；Extism `xtp plugin build` | 无重载；由外部命令重启宿主 | 无项目级构建缓存 |
| **Trunk**（对照） | `trunk serve` 单进程内建 HTTP server + WS | 默认监听 `build.target` 父目录；`[watch]` 可配；25 ms debounce + 构建后 1 s cooldown | Cargo 配置驱动，不支持任意命令（有 hooks） | 先写 `.stage`，成功才替换 `dist`；失败保留旧 dist，错误推到浏览器 overlay，server 不重启 | cargo target + dist |
| **cargo-leptos**（对照） | `watch` 另起 server 子进程 + reload server | 递归监听 working dir（排除 `.git/`），遵守 `.gitignore`，100 ms debounce | Cargo 配置驱动，不支持任意命令 | 失败保留旧 server；成功且 server 产物变化才重启子进程 | cargo target，front 默认 `CARGO_TARGET_DIR/front` |

**可迁移设计教训**：

1. **双 lane 分离**：源码 lane（watch → build）与产物 lane（watch → reload）分开；不要用同一个条件串起「源码变化」和「产物变化」。Spin 的三个 watchexec 是现成范式。
2. **先 staging 再原子 publish**：Trunk 的 `.stage`、Zed 的「cargo 成功后才写 `extension.wasm`」是共同做法；旧产物在新版本就绪前不动。
3. **失败保留 last-known-good**：首次失败可拒绝启动；后续失败不阻塞 watcher；错误结构化上报。Spin 与 Trunk 都这么做。
4. **不要先停后启**：wash 的 `reload_component` 是反例（旧实例已停、新实例失败无回滚）；正确方向是 prepare → swap → drain，但这属于热换语义站，不属于构建管线。
5. **debounce/coalescing 按构建成本取**：save-all 合并 50–200 ms；构建后 cooldown 抑制自触发（Trunk 1 s）；构建中到达的事件标 dirty，结束再跑一轮（或 kill in-flight 重跑）。
6. **构建契约不要放进任意 shell 命令**：Spin / wash 的灵活性换来缓存 key、结构校验、远端可复现的缺口；Zed 的固定 cargo 更适合有信任链的产品。
7. **监听面要排除构建产物与部署数据**：`target/`、`.git/`、`data/`、`journal/` 不能进入 watch；否则自触发构建风暴。
8. **脚手架必须生成可跑的 build/watch 配置**：零配置路径优先；多语言用模板 / 内置适配器，而不是宿主分支。

**反模式**：

1. 监听部署根（含 `data/`、目标目录、日志）且无 ignore / cooldown → 自触发构建风暴。
2. 直接覆盖 live 产物再校验；失败即丢旧版本。
3. 先停旧实例再启新实例，失败无回滚（wash 反例）。
4. 把 reload 当 rebuild：只监听安装目录、不监听源码；或只 watch 不编译（Zed 需显式 Rebuild）。
5. 每次保存都重启整个 host：Metis 有 session / timer / 状态，不能照搬 Spin 的整进程重启。
6. 宿主硬编码 `cargo build` 且无 escape hatch，或只暴露任意 shell 命令，导致无法做结构化校验与稳定缓存 key。

## 2. 构建缓存与可复现

### 2.1 Cargo 共享 target dir

Cargo 的 fingerprint 键包含 rustc 版本哈希、features、target（名称 / 包内相对路径 / edition）、profile + compile mode + 额外 flags、源路径哈希（相对 workspace root）、依赖 fingerprint、源文件状态与 build script 环境、`RUSTFLAGS`、config 哈希等。产物命名与符号 mangling 另用 metadata 哈希，含 package id（name / version / source）、features、profile、mode、LTO、compile kind、target name / kind、rustc 版本。

风险：build script 读取的绝对路径、环境变量、外部文件不会全部进 fingerprint；跨项目共享可能错误复用。Cargo 对每个 unit 加共享锁、写时升级独占锁；并发共享 target dir 会互相阻塞。旧 fingerprint、旧工具链产物不会自动回收，磁盘随项目数增长。

Cargo 官方未建议跨项目共享 `CARGO_TARGET_DIR`；跨 workspace 共享的正式建议是 sccache。`CARGO_TARGET_DIR` 只定义位置。

**落点**：每工程独立 target dir（D111），不立全局共享。

### 2.2 sccache

sccache 用 `RUSTC_WRAPPER` 包装 rustc，缓存编译单元，不缓存链接。Rust 侧限制：incremental 必须关闭；调用系统链接器的 `bin` / `dylib` / `cdylib` / `proc-macro` 不能缓存。wasm 插件最终 crate 通常是 `cdylib`，由 `wasm-component-ld` 调 `wasm-ld` + componentization 生成 component，因此 sccache 只能命中依赖 rlib，不能命中最终插件产物。

**落点**：sccache = 可选扩展位（D111）；不作为主缓存。

### 2.3 `wasm32-wasip2` 增量

`wasm32-wasip2` 已是 Tier 2，直接输出 component，默认 `panic=abort`。Cargo incremental 只对 workspace member 与 path dependency 生效，release 默认关闭；rustc 明确 incremental 会抑制部分优化、增加 codegen units，不推荐 release。`wasm-component-ld` 每次链接都重跑 `wasm-ld` 与 componentization；incremental 只加速 rlib 重编译，不缓存最终 component 链接。S1 的 release 增量 0.22–0.29 s 只说明小插件链接成本低，不能推出增量可复现。

**落点**：dev 与 dist 共用优化 profile（D112）；不靠 incremental 做可复现。

### 2.4 component 可复现

常见不可复现因素：绝对路径进入 panic 的 `file!()`、DWARF / debug info、`env!` 与 build script 生成代码、debug info 的 producer / 路径、build script 或代码读取当前时间、`HashMap` 迭代顺序写进 metadata / custom section、链接器 `--build-id`、`name` / `producers` / `target_features` / debug 等 custom section 的顺序与版本字符串。

Rust 官方是逐 bug 修复 + run-make 测试，非全平台保证；CI 跟踪 issue 仍开放（已合并的 Linux bin + debuginfo 测试见 #151517，全平台跟踪见 #75362）。手段：`--remap-path-prefix` 是 best-effort，覆盖 diagnostics、debug info、macro expansion 等；`--remap-path-scope=all` 尽量连 rustc metadata 一起重映射，linker 生成的路径不受影响。`SOURCE_DATE_EPOCH` 是标准变量，但 cargo / rustc 不会自动消费，必须由 build script / 代码显式读取；建议设为 `git log -1 --pretty=%ct` 或固定值，且不要写进产物。`[profile.release] debug = false`、`strip = "debuginfo"` 可去掉大部分路径与符号；固定 `codegen-units`、关闭 incremental、固定 `RUSTFLAGS` 与 linker 版本，字节级比对才稳定。

**落点**：本地不承诺字节级可复现；远端构建场预留 remap / locked / offline / 钉版镜像（D112/D114）。

### 2.5 `metis.build` custom section

WebAssembly custom section 的 id 为 0，内容是 name + 任意字节，语义上被忽略；即使解析失败也不得使模块无效。Component Model 的 section 0 同样是 `core:custom`，定义在 component 层。

`wasm-tools metadata add` 只处理 name / producers / OCI 字段，没有任意自定义 section 的现成命令；用 `wasm-encoder::CustomSection`，它实现了 `Section` 与 `ComponentSection`，可直接 `append_to_component`。内容用 canonical JSON（键排序、无 timestamp）：`source_sha256`、`sdk_version`、`wit_package_version` / `abi_rev`、`target`、`profile`、`toolchain_version`。

**注意**：core module → component 转换是否保留 core custom section **未确认**；确认方法：给 core wasm 加 `(@custom "metis.build" ...)`，跑 `wasm-tools component new` 后用 `wasm-tools objdump` 或 `wasmparser` 检查。**落点**：追加在 component 层（D112）。

### 2.6 校验、import 提取与 `.cwasm`

- `wasm-tools validate` 默认启用 component-model 校验；`wasm-tools component wit component.wasm` 提取 WIT，`--json` 可机读；`wasm-tools print` 输出 WAT；`wasm-tools metadata show --json` 只覆盖标准元数据。
- `wasmparser`：`Parser::parse_all` 匹配 `Payload::ComponentImportSection`（component 级）或 `Payload::ImportSection`（core module 级）；嵌套 module 用 `Payload::ModuleSection` 的 sub-parser；`Payload::CustomSection` 读 `metis.build`。先 `Validator` 再解析。
- `.cwasm` 不能复用为构建缓存：`ModuleCacheEntry` 的 key 是 `HashedEngineCompileEnv` + wasm bytes + DWARF package + unsafe intrinsics，缓存目录还带 compiler version；它绑定 wasmtime 版本、Engine 配置与 host，只能作运行时预编译缓存。

**落点**：D108 的校验与 import 提取用 `wasmparser` / `wasm-tools`；`.cwasm` 归运行时缓存。

### 2.7 内容寻址 key

建议 key = SHA-256 over canonical encoding of：

- `source_tree_hash`：**全量源码工程减排除集**（相对路径排序 + 内容哈希；排除 `target/`、`.git/`、`.metis/`、`dist/`、编辑器临时文件）；这样 build script / proc-macro 读取的工程内文件也会进 key。
- 解析后的依赖图：registry 版本取 lockfile；path 依赖递归 hash。
- 工具链指纹：`rustc -vV` 的 commit hash + host + LLVM version；`cargo -V`；`wasm-component-ld`、`wasm-tools`、`wit-bindgen`、SDK version。
- 解析后的 features 集合、profile 规范化值（opt-level、lto、codegen-units、debug、strip、panic、incremental）、target、`RUSTFLAGS`、影响构建的 env。
- WIT 包版本 / `abi_rev` 与 WIT 文件哈希。

陷阱：mtime 不可靠；`Cargo.lock` 必须入 key，构建加 `--locked`；`.gitignore` 可能过滤掉 build script 实际读取的文件；绝对路径、路径分隔符、大小写、Unicode 归一化、空目录、CRLF 都会造成 key 漂移；不要用 `HashMap` 迭代顺序生成 canonical JSON。命中后校验 artifact sha256，原子写（temp + rename），按 key 加锁；key 带 schema 版本前缀。

### 2.8 缓存清理

Cargo 没有 target 目录 GC；`[cache]` 自动清理只覆盖全局 `CARGO_HOME` 的 registry / git 源，构建产物跟踪尚未实现。`cargo-sweep` 支持按时间、已安装 toolchain、`--maxsize` 清理，但 README 已标注 unmaintained（https://github.com/holmgr/cargo-sweep）；`cargo-clean-all` 可递归按天数 / 大小清理（https://github.com/dnlmlr/cargo-clean-all）；`cargo-cache` 管理 `CARGO_HOME`（https://github.com/matthiaskrgr/cargo-cache）。sccache 本地默认 10 GiB，内部用 `LruDiskCache` 按大小淘汰；Wasmtime cache 有 file count / 总大小 soft limit 与 LRU worker。

**落点**：metis 自建 GC（D111）：每 entry 写 `last_used` 标记，按总大小 + 年龄 + 最近使用淘汰；`metis cache clean --max-size / --older-than`。

## 3. 远端构建场与信任链

### 3.1 先例

| 先例 | 可复现 / 对账定义 | key 与 provenance | 失败标注 |
| --- | --- | --- | --- |
| docs.rs | 只自动构建 crates.io 库文档；sandbox 内网络阻断、源码/依赖只读、资源配额 | key = crate 名 / 版本 / target；provenance 仅工具链版本与日志，无源码 commit / 镜像摘要 | 构建日志 + 重建入口；资源/依赖失败分类说明 |
| Homebrew bottles | bottle 为编译产物，不做 bit-for-bit 重建；BrewTestBot 在 PR 上构建，成功且维护者批准后才写 DSL 并上传 | 文件名含 formula / version / OS / arch / rebuild；DSL 记 sha256、cellar；不记源码 commit 或构建环境 | 构建失败 / 校验和不匹配则不用 bottle，回退源码构建 |
| Debian reproducible builds | 同源码、同环境、同指令，任何方 byte-for-byte 重建；CI 先构建再重建 | `.buildinfo` 记 source / binary / arch / version、Sha256、精确依赖版本、Environment、Build-Path、Build-Tainted-By，可签名 | FTBR / FTBFS；rebuilderd 状态 GOOD / BAD / UNKWN，差异用 diffoscope |
| Nix | 钉 nixpkgs commit；derivation / 输入闭包寻址；cache 带签名 | key = store path / derivation hash；provenance = 输入闭包与签名；无独立「谁构建」attestation | 内容 hash 或签名校验失败即拒绝 substitution，回退重建 |
| Bazel RBE | Action digest 覆盖 command、input root、timeout、salt、platform；CAS 内容寻址；可复现是缓存正确性的前提 | key = Action digest；provenance = Action / Command / 输入 root 的 CAS digest | `ExecuteResponse.status`；缓存写入需限权，防投毒 |

### 3.2 SLSA / Sigstore / in-toto

- SLSA provenance 分级：L1 只要求 provenance 存在；L2 要求 hosted platform 签名 provenance；L3 要求租户隔离 + 签名密钥对构建步骤不可见。小型 Rust-only marketplace 的合理目标是 L2，而非 L3。
- in-toto 四层：predicate / Statement / envelope / bundle；Statement 绑定 `subject` 与 `predicateType`，适合做通用 attestation 容器。
- cosign keyless：OIDC identity → Fulcio 短命证书 → Rekor 透明日志；验证必须给 `--certificate-identity` 与 `--certificate-oidc-issuer`；bundle 支持离线验证。

**落点**：D114 预留 in-toto Statement + SLSA predicate 子集；v1 不实现 Rekor / L3；发布路径以产物上传 + 签名 + 结构校验 + 可选重建为主。

### 3.3 服务端构建的 registry 实例

- crates.io：上传 `.crate`（metadata + tarball），registry 不构建；docs.rs 另行构建文档。
- npm：上传 tarball + integrity；registry 不构建；provenance 由 CI 生成并 Sigstore 签名。
- Deno Deploy / Vercel：上传源码（Git / CLI），托管 builder 执行 build；自动 build cache；有 CPU / RAM / 时间配额。
- Cloudflare Workers：Git 推送触发构建；也支持 `wrangler deploy` 从 CLI 部署。
- Fermyon Cloud：官方 quickstart 为本地 `spin build`，`spin deploy` 上传产物；是否有服务端构建路径 **未确认**。
- wasmCloud：`wash build` 本地构建并发布 OCI artifact，运行时不构建；是否有托管构建场 **未确认**。

**落点**：服务端源码构建只覆盖 Deno / Vercel / Cloudflare 的 Git 路径；Fermyon、wasmCloud、npm、crates.io 以产物上传为主。Metis 的 v1 发布路径按产物上传设计（D114）；服务端构建延后。

### 3.4 wasm 组件的结构校验面

- 组件自描述：imports / exports 带类型，WIT world 描述接口；memory 不导入/导出；组合可类型检查。
- `wasm-tools` 提供 `validate --features=...`、`component wit --json`、`metadata show`、`strip`、`objdump`。
- `wasmparser::WasmFeatures`：`THREADS` 默认 true（shared memory / atomics 相关）、`SHARED_EVERYTHING_THREADS` 默认 false、`CM_THREADING` 默认 false；`MEMORY64` / `MULTI_MEMORY` / `GC` 等也可显式控制；`Validator::new_with_features` 接受自定义集合。
- 可机械检查项：① 二进制 / 组件结构合法；② feature allowlist（显式关 threads / shared-everything / cm-threading，不依赖默认值）；③ import allowlist 与 Metis world + WASI 子集匹配；④ export world 与 manifest 声明 ABI 一致；⑤ ABI 指纹 = 规范化 `component wit --json` 哈希 + WIT 包版本；⑥ 资源上限（初始/最大内存、表、组件大小、start 函数）；⑦ producer / name metadata 只作线索，不作信任依据。
- 局限：结构校验不能证明无恶意逻辑，运行时 capability sandbox 仍必需。

**落点**：D108/D114 的检查面；Metis 在本地构建也执行同一套结构校验。

## 4. 部署内工具链服务与安全边界

- WASI 目标不能 spawn 进程/线程；`wasm32-wasip1` 文档明确 spawn thread / process 总是返回错误。rustc / cargo 需要进程与线程，因此构建必须落在外部容器 / 微虚拟机，不能放进 Metis 的 wasm 沙箱。
- 安全边界：microVM（Firecracker：受限设备、jailer、rate limiter）或 gVisor（用户态内核、runsc）；只读 rootfs；网络默认拒绝（docs.rs 先例：任何 crate 都不开网络）；per-job cgroup CPU / RAM / disk / time；构建环境无长期密钥；provenance 由控制平面生成，构建脚本不可见签名密钥；只允许声明的输出路径；缓存 key 覆盖 source commit + lockfile + toolchain image digest + target + features + flags；缓存写权限只给受信 builder；发布前人工 / 策略审批。
- 配额参考：docs.rs 约 6.4 GB / 15 min / 网络阻断 / 10 targets；Deno 约 2 vCPU / 3 GB / 8 GB；Vercel 约 8 GB / 32 GB / 45 min。不必照抄，但必须有硬上限。

**落点**：D113 的硬约束；v1 不做。

## 5. 未确认项与确认方法

- **wash v2 是否仍 watch**：v2 文档称 watch，当前 `dev.rs` 未见 watcher。确认：运行 `wash dev`，修改 `src/lib.rs` 观察是否出现 rebuild 日志；或核对发布 tag。
- **Zed 是否自动重编**：源码只看到显式 `rebuild_dev_extension` 与 reload watcher。确认：修改源码后观察 `extension.wasm` mtime 是否在未点 Rebuild 时变化。
- **Spin 是否有内置构建缓存**：源码显示只执行 manifest 命令。确认：清空 `CARGO_TARGET_DIR` 后连续 `spin build` 两次对比耗时。
- **jco / componentize-js / Extism 是否有隐藏 watch flag**：执行 `jco --help`、`componentize-js --help`、`extism --help`。
- **sccache 对 `wasm32-wasip2` 是否有额外 target 限制**：执行 `RUSTC_WRAPPER=sccache cargo build --target wasm32-wasip2 -vv`，看 `sccache --show-stats` 的 hit / miss 与 not-cached 原因。
- **`wasm-tools component new` 是否保留 core custom section**：给 core wasm 加 `(@custom "metis.build" ...)`，跑 `wasm-tools component new` 后用 `wasm-tools objdump` 或 `wasmparser` 检查。
- **Rust 官方 `reproducible-build` 测试是否覆盖 `wasm32-wasip2`**：查 rust-lang/rust 的 run-make 测试清单，并在不同路径构建两次比对 sha256。
- **Wasmtime `.cwasm` key 字段跨版本稳定性**：构造两种 Engine 配置，比较 cache 目录 hash 与 `COMPILER_VERSION`。
- **Fermyon Cloud / wasmCloud 是否有服务端构建路径**：查官方部署文档 / CLI 源码。
- **关闭 `THREADS` 后 shared memory / atomics 的拒绝形态**：用含 shared memory 的样例在钉版 `wasm-tools` 上跑 `validate`，把错误码固化进测试。

## 6. 来源

- Spin：https://spinframework.dev/v3/running-apps ；https://spinframework.dev/v3/manifest-reference ；https://spinframework.dev/v3/cli-reference ；https://spinframework.dev/v3/cache ；https://github.com/spinframework/spin/blob/main/src/commands/watch.rs ；https://github.com/spinframework/spin/blob/main/src/commands/watch/buildifier.rs
- wasmCloud / wash：https://wasmcloud.com/docs/v1/ecosystem/wash/dev/ ；https://wasmcloud.com/docs/wash/developer-guide/ ；https://wasmcloud.com/docs/wash/commands/ ；https://github.com/wasmCloud/wasmCloud/blob/main/crates/wash/src/cli/dev.rs ；https://github.com/wasmCloud/wasmCloud/blob/main/crates/wash/src/cli/component_build.rs
- Zed：https://zed.dev/docs/extensions/developing-extensions ；https://github.com/zed-industries/zed/blob/main/crates/extension_host/src/extension_host.rs ；https://github.com/zed-industries/zed/blob/main/crates/extension/src/extension_builder.rs
- cargo-watch / cargo-component / wasm-pack：https://github.com/watchexec/cargo-watch/blob/main/cargo-watch.1.ronn ；https://github.com/watchexec/cargo-watch/blob/main/USAGE.md ；https://github.com/bytecodealliance/cargo-component ；https://github.com/wasm-bindgen/wasm-pack ；https://github.com/wasm-bindgen/wasm-pack/blob/master/src/command/build.rs
- jco / componentize-js / Extism：https://github.com/bytecodealliance/jco ；https://github.com/bytecodealliance/componentize-js ；https://github.com/extism/cli ；https://github.com/extism/extism
- Trunk / cargo-leptos：https://trunk-rs.github.io/trunk/guide/commands/ ；https://trunk-rs.github.io/trunk/guide/configuration/ ；https://github.com/trunk-rs/trunk/blob/main/src/watch.rs ；https://github.com/trunk-rs/trunk/blob/main/src/build.rs ；https://github.com/leptos-rs/cargo-leptos ；https://github.com/leptos-rs/cargo-leptos/blob/main/src/service/notify.rs ；https://github.com/leptos-rs/cargo-leptos/blob/main/src/command/watch.rs ；https://github.com/leptos-rs/cargo-leptos/blob/main/src/service/serve.rs
- Cargo fingerprint / locking / reproducibility：https://github.com/rust-lang/cargo/blob/master/src/compiler/fingerprint/mod.rs ；https://github.com/rust-lang/cargo/blob/master/src/compiler/build_runner/compilation_files.rs ；https://github.com/rust-lang/cargo/blob/master/src/compiler/locking.rs ；https://doc.rust-lang.org/cargo/reference/build-cache.html ；https://doc.rust-lang.org/cargo/reference/environment-variables.html ；https://doc.rust-lang.org/cargo/reference/profiles.html ；https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip2.html ；https://doc.rust-lang.org/rustc/remap-source-paths.html
- sccache：https://github.com/mozilla/sccache/blob/main/docs/Rust.md ；https://github.com/mozilla/sccache/blob/main/docs/Caching.md ；https://github.com/mozilla/sccache/blob/main/docs/Local.md
- cargo 缓存清理工具：https://github.com/holmgr/cargo-sweep ；https://github.com/dnlmlr/cargo-clean-all ；https://github.com/matthiaskrgr/cargo-cache
- Reproducible builds：https://reproducible-builds.org/docs/definition/ ；https://reproducible-builds.org/docs/source-date-epoch/ ；https://reproducible-builds.org/docs/rust/ ；https://wiki.debian.org/ReproducibleBuilds ；https://manpages.debian.org/unstable/dpkg-dev/deb-buildinfo.5.en.html ；https://github.com/kpcyrd/rebuilderd
- Rust 官方可靠性：https://github.com/rust-lang/rust/pull/151517 ；https://github.com/rust-lang/rust/issues/75362
- wasm custom section / validation：https://webassembly.github.io/spec/core/binary/modules.html#custom-section ；https://github.com/WebAssembly/component-model/blob/main/design/mvp/Binary.md ；https://docs.rs/wasm-encoder/latest/wasm_encoder/struct.CustomSection.html ；https://docs.rs/wasmparser/latest/wasmparser/enum.Payload.html ；https://docs.rs/wasmparser/latest/wasmparser/struct.WasmFeatures.html ；https://github.com/bytecodealliance/wasm-tools
- wasmtime cache：https://github.com/bytecodealliance/wasmtime/blob/main/crates/wasmtime/src/compile/runtime.rs ；https://github.com/bytecodealliance/wasmtime/blob/main/crates/cache/src/lib.rs ；https://docs.wasmtime.dev/cli-cache.html
- 远端构建场 / 信任链：https://docs.rs/about/builds ；https://docs.brew.sh/Bottles ；https://nix.dev/tutorials/first-steps/towards-reproducibility-pinning-nixpkgs ；https://nix.dev/manual/nix/latest/command-ref/new-cli/nix3-store-verify ；https://bazel.build/remote/rbe ；https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto ；https://bazel.build/remote/caching ；https://slsa.dev/spec/v1.0/levels ；https://slsa.dev/spec/v1.0/provenance ；https://docs.sigstore.dev/cosign/signing/overview/ ；https://docs.sigstore.dev/cosign/verifying/verify/ ；https://github.com/in-toto/attestation/blob/main/spec/README.md
- registry 构建路径：https://doc.rust-lang.org/cargo/reference/registry-web-api.html ；https://docs.npmjs.com/cli/v10/commands/npm-publish ；https://docs.npmjs.com/generating-provenance-statements ；https://docs.deno.com/deploy/reference/builds/ ；https://vercel.com/docs/deployments/builds ；https://developers.cloudflare.com/workers/ci-cd/builds/ ；https://developers.cloudflare.com/workers/wrangler/commands/ ；https://spinframework.dev/v3/quickstart ；https://wasmcloud.com/docs/ ；https://wasmcloud.com/docs/wash/developer-guide/build-and-publish.md
- 部署内构建 / 沙箱：https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1.html ；https://firecracker-microvm.github.io/ ；https://gvisor.dev/docs/ ；https://docs.wasmtime.dev/security.html
