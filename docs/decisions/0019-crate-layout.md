# 0019. Crate 划分与 lib/ 模块体系

- Status: accepted
- Date: 2026-10-02

## Context

实现期临近，组件需要物理归位。fiber-core 的纯度是测试分层（L1 proptest / L2 基于模型 / L4 形式化）的结构前提（`../design/fiber.md` §9）：事后从胶水中剥离纯核心会被随手调用侵蚀，须第一天锁定。`lib/` 引用语法影响工具链生态——经核实 Luau 官方 require-by-string alias 机制已实现（RFC `require-by-string-aliases`，✅），无需自造方言。讨论中用户指正了长期隐含的"插件作者在部署内开发"误解，三作者语境由此确立，`.luaurc` 归属与契约包分发形态随之重塑。

被淘汰的选项：

- **fiber-core 与 runtime 合并（纯度靠纪律）**：依赖污染使 L1/L2/L4 目标无法在无 tokio/FFI 环境运行；Kani/hax 直译要求代码不碰 tokio
- **keyed diff / reconciliation 并入 fiber-core**：核心变杂；loader 是另一个"碰巧很纯"的 crate，平级不合并
- **runtime 按 events/registry/host/journal 拆四个 crate**：过碎的跨 crate 样板；内部模块化、纯→纯后拆无风险
- **自造 require 方言**：失去官方工具链（luau-lsp 的 alias 解析、`.luaurc` 生态）红利
- **插件子树嵌套 `.luaurc` 生效**：alias 覆盖 = 契约欺骗攻击面（插件不可信）

## Decision

### Workspace 布局（C1）

```
crates/
├── metis-value        # Value 数据模型（ADR-0015）——零依赖纯数据
├── metis-fiber-core   # 纯核心：状态机/账本/epoch/失效闭包（禁 tokio/mlua）
├── metis-runtime      # 胶水 + 事件表四派发 + 服务注册表 + host 路由 + journal 捕获
├── metis-luau         # mlua FFI 边界隔离（Miri 目标）：ctx、Value↔Lua、require 包装器
├── metis-loader       # entry 树/manifest/schema/keyed diff——纯数据算法 crate
├── metis              # binary：CLI / main / miette 顶层
└── xtask              # 既有
```

依赖单向：`value ← {fiber-core, loader} ← runtime ← luau ← metis`（loader 与 fiber-core 平级、仅依赖 value；runtime 依赖两者——配置 watcher 的 IO 在 runtime 胶水，reconcile 纯算法在 loader）。粒度哲学：**纯度边界即 crate 边界**（结构强制而非纪律）；runtime 先粗（内部模块化），后拆无风险。

### fiber-core 纯度（C2，定案 [ADR-0010](0010-fiber-core.md) 遗留的"纯 crate 切法待定"项，闭环 `fiber.md` §9）

- **进纯核心**：状态机转移（`state + event → state + 动作`）、账本记账/LIFO drain 次序、epoch 指纹比较、失效集反向闭包；**返回动作描述（数据），胶水层解释执行**——L2 对偶参考模型与 L4 直译的理想形状
- **留胶水层**：执行 disposer、邮箱收发、VM 调用、tokio worker、JoinSet 汇合、定时器、IO、tracing
- **依赖纪律**：仅允许纯数据依赖（slotmap/indexmap/thiserror 级）；禁 tokio/mlua；不用 tracing（返回事件数据由胶水记录）

### lib/ 模块体系（C3）

- **语法 = Luau 官方 require-by-string alias**（✅ RFC Implemented）：`./x` 插件内部、`@lib/x` 共享库（alias 大小写不敏感——官方语义，大小写变体欺骗由官方匹配兜底）；lib 目录锚点 `init.luau`（Luau 惯例；插件包入口 `main.luau` 是 loader 概念，[ADR-0004](0004-plugin-forms.md)，不冲突）；`@<scheme>/` 扩展位兼容官方裸 `@` 保留
- **实现 = host require 包装器**：alias 替换 → 解析 → 边记录 → 跨插件逃逸响亮拒绝（[ADR-0003](0003-module-require-discipline.md) 职责不变）
- **三作者语境**：平台开发者 = monorepo（唯一"开发即生产"）；第三方插件作者 = 自有 git 仓库；agent（Creator）= 部署内创作
- **`.luaurc` 归属**：运行期只认部署根一份（平台布线 `lib → ./lib`，部署者可追加）；插件仓库的 `.luaurc` 是开发期产物（alias 指本机契约包来源），不随包分发、运行期不可见
- **lib 两层**：**内嵌层**（平台契约 + 平台工具库，**契约版本 ≡ 核心版本**，内嵌二进制）/ **磁盘层**（部署根 `lib/`，用户第三方库，审批事务热更）；同名冲突响亮
- **作者 lib 来源**：v1 = 本机 metis 检出（A）；ABI 阶段 = `metis sdk export`（C：从内嵌二进制导出带版本戳副本，install 时与部署内嵌 rev 比对响亮）；第三方时代 = registry（B，[ADR-0004](0004-plugin-forms.md) 推迟项）
- **lib 护栏**：纯代码（每 VM 独立副本，[ADR-0005](0005-vm-topology.md)）；共享代码非共享实体（要共享状态做服务插件）；少而精（变更 = 反向闭包重启 + 审批）

## Consequences

- 闭环 lib/ 引用语法、crate 布局、fiber-core 切法三项遗留问题；编码解锁：起步顺序 metis-value → metis-fiber-core
- lib 写入点恒为两个（metis 仓库 / 部署磁盘层）；插件仓库永非写入点——三方漂移在源头被掐死
- 一切版本/命名错位的归宿 = 响亮失败（install 版本戳比对 + 同名冲突 + [ADR-0018](0018-service-registry.md) 方法存在性兜底）——不做兼容凑合
