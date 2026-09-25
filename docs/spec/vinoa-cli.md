# vinoa CLI 规格（v1 · 定稿）
> **状态**：定稿。草稿中所有"待定"项凡属实现取舍的，已按 rulings 层裁定并入正文；仍是开放问题的只留在附录"已知风险"。
> **版本**：vinoa 0.1.0。**读者**：第一次接触本项目的实现者，不需要读草稿。
> **来源标注**：`[VM]`=[version-matrix-sources](../research/version-matrix-sources.md)、`[LEG]`=[legacy-mc-build-feasibility](../research/legacy-mc-build-feasibility.md)、`[PAPER]`=[paper-plugin-project-shape](../research/paper-plugin-project-shape.md)、`[PROXY]`=[proxy-plugin-project-shape](../research/proxy-plugin-project-shape.md)、`[RUST]`=[rust-scaffold-cli-facts](../research/rust-scaffold-cli-facts.md)。研究结论只引用、不重新讨论。
## 1. 目标与范围
vinoa 是 Rust 实现的 CLI，把"选 MC 版本 + 选平台 + 选附加模块"翻译成一个**能直接 `./gradlew build`** 的 Minecraft 插件工程（Gradle Kotlin DSL、多模块、`gradle/libs.versions.toml` 版本目录）。同一套骨架同时覆盖 MC 1.8.9（Java 8 字节码、Spigot API）与 MC 26.2（Java 25、Paper API），靠**每模块独立的 toolchain / 坐标 / 元数据**表达，**不靠两套模板**。

**平台清单（7 个）**：`paper` / `bukkit`（服务端，一等）、`velocity` / `bungeecord`（代理端，一等）、`folia` / `sponge` / `minestom`（服务端，**experimental**，不进 A 级承诺）。
**Nukkit 已移出**：Cloudburst Nukkit 与 PowerNukkitX 是**基岩版**服务端，版本轴是 Bedrock protocol 号（如 Bedrock 26.50 / protocol 2193），与 Java 版 MC 版本轴不可混用 `[VM §0/§4.6]`。
**支持窗口**：MC `1.8.9` → `26.2`（含两端）。`26.3` 上游已发布，但 Paper 侧只有 `ALPHA` 构建（39 ALPHA / 0 STABLE），故不作为可选目标；Paper 官方自己的 `findLatest` 也是取"最新一个存在非 ALPHA 构建的版本"，当前同样得到 `26.2` `[VM §1.4][PAPER §6]`。上移窗口 = 更新矩阵数据（§6），不是改代码。
**Windows 正式支持**：原生 `cmd` / PowerShell 是目标环境（向导界面需跨平台适配，路径与 `git init` 行为一并考虑），CI 上 `windows-latest` 必跑（§12.3）。
**实现栈**（来自研究，非用户决策）：`edition = "2024"` / `rust-version = "1.85"`；`clap` 4.6.7（只解析，不做交互）；`inquire` 0.9.4（向导）；`rust-embed` 8.12.0（模板内嵌，CI 加 `debug-embed`）；`minijinja` 2.24.0（渲染）；`tempfile` 3.27.0 + `std::fs::rename`（原子落盘）；退出码用 `sysexits` 常量 `[RUST §0/§3/§5]`。
### 1.1 承诺分级
- **A 保证**：vinoa CI 每个 PR 真跑一次 `./gradlew build`，绿了才算验收。只覆盖 `paper` / `bukkit` / `velocity` / `bungeecord` 的代表性组合（§12.2）。
- **B 尽力而为**：同一套规则渲染、坐标经矩阵校验，但**不保证** `./gradlew build` 通过；验证内容是"渲染 + 依赖解析 + 静态断言"。
- **C 拒绝**：组合本身不存在。非交互路径硬报错并列出可用项，绝不静默裁剪。

**用户已确认**：只有代表性组合保证构建通过，其余尽力而为。**生成成功 ≠ 构建成功**；`--verify` 是唯一把两者绑起来的手段（§11.4）。
## 2. 术语
| 术语 | 含义 |
|---|---|
| MC 版本 | `1.8.9` / `1.21.11` / `26.2` 这类 Minecraft 发布版本号；日期式（`26.x`）与 `1.x.y` 同轴 |
| 平台 | 目标运行环境；分**服务端平台**与**代理端**（velocity / bungeecord，不锁 MC 版本） |
| 矩阵 | 内置版本矩阵（§6）。平台坐标、Java 目标、Gradle/插件地板、三方库版本的**唯一事实源** |
| 能力开关 | Rust 侧把原始变量折叠成的布尔量（`cap_*` / `is_*` / `has_*`）；模板与清单**只认开关**（§7.5） |
| 相位 G / B | G = generate-time（vinoa 用 minijinja 渲染）；B = build-time（Gradle `processResources`，**只展开 `version`**） |
| 生成物 / 模板集 | 被生成的插件工程；一棵模板树 + 根下的 `vinoa-template.toml` 清单 |
| 计划（plan） | §5 阶段 4 产出的完整文件清单 + 内容 + 额外动作。`--dry-run` 打印的就是它，落盘只执行它 |
| 目标目录 / core 边界 | `vinoa init <name>` → `./<name>`，`vinoa init` → 当前目录；`core/` 禁止 import 任何平台 API（§8.2） |
## 3. 命令面
两个子命令：`vinoa init [名称] [选项]`、`vinoa versions [选项]`。
### 3.1 `vinoa init --help`
```
用法: vinoa init [名称] [选项]

参数:
  [名称]                   工程名；省略时取当前目录名（就地初始化）
常用:
  -p, --package <包名>      Java 包名（默认 com.example.<名称去连字符>）
  -m, --mc <版本>           目标 Minecraft 版本（如 1.21.11 / 26.2）
      --platform <平台>     目标平台，可重复：paper,bukkit,velocity,bungeecord,folia,sponge,minestom
      --features <列表>     附加模块，逗号分隔（见 §3.3）
  -o, --output <目录>       输出到指定目录（目标目录非空时的首选处置）
  -y, --yes                全部用默认值，不询问
      --dry-run            只打印将要生成的内容（走同一条渲染路径）
      --json               以 JSON 输出（给脚本 / agent 解析），恰好一个文档
      --verify             生成后跑一次构建验证（默认离线）
高级:
  -c, --config <文件>       从 TOML 读取全部答案（与向导等价，可版本化、可进仓库）
      --print-config       把本次解析出的答案导成 TOML
      --metadata <格式>     plugin.yml | paper-plugin（默认 plugin.yml）
      --license <SPDX>      默认 Apache-2.0；其它值报 template.license_unsupported
      --author / --description / --lang <zh|en|both> / --ui-lang <zh|en>
                            作者（可重复）/ 描述 / 生成物语言（默认 zh）/ 界面语言（默认跟随系统）
      --bstats-id <数字>    勾选 bStats 时必填；缺失硬报错，绝不生成假 id
      --template <路径|git-url#ref>   整套替换模板集（§7.9）
      --download-jdk / --no-download-jdk   缺 Java 时是否让 Gradle 首次构建自动下载
      --verify-online / --verify-tail <N>  允许 --verify 联网（默认离线）/ 日志尾部行数（默认 20，上限 200）
      --offline / --online  矩阵取内置 / 单次联网（§6.4）
      --no-git / --no-example / --no-permissions / --no-quality   关闭默认开启的四项
```
无 `--website`：`website` 经 `-c` 提供（参数面按用户口径收敛，不新增一族 `--with-xxx`）。
### 3.2 参数语义
| 参数 | 语义 |
|---|---|
| `[名称]` | `vinoa init` 就地作用于当前目录；`vinoa init <name>` 新建子目录。就地初始化时目标目录非空同样报错（§11.3） |
| 覆盖优先级 | **显式 CLI flag > `-c/--config` > 向导交互 / 默认值**。三层不叠加、不合并 |
| `-c/--config` | 提供与向导等价的一组答案。TTY 且未给 `--yes` 时，`-c` 未覆盖的字段仍按向导补问；非 TTY 下缺必填项 → 硬报错并列出缺失键（不挂起） |
| `--yes` | 跳过确认步骤；**非 TTY 自动等价于 `--yes`**。不隐含 `--verify`，也不隐含下载 JDK |
| `--dry-run` | 跑完校验 / 矩阵 / 预检 / 渲染 / 自检，但**一个文件都不写**、不 `git init`；与真实运行**同判定、同退出码**（"预览即实际"） |
| `--json` / `--verify` | stdout 恰好一个 JSON 文档（人类文本走 stderr）；生成成功后额外跑一次 `./gradlew build`（§11.4，向导里不出现该开关） |
| `--bstats-id` | 勾选 `bStats` 而缺 id 时硬报错，并说明去哪拿（bstats.org 的 plugin id）；**绝不用假 id 兜底** |
### 3.3 `--features` 取值
| 值 | 所属 | 向导 ④ 页 | 默认 |
|---|---|---|---|
| `sqlite` / `bstats` / `update-check` / `placeholderapi` / `gui` | 附加模块 | ✓ 出现 | off |
| `spotbugs` / `coverage` / `release-ci` | 质量工程子项 | ✗ 仅 CLI | off |
前五个是用户已确认的"挑上就生成"开关；后三个是质量工程内部子项，默认关、只在 `--features` 里开放（§9.4）。
### 3.4 `vinoa versions`
```
用法: vinoa versions [--refresh] [--matrix] [--mc <版本>] [--platform <平台>] [--json]
```
`--refresh` 从 `fill.papermc.io/v3` 拉取校验后写入**用户缓存目录**（不写进生成的工程）；`--matrix` 打印内置矩阵全表；`--mc` / `--platform` 过滤。刷新失败回退内置矩阵并警告，**不阻塞** `init`。矩阵与缓存都不进生成物；生成物构建时不再查询矩阵、不再联网（§7.3）。
## 4. 交互向导
形态对齐 **IDEA 的"新建项目"向导**：逐页填写，字段与顺序照它，页间可回退。**向导只在 TTY 出现**；`--yes` / 非 TTY 全走默认值。界面语言跟随系统语言（中文系统中文，否则英文），`--ui-lang` 可覆盖——它与**生成物语言**是两个独立概念。

```
┌─ 新建 vinoa 工程 ─────────────────────────────────┐
│ ① 工程    名称 my-plugin · 位置 /root/projects/my-plugin · 包名 com.example.myplugin
│ ② 构建    语言 Java · 构建系统 Gradle · DSL Kotlin (build.gradle.kts)
│           JDK 21  ✓ 已检测到                      [下载/选择…]
│ ③ 目标服务端
│   MC 版本 1.21.11  ▾（只列我们支持的版本）
│   平台    [x] paper  [ ] velocity  [ ] …
│           （勾 paper 自动带上 bukkit 兼容模块）
│   元数据  (•) plugin.yml   ( ) paper-plugin
│ ④ 附加
│   [x] 示例代码（一条命令 + 一个监听器 + config）   [ ] SQLite 持久化
│   [x] 权限声明（permissions + 权限常量类）         [ ] bStats 统计
│   [x] 质量工程（checkstyle + JUnit + CI）          [ ] 更新检查
│   [x] git init + 首次提交                          [ ] PlaceholderAPI
│   生成物语言  中文 / English / 双语                [ ] GUI 菜单骨架
└──────────────────────────────────────────────────┘
  ← → 切页   Tab 下一项   Enter 确认   Esc 取消
```
### 4.1 ①② 工程页与构建页
| 字段 | 默认 / 取值 | 校验 / 行为 |
|---|---|---|
| 名称 | 就地初始化时取当前目录名 | `^[A-Za-z][A-Za-z0-9_-]{0,63}$`；不合法 → `var.invalid_value` |
| 位置 | 就地 = 当前目录；否则 `./<名称>` | 目标目录已存在且非空 → §11.3 |
| 包名 | `com.example.` + `pluginId` 去掉 `-` | 每段 `^[a-z_][a-z0-9_]*$` 且不是 Java 关键字 |
| 语言 / 构建系统 / Gradle DSL | `Java` / `Gradle` / `Kotlin (build.gradle.kts)` | v1 各只有唯一选项（Paper 官方文档只覆盖 Kotlin DSL `[PAPER §3]`） |
| JDK | 只读展示检测结果 + `[下载/选择…]` | **向导不直接问 Java 版本**；Java 由矩阵按 MC 版本推出（§10） |
### 4.2 ③ 目标服务端页
| 字段 | 默认 | 行为 |
|---|---|---|
| MC 版本 | 矩阵窗口内最新稳定目标（当前 `26.2`） | **先选版本**；下拉只列受支持版本 |
| 平台 | 无预选（`paper` 推荐） | **再选平台**，多选。只列该 MC 版本支持的平台；**代理端不锁 MC 版本，恒在**。勾 `paper` **自动带上 `bukkit`**（无须用户再勾一次），去重后按固定优先级排序 |
| 元数据 | `plugin.yml` | `plugin.yml` / `paper-plugin`（Experimental）**二者互斥，绝不并列生成** |
### 4.3 ④ 附加页
| 字段 | 默认 | 说明 |
|---|---|---|
| 示例代码 | **勾** | 一条命令 + 一个监听器 + `config.yml` |
| 权限声明 | **勾** | `permissions:` 段 + 权限常量类 |
| 质量工程 | **勾** | Checkstyle + JUnit + 生成物 CI（§9.4） |
| SQLite 持久化 / bStats 统计 / 更新检查 / PlaceholderAPI / GUI 菜单骨架 | 不勾 | 分别对应 `--features`；勾 `bStats` 后**向导追问一格数字 plugin id**，非交互用 `--bstats-id`，缺失硬报错 |
| git init + 首次提交 | **勾** | `--no-git` 关；落盘成功之后才 init（§9.7） |
| 生成物语言 | 中文系统 `zh`，否则 `en` | `zh` / `en` / `both`。跟随它的是：注释、README、玩家可见消息；与 `--ui-lang` 无关 |
`integrationTest` source set **不出现在向导里**：只在勾了服务端平台 `paper` / `folia` 时生成，且必须至少有一个真跑的测试；否则不生成空源集（§9.4）。
### 4.4 交互约定与确认步骤
`←/→` 切页、`Tab` 下一项、`Enter` 确认、`Esc` 取消（不落盘）；页间可回退，回退后改上游字段会重跑校验与矩阵解析（阶段 2→3）。缺 Java 时在**确认摘要里**用 ✓/✗ 展示，并追问一句"是否让 Gradle 首次构建时自动下载？"（§10.3）。

确认步骤默认打印摘要（"即将创建（共 N 个文件）"+ 目录树）并问 `Y/n`；`--yes` 与非 TTY 跳过。完成提示必须包含五件东西：

```
✓ 完成: /root/projects/my-plugin

下一步:
  cd my-plugin
  ./gradlew build          # 构建插件 jar
  ./gradlew runServer      # 起本地测试服（首次需联网）
  注意: 当前目标是 1.21.11（Java 21）。
  文档: <仓库内对应平台的 README 章节链接>
```
- 目标为 `1.8.9` 时第 4 行改为明确说明"**Paper 没有 1.8.9，起不了本地 Paper 测试服**"，并给 BuildTools / vanilla 自建服提示（由用户自行调用，vinoa 不下载、不分发、不代执行）`[LEG §1.4/§4.3]`。
- `runTask = none` 时第 3 行替换为等价替代说明，不打印会被误认为可用的命令。
## 5. 执行流程与阶段
单一代码路径：**先算完整计划 → 落盘只执行计划**，因此 `--dry-run` 的输出与实际写入必然一致。

| # | 阶段 | 职责 | 失败产物 |
|---|---|---|---|
| 0–1 | 上下文与 TTY 检测；输入收集 | 目录占用、是否在 git 仓库内、是否离线、是否 TTY；向导填写 / flags / `-c` 补齐 | 无 |
| 2 | 校验 | 名称、包名合法性、目录占用、`(mc, platform)` 组合（**一次列全所有非法字段**） | 无 |
| 3 | 版本矩阵解析 | 锁定平台坐标 / Java 目标 / Gradle 与插件地板 / 元数据格式能力 / 能力开关 | 无 |
| 3.5 | 环境预检 | 按已解析的 Java 目标查本机 JDK，产出 ✓/✗ 清单；缺失时触发"是否自动下载"询问 | 无（不阻塞；`--verify` 且缺失见 §10.3） |
| 4 | 计划生成 | 完整文件清单 + 内容 + 额外动作（= `--dry-run` 的产物） | 无 |
| 5 | 渲染 | 模板树 → 内存文件集（变量替换、条件文件/片段、重命名），随后跑自检 A1–A6 | 首错即停，**不写盘** |
| 6 | 落盘 | 先写临时目录，再整体 `rename`（原子） | 回滚，无残留 |
| 7 | 收尾 | `git init` + 首次提交；可选 `--verify` | 见 §11.1 |
| 8 | 结果输出 | 人类可读摘要（stderr）/ `--json`（stdout） | — |
**可复现硬要求**：同一 CLI 版本 + 同一输入 → **字节相同**的文件树。时间戳、年份、随机种子、机器路径一律不得进入产物。该约束只约束**生成阶段**（阶段 0–6），不约束构建阶段。
## 6. 版本矩阵契约
**解决的问题**：用户只选 MC 版本 + 平台；矩阵把选择翻译成可构建的工程参数，并回答"哪些组合根本不成立"。
### 6.1 数据模型：按平台白名单，不做范围推导
范围推导必然出错：Paper 缺 `1.8.9` / `1.20.3` / `1.21.2` / 裸 `26.1`，Folia 最早 `1.19.4`，Minestom 只有 4 个版本，Sponge 有 6 个缺口 `[VM §0/§6]`。**只有明确存在的版本才进表。**

```toml
# templates/version-matrix.toml（编译进二进制）
schema = 1
generated_at = "2026-09-25"

[java]   # java_min 是工具链事实源；recommended 只用于提示文案
# 全表 66 行（MC 1.8.9 → 26.3），数据与来源见 [VM §7]
"1.8.9" = { min = 8, recommended = 8 }  "1.12.2" = { min = 8, recommended = 11 }
"1.16.5" = { min = 8, recommended = 16 }  "1.17.1" = { min = 16, recommended = 17 }
"1.20.6" = { min = 21, recommended = 21 }  "1.21.11" = { min = 21, recommended = 21 }
"26.1.1" = { min = 25, recommended = 25 }  "26.2" = { min = 25, recommended = 25 }

[platform.bukkit]                        # 1.8.x 的唯一选择
api = "org.spigotmc:spigot-api"
versions = ["1.8.9", "1.12.2", "1.16.5", "1.21.11", "26.2", "…"]
coordinate_map = { "1.8.9" = "org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT",
                   "1.12.2" = "org.spigotmc:spigot-api:1.12.2-R0.1-SNAPSHOT",
                   "1.16.5" = "org.spigotmc:spigot-api:1.16.5-R0.1-SNAPSHOT" }

[platform.paper]
api = "io.papermc.paper:paper-api"
versions = ["1.9.4", "1.10.2", "…", "1.21.11", "26.1.1", "26.1.2", "26.2"]
legacy_namespace_until = "1.16.5"    # 1.9.4–1.16.5 用 com.destroystokyo.paper
legacy_version_until = "1.21.11"     # ≤1.21.11 用 {MC}-R0.1-SNAPSHOT
pinned = { "26.2" = "io.papermc.paper:paper-api:26.2.build.129-stable" }

[platform.velocity]                  # 代理端：协议范围，不锁 MC 版本
model = "protocol"; protocol_min = "1.7.2"; protocol_max = { "4.2.0" = "26.2" }
default_api = "com.velocitypowered:velocity-api:4.2.0"; java = 25

[platform.bungeecord]                # 代理端：大颗粒版本，每个坐标自带 java
model = "major-grain"; default = "1.21-R0.4"   # 默认 release 线；快照线为可选（见 §8 与 §12.2 A7）
coordinates = { "26.1-R0.1-SNAPSHOT" = { api = "net.md-5:bungeecord-api:26.1-R0.1-SNAPSHOT", java = 17 },
                "1.21-R0.4" = { api = "net.md-5:bungeecord-api:1.21-R0.4", java = 8 } }

[platform.folia]    { api = "io.papermc.paper:paper-api", min_version = "1.19.4" }
[platform.minestom] { versions = ["1.21.11", "26.1.1", "26.1.2", "26.2"], java = 25 }
[platform.sponge]   { api_versions = { "1.8.9" = "4.2.0", "1.9.4" = "5.0.0", "1.12.2" = "7.4.8",
                                       "1.16.5" = "8.2.1", "1.21.11" = "18.0.0", "26.2" = "20.0.0" } }

[thirdparty]   # 三方坐标并入矩阵，不另设事实源；取值见附录 R6
placeholderapi = "me.clip:placeholderapi:2.11.6"; bstats = "…"; sqlite_jdbc = "…"; gui = "…"

[gradle]       # 插件地板取自各自 .module 的 org.gradle.plugin.api-version，不抄文档
version = "9.8.0"; run_on_jvm = "17-27"
shadow       = { version = "9.6.1",         gradle_min = "9.2.0", java = 17 }
run_paper    = { version = "3.1.0",         gradle_min = "9.7.0", java = 17 }
run_velocity = { version = "3.1.0",         gradle_min = "9.7.0", java = 17 }
paperweight  = { version = "2.0.0-beta.24", gradle_min = "9.7.1", java = 21 }

[quality]      # 老版本模块的降级下界
checkstyle_java8_max = "9.3"; spotbugs_java8_max = "4.8.6"; junit6_java = 17
junit_java8_max = "5.14.4"; gradle_checkstyle_default = "10.24.0"
```
`[thirdparty]` 只有 `placeholderapi` 的取值有草稿来源，其余三个是**数据填充**（实现时按 primary source 补齐，矩阵是唯一写入点）——见附录 R6。
### 6.2 必须精确的事实锚点
| 事实 | 值 | 来源 |
|---|---|---|
| MC 1.8.9 的 API | `org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT`（**不存在 1.8.9 制品**） | `[LEG §1.1]` |
| 1.8.x 的 Paper API | **不存在**（`com.destroystokyo.paper` 最早 `1.9.4`） | `[VM §2.2][LEG §1.2]` |
| `io.papermc.paper:paper-api` 下界 | MC **1.17** 起；之前用 `com.destroystokyo.paper:paper-api` | `[VM §0/§2.1]` |
| 坐标字符串格式翻转点 | **26.1**：`{MC}-R0.1-SNAPSHOT` → `{MC}.build.+` / `{MC}.build.{N}-{channel}` | `[VM §2.1]` |
| Java 硬下界 `java_min` 阶梯 | `8` 1.8.9–1.16.5 · `16` 1.17–1.17.1 · `17` 1.18–1.20.4 · `21` 1.20.5–1.21.11 · `25` 26.1+ | `[VM §7.1]` |
| Java 推荐值阶梯 | `8` ≤1.11 · `11` 1.12–1.16.4 · `16` 1.16.5 · `17` 1.17–1.19 · `21` 1.20–1.21.11 · `25` ≥26.1 | `[VM §3.1]` |
| 最后一个 Java-21 版本 / 第一个 Java-25 版本 | `1.21.11` / `26.1`（Paper 发布 `26.1.1`、`26.1.2`，无裸 `26.1`） | `[VM §3.3]` |
| Gradle | 稳定 `9.8.0`；**运行 Gradle 需 JVM 17–27**；Java-25 toolchain 需 Gradle ≥ 9.1.0 | `[LEG §2.1][VM §5.1]` |
| Java 8 目标 | Gradle 9.8 不能在 Java 8 上*运行*，但能*编译到* Java 8：实测 JDK 21 + `options.release = 8` → major 52 | `[LEG §0/§2.3]` |
| Checkstyle 9.3 / SpotBugs 4.8.6 / JUnit 5.14.4 | 分别是最后一版能在 Java 8 上运行的线路（JUnit 6 是 major 61，需 Java 17） | `[LEG §3]` |
| Velocity 4.2.0 | 需要 **Java 25**（4.x 全部 `java.minimum = 25`） | `[PROXY §0.1/§2.1]` |
| run-paper | **无法启动 1.8.9**（Paper 没有 1.8.9，Fill 返回 `version_not_found`） | `[LEG §4.1]` |
| Folia 下界 / Minestom 窗口 | `1.19.4` / 仅 `1.21.11`、`26.1.1`、`26.1.2`、`26.2` | `[VM §4.1/§4.5]` |
| BungeeCord 制品 | `26.1-R0.1-SNAPSHOT`（Java 17）；release 线 `1.21-R0.4`（Java 8） | `[PROXY §0.1][VM §4.2]` |
### 6.3 查询语义
1. 输入 `(mc, platform)` → **命中 / 未命中**；未命中时同时返回**该 mc 的可用平台**与**该平台的可用版本**，供交互过滤与报错文案使用（§11.3）。
2. Java 取 `java_min`；`java_recommended` **只用于向导与预检的提示文案**，不进 toolchain。因此 1.16.5 的 toolchain 写 **8**（不是推荐值 16），避开非 LTS、CI 难装的坑。
3. 坐标**一律查 `coordinate_map` / `pinned` / `coordinates`**，禁止字符串拼接：`1.8.9 → spigot-api:1.8.8` 这类映射必须显式存在（拼接必然产出 404）。
4. 模块 Java 目标：`paper`/`folia` = 该版本 `java_min`；`bukkit` = `1.8.9`–`1.16.x` → 8，之后取该版本 `java_min`；`velocity` = 25；`bungeecord` = 所选坐标自带 `java`（**默认 release 线 `1.21-R0.4` → 8**；快照线 `26.1-R0.1-SNAPSHOT` → 17 为可选）；`sponge` = 该 `spongeapi` 线要求；`minestom` = 25；`core` = **所有已启用模块中最低的那个目标**。
5. 代理端（`velocity` / `bungeecord`）不参与 MC 版本过滤，恒可选。
### 6.4 离线 / 联网语义
- **默认离线**：内置矩阵是唯一事实来源；同一输入两次运行必须给出完全相同结果。
- `vinoa versions --refresh`：写**用户缓存目录**，**不写进生成的工程**。
- **谁读缓存（2026-09-25 收敛，实现为准）**：`init` 默认**只用内置矩阵**，使“同一 CLI 版本 + 同一输入 → 字节相同的产物”不依赖机器状态；`--online` 才使用刷新后的数据（并落缓存）；`versions` 输出来源三态 `builtin` / `cache` / `online`。缓存服务于 `versions` 与未来的 `build` 命令。理由：缓存只含 Fill 的 release/build 数据，**只能收窄可用版本而不能新增**，而收窄是危险方向（例如误裁 1.8.9 这类无 Paper 上游的版本）。
- `--online`：单次使用联网结果（仍落缓存）；离线时回退内置矩阵并**警告**，不阻塞生成。
- 请求必须带**非通用 `User-Agent`**（标识软件 + 联系 URL/邮箱）`[VM §1.6]`；数据带 `generated_at`，`--json` 标注来源（`builtin` / `cache` / `online`）。
- `?channel=STABLE` 只对 `/builds` 有效；`/builds/latest` **忽略该参数** → 频道过滤在 Rust 侧自己做（取第一个 `channel == "STABLE"`），绝不依赖 `latest` `[VM §1.4]`。
### 6.5 裁剪规则
交互路径**只列支持项**，用户根本选不到不成立的组合；非交互 / `--yes` 路径**硬报错 + 列出可用项**，绝不静默裁剪或换版本，也不做任何补救推断。
## 7. 模板变量契约与渲染管线
### 7.1 变量清单
命名规则：**模板变量一律 camelCase**，与 `-c/--config` 的 TOML 键同名同义、不做别名。`可入路径` = 允许出现在文件名 / 目录名占位符里；`只读` = 用户不能直接赋值。

| 变量 | 类型 | 默认 / 来源 / 派生 | 相位 | 可入路径 |
|---|---|---|---|---|
| `projectName` | string | `vinoa init <name>`；省略时当前目录名；`^[A-Za-z][A-Za-z0-9_-]{0,63}$` | G | ✓ |
| `pluginId` | string 只读 | `slug(projectName)`：小写、丢弃非 `[a-z0-9-_]`；须匹配 `[a-z][a-z0-9-_]{0,63}`（Velocity 规则），否则 `var.invalid_value` | G | ✗ |
| `pluginName` | string 只读 | `PascalCase(projectName)`（`my-plugin` → `MyPlugin`），用作类名叶子与元数据 `name` | G | ✓ |
| `packageName` | string | `-p/--package`；默认 `com.example.<pluginId 去 ->`；每段 `^[a-z_][a-z0-9_]*$` 且非 Java 关键字 | G | ✗ |
| `packagePath` | string 只读 | `packageName.replace('.', '/')` | G | ✓ |
| `mainClass.<platform>` | string 只读 | `packageName + "." + <platform> + "." + pluginName` | G | ✗ |
| `commandName` / `permissionNode` | string 只读 | `slug(projectName)`（截断 32、仅 `[a-z0-9_-]`）/ `pluginId + ".command"` | G | ✗ |
| `projectVersion` | string | 常量 `0.1.0-SNAPSHOT`；**不在向导里问** | G / B | ✗ |
| `group` / `artifactName` | string 只读 | `group = packageName`；`artifactName = projectName` | G | ✗ |
| `gradleVersion` | string | 矩阵 `[gradle].version` = `9.8.0` | G | ✗ |
| `javaTarget` | map<platform,int> | 矩阵 `java_min`（每模块一份，代理端按 §6.3.4） | G | ✗ |
| `toolchainAutoDownload` | bool | 向导"缺 Java 是否让 Gradle 下载"；`--download-jdk/--no-download-jdk`；`--yes` 默认 `false` | G | ✗ |
| `qualityToolVersions` | map<string,string> | 矩阵 `[quality]` + `javaTarget`（checkstyle/spotbugs/junit 降级下界） | G | ✗ |
| `mcVersion` | string | `-m/--mc`；向导先选版本 | G | ✗ |
| `platforms` | list<string> | `--platform`（可重复）/ 向导多选；**选 `paper` 自动并入 `bukkit`**，去重后按固定优先级排序 | G | ✗ |
| `metadataFormat` | enum `plugin.yml` \| `paper-plugin.yml` | 向导 ③ 页；默认 `plugin.yml` | G | ✗ |
| `apiCoordinate.<platform>` | string 只读 | **一律查矩阵**；禁止字符串插值 | G | ✗ |
| `apiFlavor` | enum `spigot` \| `paper` | `bukkit` 模块的坐标风味；**默认 `spigot`**（用户已确认 bukkit 用 `spigot-api`）；仅当 `mcVersion ≤ 1.16.5` 且同工程带 `paper` 时矩阵允许 `paper` | G | ✗ |
| `pluginYmlApiVersion` / `metadataApiVersion` | enum / string | `plugin.yml`：`mcVersion ≥ 1.13` 写 `1.13`（最宽松合法值），否则整行省略；`paper-plugin.yml`：必填，取 `'1.19'`（该格式最低服务端下界） | G | ✗ |
| `librariesEnabled` | bool | `mcVersion ≥ 1.16.5`（`libraries:` 自该 API 版本才有） | G | ✗ |
| `runTask` | enum `run-paper` \| `run-velocity` \| `none` | Paper ≥ 1.8.8 → `run-paper`（`minecraftVersion = mcVersion`）；Velocity → `run-velocity`；**1.8.9（无 Paper）→ `none`** + README 说明 BuildTools/vanilla | G | ✗ |
| `paperweightEnabled` / `reobfEnabled` | bool | 前者仅 `≥ 1.17.1` 且用户显式要 NMS（默认 `false`）；后者 = `paperweightEnabled && mcVersion < 26.1`（26.1 起官方移除 reobf） | G | ✗ |
| `legacyNamespace` / `apiVersionFormat` | bool / enum | 矩阵 `legacy_namespace_until`（`≤1.16.5` 用 `com.destroystokyo.paper`）；格式取矩阵 `legacy_version_until` / `pinned`：`R0.1-SNAPSHOT`\|`build`\|`pinned` | G | ✗ |
**元数据 / 语言 / 许可 / 可选模块**

| 变量 | 类型 | 默认 / 来源 | 说明 |
|---|---|---|---|
| `pluginDescription` | string | `--description`；默认文案取自语言包 | 玩家可见，跟随 `lang` |
| `authors` / `website` | list<string> / string? | `--author`（可重复）/ `-c` 的 `website` | 空则**整个 `authors:` / `website:` 片段不生成** |
| `exampleEnabled` / `permissionsEnabled` / `qualityEnabled` / `gitInit` | bool | 向导 ④ 页默认勾；`--no-example/--no-permissions/--no-quality/--no-git` | `gitInit` 只影响 `.gitignore`/`.gitattributes` 与收尾动作 |
| `featureSqlite` / `featureBstats` / `featureUpdateCheck` / `featurePlaceholderApi` / `featureGui` | bool | `--features`；默认 `false` | 五个独立开关 |
| `bstatsPluginId` | int? | 无默认 | 缺失策略见 §11.1；绝不生成假 id |
| `lang` | enum `zh` \| `en` \| `both` | `--lang`；默认 `zh`（中文系统）/ `en` | 只影响生成物；与 `--ui-lang` 无关 |
| `license` | string | `Apache-2.0` | 模板集只内嵌 Apache-2.0 全文；其它 SPDX id → `template.license_unsupported` |
| `copyrightYear` | int? | **默认不设置** | 只有 `SOURCE_DATE_EPOCH` 存在时才取值；差异必须出现在 `--json` 里（§13） |
**推导顺序固定**：`pluginId → 校验 → packageName → pluginName → mainClass → 能力开关`；每步消费前一步，实现上就是 §5 的阶段 1→3。
### 7.2 generate-time 表达式受限（minijinja）
- 语法 `{{ var }}`、`{% if flag %}…{% endif %}`、`{% for p in platforms %}…{% endfor %}`；**不用 `${}`**，以免与 Gradle 自己的展开撞车。
- **只允许**纯变量输出与分支 / 循环；**不允许**算术、函数调用、过滤器链——所有派生量在 Rust 侧算好（§7.1 的"只读"列）。
- 分支只允许判**能力开关**（`cap_*` / `has_*` / `is_*`），**不允许**直接判 `mcVersion`、`platforms`；这样模板可被静态校验，清单能声明依赖。
- 未定义变量是**硬错误**（`UndefinedBehavior::Strict`，配 `render_named_str` 以便报错带文件名），**不允许**渲染成空串。
- 需要输出字面 `{{` / `{%` 时用 `{% raw %}…{% endraw %}` 包裹；关闭自动转义（`set_auto_escape_callback(|_| AutoEscape::None)`），模板是文本拼接。
### 7.3 build-time：只展开 `version`
**除 `version` 外，§7.1 的所有变量都在 generate-time 落盘定型**：`mcVersion`、坐标、Java 目标、能力开关一律**烘焙进生成物**，生成物构建时不再查询矩阵、不再联网（理由：§6.4 的可复现硬要求 + 离线构建）。

| build-time 变量 | 值来源 | 生成物中的写法 |
|---|---|---|
| `version` | `project.version`（Gradle 单一事实源，CI/发布工具 bump 它） | `version: "${version}"`（**必须加引号**，防 SnakeYAML 数字强转） |

```kotlin
tasks.processResources {
    val props = mapOf("version" to project.version)
    inputs.properties(props)              // 不加这行 up-to-date 判定会错
    filesMatching("plugin.yml") { expand(props) }
}
```
`plugin.yml` 里任何字面 `$` 必须写成 `\$`（SimpleTemplateEngine 的转义）。清单可用 `build_time_vars = [...]` 声明扩展，**默认集合就是上表这一行**。
### 7.4 文件 / 目录名占位符
路径段只允许 `{{ projectName }}`、`{{ pluginName }}`、`{{ packagePath }}`、`{{ platform }}`（**仅这四个**）；其余变量出现在路径里 → `template.bad_path_var`。渲染后每段必须非空、不含 `/` `\` `:`、不是 `.`/`..`（防路径穿越）。二进制文件（`gradle-wrapper.jar`、图片）**只复制不渲染**，由清单 `render = "copy"` 声明。编码统一 UTF-8；行尾统一 LF，**例外**：`gradlew.bat`、`*.bat` 为 CRLF；Unix 上 `gradlew` 落盘后 `set_permissions(0o755)`（Windows 忽略）。
### 7.5 条件语言：只有一套开关
Rust 侧把原始变量折叠成布尔开关，模板与清单**只认开关**：

```toml
[conditions]
cap_bukkit_module    = "platforms contains 'bukkit'"
cap_libraries        = "mc_version >= 1.16.5"
cap_api_version_line = "mc_version >= 1.13"
cap_legacy_namespace = "mc_version <= 1.16.5"
cap_dual_lang        = "lang == 'both'"
is_paper_metadata    = "metadata_format == 'paper-plugin.yml'"
has_authors          = "len(authors) > 0"
cap_sqlite           = "feature_sqlite"
is_lang_zh           = "lang == 'zh'"
```
支持 `all_of` / `any_of` / `not` + 叶子 `var == '值'`、`var >= 值`、`list contains '值'`。**不引入 Rhai 之类表达式引擎**，避免"模板即代码"的信任成本。
### 7.6 条件文件 / 条件片段矩阵
| 维度 | 触发开关 | 影响文件（F）与片段（S） |
|---|---|---|
| 平台 | `cap_bukkit_module` | F `platforms/bukkit/**`；S `settings.gradle.kts` 的 include 行、根 README 模块表、CI 矩阵 |
| 平台 | 每个 `platforms` 元素 | F `platforms/<p>/build.gradle.kts`、`src/main/**`；velocity 模块**没有 resources 目录**（描述符由注解处理器生成） |
| 元数据格式 | `is_paper_metadata` | F `paper-plugin.yml`（**替代** `plugin.yml`，绝不并列）；S 必填 `api-version`、依赖用 `dependencies.server.*`、命令注册走 `LifecycleEvents.COMMANDS` —— 与 `plugin.yml` 路径**二选一** |
| MC 能力 | `cap_api_version_line` / `cap_libraries` | S `api-version: '1.13'` 一行（否则整行省略 = legacy 加载 + 控制台警告）；S `libraries:` 块，为 `false` 时该依赖改走 shading 并在 README 注明 |
| MC 能力 | `cap_legacy_namespace` / `runTask` / `reobfEnabled` | 坐标与仓库全部取自 `apiCoordinate`，模板不分支；F 根 `build.gradle.kts` 的 `run-paper`/`run-velocity` 插件块（`none` → README 给替代说明）；S `paperweight` 的 `reobfJar` 接线（**≥26.1 绝不生成**） |
| 语言 | `is_lang_zh` / `is_lang_en` / `cap_dual_lang` | F `README.md`、`README.en.md`；S 源码注释块、`config.yml` 注释、玩家可见消息（§7.7） |
| 可选模块 | `cap_sqlite` | F `core/.../storage/SqliteStorage.java`；S core 依赖、`config.yml` 的 db 段 |
| 可选模块 | `cap_bstats` / `cap_update_check` / `cap_placeholderapi` / `cap_gui` | 各自 F 一个类：`metrics/Metrics.java`（+ `bstatsPluginId` 常量）、`update/UpdateChecker.java`、`hook/PlaceholderHook.java`、`gui/Menu.java`；S 启动代码、`config.yml` 开关、PlaceholderAPI 的 `softdepend` + 仓库/坐标 |
| 质量 / 示例 / 权限 / git | `cap_quality` / `cap_example` / `cap_permissions` / `cap_git` | F `config/checkstyle/checkstyle.xml`、`.github/workflows/build.yml`、示例命令与监听器、权限常量类、`.gitignore`（`cap_git=false` 时**连它也不生成**）；S 根 `build.gradle.kts` 的 checkstyle/spotbugs/junit 块（版本取自 `qualityToolVersions`） |
### 7.7 语言包机制
模板集内 `i18n/zh.toml`、`i18n/en.toml` 存字符串表（README 段落、注释块、玩家消息、`pluginDescription` 默认值）。渲染时按 `lang` 选择：

- `zh` → 中文串；`en` → 英文串。
- `both` → **注释中英双语（中文在前、英文在后）**；`README.md`（中文）+ `README.en.md`（英文，顶部互链）；玩家消息在每个平台模块的 `resources/lang/` 下同时生成 `zh_CN.yml` + `en.yml` 两份，`config.yml` 里 `default-locale: zh_CN`。
- `zh` 只留 `lang/zh_CN.yml`，`en` 只留 `lang/en.yml`。语言文件统一用 `lang/` 形态（草稿里的 `messages_*.yml` 是同一文件的早期命名）。
- 语言包缺 key → `template.i18n_missing_key`（硬错误，**不允许**回退成空串）。
- 构建文件（`.github/workflows/*.yml`、`build.gradle.kts`）的注释**不**跟随生成物语言，保持英文，避免编码 / 工具解析差异。
### 7.8 模板清单 `vinoa-template.toml`
每个模板集根目录一份，随模板集一起被 rust-embed 内嵌；schema 版本化、字段扁平、无继承，`serde(deny_unknown_fields)` 校验（未知键即拒）。

| 顶层键 | 必填 | 语义 |
|---|---|---|
| `schema` / `id` / `template_version` / `min_cli_version` | ✓ | schema 版本；模板集标识；模板集自身版本；CLI 更低 → `template.version_too_old` |
| `[variables.<name>]` | ✓ | `type` / `required` / `default` / `values` / `pattern` / `derived` / `readonly` / `description` |
| `[conditions]` | 按需 | 开关名 → §7.5 条件表达式 |
| `[dependencies]` | 按需 | **外部模板集自持**的三方坐标；内置模板集不写这里（版本在矩阵 `[thirdparty]`） |
| `[[files]]` | ✓ | `template` / `target` / `render`（`template`\|`copy`）/ `when` / `foreach` + `paths` / `build_time_vars` |
| `[[assertions]]` | 按需 | `target` + `contains[]`（可引用变量），渲染后比对，对应 A1/A3/A5 |

```toml
schema = 1
id = "vinoa/paper-gradle-v1"
template_version = "1.0.0"
min_cli_version = "0.1.0"

[[files]]                      # foreach 即条件：逐元素渲染，路径段 _p_ 被平台键替换
template = "platforms/_p_/build.gradle.kts.jinja"; target = "platforms/{{platform}}/build.gradle.kts"
foreach = "platforms"; paths = ["platform", "gradlePath"]

[[files]]
template = "platforms/paper/src/main/resources/plugin.yml.jinja"
target = "platforms/paper/src/main/resources/plugin.yml"
when = "platforms contains 'paper'"; build_time_vars = ["version"]

[[files]]                      # 二进制只复制不渲染
template = "bin/gradle-wrapper.jar"; target = "gradle/wrapper/gradle-wrapper.jar"; render = "copy"

[[assertions]]
target = "platforms/paper/src/main/resources/plugin.yml"
contains = ["name: {{pluginName}}", "main: {{mainClass.paper}}", 'version: "${version}"']
```
`[variables.*]` 的键名用 snake_case 只是 TOML 书写习惯；**契约上变量名是 camelCase**（§7.1），单一映射函数 `snake↔camel`，不允许另起别名。
### 7.9 内嵌 vs 外部模板：优先级与信任
**优先级（不叠加、不合并）**：`--template <本地路径>` > `--template <git-url#ref>` > 内嵌（默认）。给定外部模板时**完全不读内嵌模板**；只支持"整套替换"，v1 **不做覆盖层 / 文件级合并**（否则"哪个文件赢"无法静态校验，A5/A6 也失去意义）。

外部模板的硬规则：

1. 根目录必须有 `vinoa-template.toml` 且 `schema` 被本 CLI 认识。
2. **模板是纯数据，永不执行**：清单 schema 里没有 `hooks`/`scripts`/`commands` 字段，未知键报错；渲染过程不调用 `git`、不调用 Gradle、不做网络请求。
3. `target` / `template` 必须是相对路径、无 `..`、无绝对路径、无符号链接逃逸（解析后须落在模板根内）。
4. git URL：resolve 到**具体 commit SHA** 并回显；`--json` 记录 `{kind:"git", url, ref, commit}`；可变 ref（分支/tag）额外警告一句。不拉 submodule，模板内的 `.git` 忽略。
5. 渲染前限额：文件数 ≤ 5000、解压后总量 ≤ 32 MiB，超限 `template.too_large`；`--offline` + git URL → 直接报错（**不回退内嵌**：静默换一套模板比失败更糟）；用 git 模板时确认摘要必须打印来源 + commit + 文件数，非 `--yes` 需二次确认。
7. **只校验 `min_cli_version`，不设上限**（`max_cli_version` 只会挡住合理用法，收益为零）。v1 **不做**签名校验（cosign/sigstore），**不做**私有仓库凭据透传——两者都在 README 明说。
### 7.10 渲染后自检（`--dry-run` 也执行）
| ID | 断言 | 失败码 |
|---|---|---|
| A1 | 全部文本文件与文件/目录路径中不再出现 `{{`、`}}`、`{%`、`%}`（排除二进制与 `{% raw %}` 内外） | `render.leftover_placeholder` |
| A3 | 模板集默认身份串 `com.example`、`my-plugin`、`MyPlugin` 不出现在生成物里，**除非**用户选择恰好等于它（比对变量实际值） | `render.stale_default` |
| A4 | 每个平台模块的入口类文件存在，且元数据 `main` 的 FQCN 能按 `src/main/java/<pkg 转路径>/<类>.java` 定位到 | `render.main_class_mismatch` |
| A5 | 清单声明的变量集合与实际渲染消费的变量集合**互为子集**（双向） | `template.variable_mismatch` |
| A6 | `settings.gradle.kts` 的 `include` 集合 == `platforms` 目录集合 == 清单 `foreach` 产出集合 | `render.module_set_mismatch` |
A5 是"零残留"的关键：只靠正则扫残留会漏掉**渲染成空串**的变量，双向核对才能兜住。
### 7.11 重命名规则
| # | 对象 | 规则 |
|---|---|---|
| R1 | 包目录路径 | `src/main/java/{{packagePath}}/…`（core 与每个平台模块各一份）；平台子包 = `{{packagePath}}/<platform>/`，**不平铺** |
| R2 | 主类文件 + 类名 | 文件 `{{packagePath}}/<platform>/{{pluginName}}.java`，内容 `public final class {{pluginName}}`；两者由同一变量驱动，不允许只改一个 |
| R3 | `settings.gradle.kts` | `rootProject.name = "{{projectName}}"`；`include(...)` 由 `{% for p in platforms %}` 生成 `"platforms:<p>"`（Gradle 用 `:` 不是 `/`） |
| R4 | 跨模块引用 | `gradlePath = "platforms:" + platform`；`implementation(project(":core"))`、`project(":{{gradlePath}}")` 全走它，禁止模板里手写路径 |
| R5 | 跨模块 Java 引用 | 平台模块 `import {{packageName}}.core.*`；**core 不 import 任何平台 API**（§8.2） |
| R6 | 元数据 | `plugin.yml: main: {{mainClass.<platform>}}`、`name: {{pluginName}}`、`version: "${version}"`；Velocity **不生成**任何描述符文件（注解处理器产出 `velocity-plugin.json` 到 jar 根） |
## 8. 生成工程结构
`vinoa init my-plugin -m 1.21.11 --platform paper --platform velocity` 生成：

```
my-plugin/
├─ settings.gradle.kts · build.gradle.kts · gradle.properties
├─ gradle/libs.versions.toml · gradle/wrapper/gradle-wrapper.{jar,properties}   # 9.8.0
├─ gradlew · gradlew.bat · .gitignore · .gitattributes
├─ README.md · LICENSE · CHANGELOG.md · .github/workflows/build.yml · config/checkstyle/
├─ core/                              # 平台无关：抽象接口 + 纯逻辑，不 import 任何平台 API
│  ├─ build.gradle.kts                # toolchain = 已启用模块中的最低目标
│  └─ src/main/java/<pkg>/{command/CommandSpec,config/PluginConfig,message/Messages,
│           permission/Permissions,ExampleService}.java + src/test/java/<pkg>/ExampleServiceTest.java
└─ platforms/{paper,bukkit,velocity}/        # 每平台一份 build.gradle.kts + src/main/java/<pkg>/<平台>/
   paper    → PaperPlugin + command/ + listener/ + resources/{plugin.yml|paper-plugin.yml,config.yml,lang/}
   bukkit   → 勾 paper 时自动带上，资源为 resources/plugin.yml
   velocity → VelocityPlugin(@Plugin) + command/ + listener/（无 resources/）
```

```kotlin
// settings.gradle.kts（按勾选条件 include；勾 paper 自动带 bukkit）
rootProject.name = "my-plugin"
include("core", "platforms:paper", "platforms:bukkit", "platforms:velocity")   // bukkit 由 paper 带入
// toolchainAutoDownload = true 时追加：
// plugins { id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0" }
```
### 8.1 每模块 Java 目标
| 模块 | Java 目标 |
|---|---|
| `paper` / `folia` | 该 MC 版本的 `java_min` |
| `bukkit` | `1.8.9`–`1.16.x` → **8**；之后取该版本 `java_min` |
| `velocity` | **25** |
| `bungeecord` | 所选坐标自带的 `java`（**默认 release 线 `1.21-R0.4` → 8**；快照线 `26.1-R0.1-SNAPSHOT` → 17 为可选） |
| `sponge` | 该 `spongeapi` 线的要求（`8.2.1` → 8 时代；`18.0.0` → 21；`20.0.0` → 25） |
| `minestom` | **25** |
| `core` | **所有已启用模块中最低的那个目标** |
**关键约束**：`core/` 必须编译在启用模块里最低的 Java 目标上——勾了 `bukkit`(8) + `paper`(21) 时 `core` 就得是 8，否则低版本平台模块根本无法依赖它。
### 8.2 core 边界
- **放**：平台无关接口（`Command` / `Config` / `Message` / `Scheduler` / `Sender` / `MenuService`）+ 纯逻辑（参数解析、消息格式化、配置模型）+ 示例业务代码。
- **禁止**：import 任何平台 API（`org.bukkit` / `io.papermc` / `com.velocitypowered` / `net.md_5`）。由质量工程里的 import 检查保证，并写进验收。
- 平台模块只做三件事：bootstrap 入口、平台 API 适配、资源文件；`core` 的 public 签名里**不出现**任何平台类型（否则老版本模块会因懒加载链接失败而崩）。
### 8.3 版本目录组织
```toml
# gradle/libs.versions.toml（只出现在根；平台模块只引用，不硬编码）
[versions]   # 由版本矩阵写入，不手改；Java 8 目标模块把 checkstyle/junit 降到 ≤9.3 / 5.14.4
paperApiVersion = "1.21.11-R0.1-SNAPSHOT"   spigotApiVersion = "1.8.8-R0.1-SNAPSHOT"
gradle = "9.8.0"  checkstyle = "13.0.0"  junit = "6.1.3"  runPaper = "3.1.0"  shadow = "9.6.1"

[libraries]
paper-api  = { module = "io.papermc.paper:paper-api", version.ref = "paperApiVersion" }
spigot-api = { module = "org.spigotmc:spigot-api",    version.ref = "spigotApiVersion" }

[plugins]
run-paper = { id = "xyz.jpenilla.run-paper", version.ref = "runPaper" }
```
- 每个启用的平台一个 `[versions]` 条目；坐标一律来自矩阵查找表，**禁止字符串拼接**；不再额外生成 `gradle.properties` 里的版本号，避免两处真相。
- 仓库统一 `mavenCentral()` + `maven("https://repo.papermc.io/repository/maven-public/")`：该源同时代理 legacy Spigot 与 `net.md-5` 快照，是"一个源覆盖两端"的关键 `[LEG §1.3]`。
- 每个平台模块自己写 `java.toolchain` 与 `options.release`；根不设固定 toolchain。
## 9. 模板内容基线
### 9.1 文件清单
标记：`✓` 恒在；`P` 勾 `paper` 时（连带 `bukkit`）；`C` 勾对应平台时；`Q` 质量工程开启（默认开）；`G` git init 开启（默认开）。

```
my-plugin/
├─ settings.gradle.kts · build.gradle.kts · gradle.properties · gradle/libs.versions.toml        ✓
├─ gradle/wrapper/gradle-wrapper.{jar,properties} · gradlew · gradlew.bat                        ✓
├─ README.md · LICENSE · CHANGELOG.md · .gitignore · .gitattributes                              ✓ / G
├─ .github/workflows/build.yml · config/checkstyle/ · .editorconfig                              Q
├─ core/  build.gradle.kts + src/main/java/<pkg>/{ExampleService,command/CommandSpec,config/PluginConfig,
│        message/Messages,permission/Permissions}.java（Permissions 随 --no-permissions 删）+ test/  Q
└─ platforms/
   paper (P)       build.gradle.kts · PaperPlugin.java · command/ · listener/ ·
                   resources/{plugin.yml|paper-plugin.yml} · config.yml · lang/ · test/ · integrationTest/
   bukkit (P 自动) 同上，资源为 resources/plugin.yml
   velocity (C)    VelocityPlugin.java(@Plugin) · command/ · listener/      # 无 resources/
   bungeecord (C)  BungeePlugin.java · command/ · listener/ · resources/plugin.yml
   folia (C)       同 paper + folia-supported: true + regionised 调度器说明
   sponge (C)      SpongePlugin.java(@Plugin/@Inject) · command/ · listener/
   minestom (C)    MinestomPlugin.java（入口 + 初始化）· command/ · listener/
```
### 9.2 每平台的"元数据 × 注册方式"（**必须成对**，混用即双重注册或命令不可见）
| 平台 | 元数据文件 | 命令声明 | 命令注册 | 监听器注册 |
|---|---|---|---|---|
| `bukkit` | `resources/plugin.yml` | `commands:` 段（description/usage/aliases/permission/permission-message） | `CommandExecutor`/`TabCompleter` 在 `onEnable` 用 `getCommand("example").setExecutor(...)` | `getServer().getPluginManager().registerEvents(...)` |
| `paper`（`plugin.yml`） | `resources/plugin.yml` | 同上 | `registerCommand("example", new ExampleCommand())`（Brigadier `BasicCommand`） | 同上 |
| `paper`（`paper-plugin.yml`） | `resources/paper-plugin.yml` | **不写 `commands:`**（Paper 插件忽略它） | `getLifecycleManager().registerEventHandler(LifecycleEvents.COMMANDS, e -> e.registrar().register("example", …))` | 同上 |
| `folia` | 同 `paper` | 同所选 metadata 格式 | 同 `paper` | 同 `paper`；adapter 用 **regionised 调度**（`Bukkit.getRegionScheduler()` / `getEntityScheduler()`），除 `runTaskAsynchronously` 外禁用全局调度假设 |
| `velocity` | **无 resources** | 无（描述符由 `@Plugin` 生成 `velocity-plugin.json` 到 jar 根） | `commandManager.metaBuilder("example").plugin(this).build()` + `register(meta, cmd)`（`SimpleCommand`） | 主类自动成为监听器；其他类 `proxy.getEventManager().register(this, listener)`。`@Subscribe` 必须 import `com.velocitypowered.api.event.Subscribe`；构造器里**不注册任何东西**，等 `ProxyInitializeEvent` |
| `bungeecord` | `resources/plugin.yml`（POJO：`name`/`main`/`version`/`author`/`depends`/`softdepends`/`description`/`libraries`） | **没有 `commands:` 字段**（会被忽略），只在代码里注册 | `getProxy().getPluginManager().registerCommand(this, new ExampleCommand())` | `getProxy().getPluginManager().registerListener(...)`；主类 `extends net.md_5.bungee.api.plugin.Plugin` |
| `sponge` | 无独立资源文件（`@Plugin` 注解 + 代码注册） | 代码内 `CommandManager` 注册，权限在 builder 上给 | 初始化阶段拿 `CommandManager`，`commandManager.command(...)` | 初始化阶段注册；**标注实验性**，具体 API 名与注册时机需逐版本实测（附录 R7） |
| `minestom` | 无（是库，不是打包服务端） | 代码内 `CommandManager` | 初始化时注册 `Command` 节点 | `GlobalEventHandler`/`EventNode` 注册；生成的是"服务端骨架"而非插件 jar；**标注实验性** |
> **paper 的陷阱**：`plugin.yml` 的 `commands:` 与 `LifecycleEvents.COMMANDS` 同时使用 = 双重注册。模板按 `--metadata` 只生成一条路径，另一个文件都不生成 `[PAPER §4.4]`。
### 9.3 示例代码的四件套（默认全开，`--no-example` 可关）
| 件 | 位置 | 内容 |
|---|---|---|
| 示例命令 | `core/command/CommandSpec.java` + 每平台一个 adapter | `/example <message>`：无参打印用法；有参把消息广播给在线玩家；权限 `myplugin.command.example` |
| 示例监听器 | 每平台 `listener/ExampleListener.java` + `core/ExampleService.java` | 玩家加入事件：读 `config.yml` 的 `welcome-message`，套语言包模板，发消息 |
| `config.yml` | **每个平台模块各一份** `resources/config.yml` | 三个键：`welcome-message`、`broadcast-prefix`、`debug`；共享的是 `core` 里的**配置模型** |
| 权限声明 | `core/permission/Permissions.java`（常量）+ 平台元数据 | `myplugin.command.example`（default `op`）、`myplugin.admin`（default `op`，children 含上者） |
`--no-permissions` 只删常量类与元数据里的 `permissions:` 段，命令内的 `permission()` 调用改为**不判断**（不是留悬空常量）。配置键名与权限节点由 `pluginName` 推导，不出现 `com.example`。
### 9.4 质量工程
| 项 | 默认 | 归属 | 说明 |
|---|---|---|---|
| Checkstyle | **开** | 根 `build.gradle.kts` + `config/checkstyle/` | `maxWarnings = 0`；`check` 依赖 `checkstyleMain`/`checkstyleTest`；`LineLength` 120 |
| 单元测试（JUnit） | **开** | 每模块 `src/test/java` | `core` 测 `ExampleService`（纯逻辑）；平台模块放一个冒烟测试 |
| `integrationTest` source set | **条件开** | 勾了服务端平台 `paper` / `folia` 的模块 | 独立 source set + `integrationTest` 任务，**必须至少有一个真跑的测试**；`check` **不**依赖它（默认 `./gradlew build` 不跑集成测试）。未勾 `paper`/`folia` 时**不生成空源集** |
| 生成物 CI（`.github/workflows/build.yml`） | **开** | 根 | `ubuntu-latest` + `windows-latest` × 生成物 toolchain 的 JDK；`./gradlew build` |
| SpotBugs | 关（`--features spotbugs`） | 根 | 只对 `core` + 现代模块开；Java 8 目标需 `4.8.6` |
| JaCoCo / 覆盖率；`release.yml`（tag → build + artifact） | 关（`--features coverage` / `--features release-ci`） | 根 | 覆盖率与 `integrationTest` 同用时需额外 aggregate 任务；release 需要 `GITHUB_TOKEN` 权限说明 |
| `spotless` / ktlint | **不用** | — | 避免与 checkstyle 职责重叠 |
### 9.5 旧版本模块上的降级（硬地板）
| 工具 | 目标 Java ≤ 8 | 目标 Java 11–16 | 目标 Java 17+ |
|---|---|---|---|
| Checkstyle | **9.3**（最后一版能跑在 Java 8 上；Gradle 9 默认 10.24.0 需 Java 11） | 10.x（默认 `toolVersion`） | 13.x / 14.x（需运行 JVM 21） |
| SpotBugs（若开） | **4.8.6** | 4.9.x / 4.10.x | 4.10.x |
| JUnit | **5.14.4**（JUnit 6 是 major 61，Java 8 加载不了） | 5.14.4 | 6.1.x |
| 运行 Gradle 的 JVM | 17–27（Gradle 9.8 硬要求，**不是** Java 8） | 同左 | 同左 |
模板把上表做成 `libs.versions.toml` 里的**按模块条件引用**（`1.8.9` 的 `platforms/bukkit` 用 checkstyle 9.3，`platforms/paper` 用 13.x，同工程并存）。注意 Checkstyle / SpotBugs 跑在 **Gradle 的 JVM** 上，不要求目标 JDK；上表是"能分析 Java 8 字节码"的下界。这些取值来自字节码探测，**以生成物首次真机构建为准**（附录 R3）。
### 9.6 README / LICENSE / CHANGELOG 与语言
| 文件 | 必含 | 语言行为 |
|---|---|---|
| `README.md` | 插件是什么；前置（目标 MC / 所需 Java / 支持平台）；`./gradlew build` + `./gradlew runServer`（老版本注明无本地测试服）；产物路径；权限表；配置表；"由 vinoa 生成"一句 | `zh` → 中文；`en` → 英文；`both` → `README.md`(中文) + `README.en.md`(英文，顶部互链) |
| `LICENSE` | Apache-2.0 英文全文 | **不翻译**；`both` 也在 README 挂中文说明链接，不生成 `LICENSE.zh` |
| `CHANGELOG.md` | Keep a Changelog 骨架 + `## [Unreleased]` + `0.1.0` 首条；语义化版本约定一句 | 跟随生成物语言；标题锚点（`Unreleased` 等）保持英文 |
| 代码注释 / Javadoc | 每个示例类头部一句用途；公共方法一行说明 | `zh`/`en` 跟随；**`both` = 中文在前、英文在后** |
| 玩家可见消息 | 每平台 `resources/lang/zh_CN.yml` / `lang/en.yml` | 与 README 同一选择，**一次决定、全工程一致** |
| `.github/workflows/*.yml`、`build.gradle.kts` 注释 | 仅必要英文注释 | **不**跟随语言 |
### 9.7 git 默认
| 项 | 默认 | 说明 |
|---|---|---|
| 是否初始化 | **是**（`--no-git` 关） | 落盘**成功之后**才 init（原子落盘 → 再 `git init`） |
| 分支名 | `main` | `git init -b main`；旧 git 时回退 `git init` + `git symbolic-ref HEAD refs/heads/main` |
| 首次提交 | 一次提交，message：`chore: scaffold <project> with vinoa` | 提交前 `git add -A`；**不**代配 `user.name`/`user.email`（缺失时跳过提交并警告）。溯源信息（模板 id / commit / 生成时间）只出现在这条 message 与 `--json` 里，**不落盘** |
| wrapper jar | **提交** `gradle/wrapper/gradle-wrapper.jar` | CI 与无网机器必须有；`.gitignore` 明确不排除它 |
| 已在 git 仓库内 | 不重复 init，只在摘要里说明 | 避免把新工程塞进内层仓库 |
| 换行符 | `.gitattributes`：`* text=auto`、`gradlew text eol=lf`、`*.bat text eol=crlf`、`*.jar binary` | Windows 原生支持的必要项 |
生成的 `.gitignore`：

```
# Gradle
.gradle/  build/
!gradle/wrapper/gradle-wrapper.jar
!gradle/wrapper/gradle-wrapper.properties
# IDE
.idea/  *.iml  .vscode/  *.ipr *.iws
# 运行 / 日志（.vinoa/ 只有 --verify 的日志会写）
run/  logs/  *.log  .vinoa/
# OS
.DS_Store  Thumbs.db
```
### 9.8 "必须替换、零残留"检查清单
生成物交付前（以及 `--verify` 前）跑同一套断言。

| # | 检查 | 判定方式 |
|---|---|---|
| 1 | 无占位符 | 全文不含 `com.example`、`example.com`、`Example-Plugin`、`__NAME__`、`{{`、`}}`、`TODO`、`FIXME`、`XXX`（`ExampleService`/`ExampleCommand` 这类**示例类名**允许存在） |
| 2 | 无脚手架痕迹 | 不含 `vinoa`（README 的"由 vinoa 生成"一句除外）、不含其它工具名、不含 "generated template" 式注释 |
| 3 | 名称一致 | `rootProject.name`、元数据 `name`、`Permissions` 常量前缀、命令名/别名、包路径互相吻合；不出现同一工程的两种拼写 |
| 4 | 主类可寻 | 元数据 `main` 指向的类文件真实存在；类名不等于 `Main` |
| 5 | 坐标可解析 | `libs.versions.toml` 里每个坐标都能在矩阵/仓库命中；不存在拼接出来的 `{mc}-R0.1-SNAPSHOT` 型坐标（尤其 1.8.9） |
| 6 | 注册成对 | 每个命令名在"元数据声明"与"代码注册"里**恰好出现一次** |
| 7 | 权限闭环 | 元数据（或代码）声明的每个节点都有对应常量；代码检查的每个节点都在元数据/代码里声明；无悬空常量 |
| 8 | 语言闭环 | `zh` 时不存在 `lang/en.yml`；`en` 时不存在 `lang/zh_CN.yml`；README/LICENSE/CHANGELOG 形态与 §9.6 一致 |
| 9 | 文件清单一致 | `--dry-run` 输出 == 实际落盘集合（含删除项：`--no-example` 不产出 adapter、`--no-permissions` 不产出常量类） |
| 10 | 工程能构建 | `--verify` 或 CI 的 `./gradlew build`（`check` 已含 checkstyle 与单测） |
| 11 | 老版本专属 | 1.8.9 工程不含 `paper-api`、不含 `paper-plugin.yml`、不含 `runServer` 配置；`spigot-api` 是 `1.8.8-R0.1-SNAPSHOT` |
| 12 | Windows 可用 | 路径与文件内容不含硬编码绝对路径；`gradlew.bat` 在列且行尾 CRLF；无 shell-only 脚本 |
## 10. 环境预检（Java）
**规则：init 不直接问 Java 版本。** 只问 MC 版本，Java 由版本矩阵推出：矩阵给出 `java_min`（必须）与 `java_recommended`（推荐，仅提示）；多平台勾选时**按模块分别列出**所需 Java（例：`bukkit` 需 8、`paper` 需 21、`velocity` 需 25）；结果出现在确认摘要里（✓/✗ 一眼可见）并进入 `--json` 的 `precheck`。

**检测顺序（固定）**：`JAVA_HOME` → PATH 上的 `java -version` → 常见安装目录扫描（Linux `/usr/lib/jvm`；Windows `Program Files\Java` 与 Adoptium；macOS `/Library/Java/JavaVirtualMachines`；SDKMAN / asdf 目录）。**不走 Gradle 进程**（慢且离线不可用）。

```
本机环境预检:
  Java 21  ✓ /usr/lib/jvm/java-21-openjdk        （platforms:paper 需要）
  Java 25  ✗ 未检测到                            （platforms:velocity 需要）

? 缺少的 Java 25 是否让 Gradle 首次构建时自动下载？ (Y/n)
  → y: 生成物 settings.gradle.kts 启用 foojay-resolver-convention
  → n: 不加插件，并提示"构建前需自行安装 Java 25，否则 ./gradlew build 会失败"
```
| 情形 | 结果 |
|---|---|
| 交互 + `y` | 写入 `org.gradle.toolchains.foojay-resolver-convention`；`precheck.auto_download.enabled = true`；退出码 `0` |
| 交互 + `n` | 不加插件 + 安装提示；**不阻塞生成**，退出码 `0` |
| 非交互 + `--no-download-jdk`（`--yes` 的保守默认） | 同 `n`，且 `warnings[]` 带 `missing_jdk` |
| 非交互 + `--verify` + JDK 缺失 | 预检阶段直接失败，`exit 75`，**不生成半个工程** |
**老版本提示**：`1.8.9` 目标需 Java 8，且该版本**起不了本地 Paper 测试服**（Paper 没有 1.8.9，Fill 返回 `version_not_found`），因此 `runTask = none`；提示必须说清，并给"用 BuildTools 或 vanilla 自建 1.8.9 服"的替代路径——**BuildTools 由用户自行调用，vinoa 不下载、不分发、不代执行** `[LEG §1.4/§4.3]`。
## 11. 失败语义、退出码与 `--json`
### 11.1 失败时用户看到什么
| 情形 | 用户看到 |
|---|---|
| 目标目录非空 | `目录已存在且非空` + 冲突项（前 10 条）+ 处置选项（`--output <other>` / `cd <dir> && vinoa init`）+ 复现命令。默认**不覆盖、不合并、不跳过**；`--dry-run` 同样按此判定 |
| 平台 × 版本组合不存在 | 交互路径下**根本选不到**；非交互路径**硬报错 + 列出可用项**，绝不静默裁剪 |
| 本机缺所需 Java | 列出"哪个模块需要哪个 Java、当前装了什么"，**询问是否让 Gradle 自动下载**；不阻塞生成 |
| 勾了 bStats 但缺 plugin id | 硬报错，提示去 bstats.org 取数字 plugin id；**绝不生成假 id** |
| 版本矩阵联网刷新失败；中途中断 / 渲染失败 | 刷新失败回退内置矩阵并警告（不阻塞生成），矩阵来源标 `builtin`；中断 / 渲染失败不留半个工程（原子落盘），说明已回滚 + 可复现命令 |
| `--verify` 构建失败 | 工程**保留**，报告失败原因与日志路径，退出码 `2`（生成成功、验证失败） |
| `-c` 配置文件语法错 | 指出行/列 + 期望的键；`exit 78` |
### 11.2 退出码
| 退出码 | 名字 | 何时 |
|---|---|---|
| `0` | `SUCCESS` | 生成成功；带 `--verify` 时构建也通过 |
| `2` | `VERIFY_FAILED` | **生成成功、验证失败**（构建失败 / 超时 / 离线取不到依赖）。该值**已冻结**；未来 `build` 子命令若需重排属于另一个 effort |
| `64` | `USAGE` | 非法 flag / 缺必填参数且无法询问（`EX_USAGE`） |
| `65` | `DATAERR` | 名称/包名非法，或 `(mc, platform)` 组合不存在（`EX_DATAERR`） |
| `73` | `CANTCREAT` | 目标目录非空且未给 `--output`，或无写权限（`EX_CANTCREAT`） |
| `74` | `IOERR` | 落盘中途失败，已回滚（`EX_IOERR`） |
| `75` | `TEMPFAIL` | JDK 缺失且用户拒绝自动下载、且**继续生成**不允许时；或渲染依赖的临时资源不可用（`EX_TEMPFAIL`） |
| `78` | `CONFIG` | 内置模板/矩阵损坏，或 `-c` 配置文件语法错（`EX_CONFIG`） |
| `130` | `INTERRUPTED` | Ctrl-C / SIGINT：原子落盘已回滚，POSIX 128+2 |
### 11.3 两个负向验收样例
```
$ vinoa init my-plugin -m 1.8.9 --platform paper -y
✗ 平台与版本组合不存在：paper × 1.8.9
  原因: Paper 最早的构建是 1.8.8；1.8.9 没有 Paper/paper-api 制品
  1.8.9 可用平台: bukkit, sponge
  该平台可用版本: 1.9.4, 1.10.2, 1.11.2, 1.12.2, … , 26.2
  复现: vinoa versions --matrix --mc 1.8.9                                      → exit 65

$ vinoa init my-plugin -m 1.21.11 --platform paper -y      # my-plugin/ 已有 src/、pom.xml
✗ 目标目录已存在且非空: /root/projects/my-plugin
  冲突项（前 10 条）: pom.xml, src/, .idea/
  处置选项: --output <other-dir> 换空目录 | cd <dir> && vinoa init 确认"就地初始化"（仍逐文件询问）
  复现: vinoa init my-plugin -m 1.21.11 --platform paper --output /root/projects/my-plugin-2 -y
                                                                                → exit 73
```
**三个硬性要求**：说清发生了什么、给出下一步动作、给出可复现命令。禁止"操作失败"式文案。
### 11.4 `--verify` 精确语义（opt-in）
| 项 | 约定 |
|---|---|
| 触发与执行位置 | 仅当显式传 `--verify`（向导 ④ 页不出现；`--yes` 不隐含开启）；命令在生成物根目录执行（`cwd = <目标目录>`），不改环境变量、不清理既有 `build/` |
| 命令 | Linux/macOS `./gradlew build`；Windows `gradlew.bat build`（走 `cmd`，不做 WSL 假设） |
| 附加 flag | `--offline`（默认开，除非 `--verify-online`）；`--no-daemon`；`--console=plain`；`--stacktrace` 只写日志不进 stdout |
| 环境 | 强制继承本机 JDK；未装所需 JDK 时**不**自动下载（下载只由生成物里的 foojay 决定，且 `--yes` 下保守为关） |
| 超时 | 默认 **600 s**（`VINOA_VERIFY_TIMEOUT` 可覆盖，硬上限 1800 s）；超时 → `reason="timeout"`，杀进程树 |
| 离线边界 | **用户机器默认离线**；**CI 的保证行允许先 warm-up（联网）再离线构建**。"可复现"只约束生成阶段，不约束构建阶段 |
| 失败分类 | `network_unavailable` / `build_failed` / `timeout`（退出码都是 `2`，`reason` 区分） |
| 日志 | 永远落盘 `<目标目录>/.vinoa/verify/verify-<UTC时间戳>.log`；stdout 只打印**最后 20 行**（`--verify-tail <N>`，上限 200）+ 路径 |
| 失败后 | 工程**保留**、不写状态文件、不做任何回滚；提示"可复现命令" |
人可读失败报告的顺序固定：**先给动作（复现命令 + 日志路径），再给日志尾部**。

```
✗ 生成成功，验证失败（exit 2）
  工程: /root/projects/my-plugin      命令: ./gradlew build --offline --no-daemon
  退出码: 1      原因: build_failed
  日志: .vinoa/verify/verify-20260925T102233Z.log
  复现: cd /root/projects/my-plugin && ./gradlew build

  --- 日志尾部 20 行 ---
  > Task :platforms:paper:compileJava FAILED
  error: cannot find symbol  …
```
### 11.5 `--json` 形状（所有路径同构）
stdout 恰好一个 JSON 文档；人类可读文本走 stderr。

```json
{
  "schema": "vinoa.init/v1", "vinoa": "0.1.0", "ok": false, "command": "init", "phase": "verify",
  "generated_at": "2026-09-25T10:22:33Z", "status": "verify_failed", "exit_code": 2,
  "project": {
    "name": "my-plugin", "path": "/root/projects/my-plugin", "package": "com.example.myplugin",
    "lang": "zh", "mc": "1.21.11", "java_target": 21, "gradle": "9.8.0",
    "platforms": ["paper", "bukkit"], "metadata": "plugin.yml", "features": [],
    "quality_engineering": true, "git": {"initialized": true, "branch": "main"}
  },
  "precheck": {
    "jdk": [
      {"module": "platforms:paper",  "required": 21, "found": true,  "path": "/usr/lib/jvm/java-21-openjdk"},
      {"module": "platforms:bukkit", "required": 8,  "found": false, "path": null}
    ],
    "auto_download": {"enabled": true, "plugin": "org.gradle.toolchains.foojay-resolver-convention"}
  },
  "matrix": {"source": "builtin", "generated_at": "2026-09-25"},
  "template_source": {"kind": "embedded", "id": "vinoa/paper-gradle-v1", "version": "1.0.0"},
  "files": {"count": 41, "written": 41},
  "verify": {
    "ran": true, "ok": false, "reason": "build_failed",
    "command": "./gradlew build --offline --no-daemon", "timeout_s": 600, "duration_ms": 91234,
    "log_path": ".vinoa/verify/verify-20260925T102233Z.log", "tail": ["…最后 20 行…"]
  },
  "warnings": [],
  "errors": [{
    "code": "verify_failed", "phase": "verify", "severity": "error",
    "message": "Gradle build failed (exit 1)", "hint": "cd /root/projects/my-plugin && ./gradlew build"
  }]
}
```
契约要点：

- 每个错误必有**稳定 ASCII 码**（`命名空间.小写_下划线`）与 `phase` ∈ `input|validate|matrix|precheck|plan|render|verify|write|post`；`line`/`column` 为 1 基，**只有错误发生在文件内时才有**；`message` 跟随 `--ui-lang`，`code` 不跟随语言。
- **校验阶段聚合报错**（一次列全所有非法字段）；**渲染阶段首错即停**（后续多半是级联噪声），且不写盘。
- 机器判定只看 `exit_code`（与人类路径**同一套**）；`status` / `reason` 只做分类，便于 agent 决定重试策略。
- 未跑 `--verify` 时 `verify = {"ran": false, "ok": null, "reason": null}`；`--dry-run` 时 `files` 变为 `planned`（含 `target` / `render` / `bytes` / `sha256`）加 `skipped[]`（带 `when` 原因），`git` 与 `verify` 恒为 `ran:false`。
- `SOURCE_DATE_EPOCH` 一旦影响产物（例如写入年份），必须在 `--json` 里标注；默认路径不设置 `copyrightYear`，因此不产生差异。中断时 `--json` 仍要能完整输出（走 stderr 旁路）。
### 11.6 错误码表
`template.not_found` / `template.manifest_invalid` / `template.manifest_unknown_key` / `template.version_too_old`（清单缺失、TOML 语法错带行列、未知键、`min_cli_version` 过高）；`template.undefined_variable` / `template.variable_mismatch` / `template.bad_path_var` / `template.bad_target`（未定义变量、清单与模板声明不对称 A5、路径用了不可入路径的变量、目标路径非法或逃逸）；`template.i18n_missing_key` / `template.license_unsupported` / `template.too_large`；`var.invalid_value`（名称、包名、`mcVersion`×平台、`bstatsPluginId` 等校验失败）；`render.*`（`syntax_error` / `leftover_placeholder` / `stale_default` / `main_class_mismatch` / `module_set_mismatch` / `assertion_failed`，即 §7.10 的自检）；`write.exists` / `write.io`（目标目录非空 / 落盘失败，原子回滚无残留）；`verify_failed`（`--verify` 构建失败，`exit 2`）。

渲染错误的人可读形态：

```
错误[render.leftover_placeholder] 渲染后仍有未替换占位符
  templates/platforms/_p_/build.gradle.kts.jinja:37:18
  → platforms/paper/build.gradle.kts
  hint: 变量 cap_placeholderapi 未在 vinoa-template.toml 的 [conditions] 中声明
```
## 12. 验收与承诺矩阵
### 12.1 等级归属（已裁定）
- **A**（真跑 `./gradlew build`）：`paper` / `bukkit` / `velocity` / `bungeecord`；**B**（渲染 + 依赖解析 + 静态断言，不跑完整构建）：`folia` / `sponge` / `minestom`（它们已标 experimental，承诺级别与标注一致）；`paper-plugin` 元数据全部变体；`--lang en|both`；toolchain 覆盖值；`--no-*` 之外的其它变体。
### 12.2 A 级保证矩阵
统一条件：元数据 `plugin.yml`、示例代码 + 权限声明 + 质量工程全开（向导默认）、Gradle wrapper 9.8.0、**依赖先 warm-up 再离线构建**。第 4 列 = 生成物默认 toolchain（一律 `java_min`，代理端按 §6.3.4）；第 5 列 = vinoa CI 实际执行 build 的 JDK。

| # | MC 版本 | 平台（连带模块） | 生成物 toolchain | CI 的 JDK | 依赖坐标（矩阵查表，非拼接） |
|---|---|---|---|---|---|
| A1 | 1.8.9 | `bukkit`（单独勾选） | **8** | **21**（用 `options.release = 8`） | `org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT` |
| A2 | 1.12.2 | `paper` + `bukkit` | 8 | 21 | `com.destroystokyo.paper:paper-api:1.12.2-R0.1-SNAPSHOT` |
| A3 | 1.16.5 | `paper` + `bukkit` | **8** | 21 | `com.destroystokyo.paper:paper-api:1.16.5-R0.1-SNAPSHOT` |
| A4 | 1.21.11 | `paper` + `bukkit` | 21 | 21 | `io.papermc.paper:paper-api:1.21.11-R0.1-SNAPSHOT` |
| A5 | 26.2 | `paper` + `bukkit` | 25 | **25** | `io.papermc.paper:paper-api:26.2.build.+`（钉 `26.2.build.129-stable`） |
| A6 | —（代理端不锁 MC） | `velocity` | 25 | 25 | `com.velocitypowered:velocity-api:4.2.0` |
| A7 | —（代理端不锁 MC） | `bungeecord` | **8** | 21 | `net.md-5:bungeecord-api:1.21-R0.4` |
要点：

1. **1.8.9 ≠ Paper 1.8.9**：Paper 从 1.9.4 起；1.8.9 只能走 `bukkit` 模块且用 `spigot-api:1.8.8`（没有 `1.8.9` 制品）。**A3 的 toolchain 是 8**（`java_min`，不是推荐值 16）；`java_recommended` 只用于向导提示。
2. **CI 的 JDK ≠ 生成物 toolchain**：Gradle 9.8 要求 JVM 17–27，所以 Java 8 目标用 JDK 21 跑 Gradle + `options.release = 8`，该路径已实测（Gradle 9.7.1 / JDK 21 → major 52）`[LEG §2.3]`；**A7 取 release 线 `1.21-R0.4`（Java 8）**——不拿快照当生成物默认：快照会变甚至消失，而且 Java 17 目标白丢老代理端兼容。快照线 `26.1-R0.1-SNAPSHOT`（Java 17）进 B 级。

**同为 A 级的降级子行**：`--no-quality` 与 `--no-example` 在 A1（1.8.9 bukkit）上各跑一条，证明"关掉后仍能构建"。
**明确为 B 级**：`bungeecord` 快照线（Java 17）变体、`1.16.5` 的 toolchain=11/16 变体、A1–A7 的 `--metadata paper-plugin` 变体、A1–A7 的 `--lang en|both`、`folia` / `sponge` / `minestom` 全部组合（各取 26.2 为锚点；`minestom` 另有 `1.21.11` / `26.1.1` / `26.1.2` 三个可选项）。
### 12.3 vinoa 自己的 CI
`.github/workflows/acceptance.yml`（**vinoa 仓库**的工作流，不是生成物的）。

| 维度 | 取值 |
|---|---|
| Runner 与 Gradle 运行 JDK | `ubuntu-latest`（必跑）、`windows-latest`（**必跑**，Windows 是官方支持环境）、`macos-latest`（仅 `--dry-run` + 渲染检查，不跑 Gradle）；JDK `17` / `21` / `25`（Temurin，17 覆盖 Gradle 下界） |
| 生成物 toolchain / 元数据 / 语言 | 由矩阵推导，额外扫 `8 / 11 / 16 / 21 / 25`；`plugin.yml`（保证行默认）+ `paper-plugin` 现代行至少 2 条；`zh`（默认）+ 每 PR 至少 1 条 `en`、1 条 `both` |
| 真跑 Gradle 的行 | §12.2 的 A1–A7 × `ubuntu` = 7 个 job（另加 2 条 A 级降级子行）；`windows` 抽 A1 / A4 / A5 / A7 共 4 个 job |
| 其余组合 | `--dry-run` + 渲染 + `libs.versions.toml` 解析 + 静态断言（不跑 Gradle） |
时间预算（单 job 硬超时，超时即失败）：vinoa 自身 `cargo build` + 单测 **3 min**；生成 + `--verify`（paper/bukkit 行，含配置阶段）**6 min**；生成 + `--verify`（proxy 行）**4 min**；整个 workflow（含并行 fan-out）**20 min**。

实现要点：`actions/setup-java` 装 JDK；生成物内的 foojay 在 CI 里**不下网**（`-Porg.gradle.java.installations.auto-download=false`），逼出"本机 JDK 必须在场"的真实路径；依赖用一次 warm-up 任务灌进 `~/.gradle`，然后**离线**跑保证行（保证行本身不允许联网，否则"保证"依赖上游可用性）；每个 job 上传 `vinoa-report.json`、生成物 `build/reports/**`、失败时 `*.log` 尾 200 行；夜间 cron（非 PR 阻塞）额外跑"全矩阵 B 级"和 `versions --refresh` 后再跑一次，用来**提前发现**上游坐标漂移。
### 12.4 其余负向项（点名验收）
| 负向场景 | 期望 |
|---|---|
| `versions --refresh` 联网失败 | 回退内置矩阵 + 警告，**不阻塞**；矩阵来源标 `builtin` |
| 落盘中途 SIGINT | 原子回滚，无残留目录，`exit 130`；`--json` 仍完整输出（stderr 旁路） |
| `-c config.toml` 语法错 | 指出行/列 + 期望的键；`exit 78` |
| Windows 下路径含空格 / 中文 | 生成、`--verify`、`git init` 全部成功（原生路径与编码，不回退 WSL）——**当前未在真机实测**（附录 R1） |
| `--dry-run` 于非空目录 | 与真实运行**同**判定、同退出码 |
### 12.5 我们**不**承诺
- **不承诺** B 级任何组合能构建通过；也不承诺它们是稳定接口（可能随上游坐标变化失效，只保证"要么渲染成功、要么硬报错"）；生成物格式仍会在 1.x 内演进。
- **不承诺**任何环境都能构建：首次构建需要网络（Gradle 发行包、平台 API、插件），离线只在缓存预热后成立；**1.8.9 起不了本地测试服**（Paper 无 1.8.9，Fill 返回 `version_not_found`）。
- **不承诺**旧版构建路径的长期寿命：JDK 25 上 `--release 8` 已带 "obsolete" 警告并将被移除，届时 Java 8 目标会先在 CI 里变红再谈支持；也不承诺跨平台交叉构建或 CI 在 Windows 覆盖全部 A 行（抽样 4 行）。
- **不承诺** NMS/`paperweight`、`libraries:` 在 <1.16.5、`api-version` 在 <1.13 等现代能力的降级等价物（矩阵能力位已标出）；**不承诺** `--verify` 的耗时下界（超时是设计内失败模式）。
## 13. 许可证与 clean-room 归属
- **vinoa 自身**与**生成物**都用 **Apache-2.0**。模板集只内嵌 Apache-2.0 全文；其它 SPDX id → `template.license_unsupported`。生成的 `LICENSE` 是 Apache-2.0 **英文全文，不翻译**；`lang = both` 时在 README 挂中文说明链接，不生成 `LICENSE.zh`。
- **默认不写任何年份**：Apache-2.0 文本本身不需要年份；**源文件不生成版权头**。`--author` 给定时可写入一条不带年份的版权行；未给定时保留 LICENSE 模板中的占位说明。只有显式设置 `SOURCE_DATE_EPOCH` 时才允许取 `copyrightYear`，且该差异必须出现在 `--json` 里。
- **不落盘任何溯源文件**（无 `.vinoa.toml`）：溯源信息只出现在 `--json` 与首次提交的 message 里；`.vinoa/` 目录只用于 `--verify` 日志。
- **clean-room**：参考的是两个社区模板的**结构与工程纪律**（"必须替换、零残留"这一设计意图），**不复制任何代码正文/文本**；参考过的项目只用于确定"哪些 key、哪些检查项存在"。平台细节的一手来源是上游官方文档与**制品本身**（POM / JAR / maven-metadata / Fill API），来源清单见 `docs/research/`。生成的 README 必须注明参考来源；"由 vinoa 生成"一句是唯一允许出现的 `vinoa` 字样。
## 14. 不在范围内
| 项 | 说明 |
|---|---|
| `build` / `run` / 发布子命令；已生成工程的更新 / 同步 | 本轮只留接口口子；不写回已存在的工程，不提供 `update`/`sync` |
| 模板覆盖层 / 文件级合并；外部模板签名校验与私有仓库凭据透传 | 只支持"整套替换"（§7.9）；后两项 v1 明确不做并在 README 说明 |
| NMS / `paperweight` 默认开启 | 默认 `false`；26.1 起不生成任何 `reobfJar` 接线 |
| 基岩版（Nukkit / PowerNukkitX）、Waterfall | 基岩版版本轴不同；Waterfall 已 EOL |
| 交叉编译（Windows 上构建 Linux 产物等）；IDE 工程文件 | 生成物只保证 Gradle 命令行可构建 |
| 除 `zh`/`en`/`both` 之外的生成物语言；生成物 CI 在 Windows 覆盖全部 A 行 | 语言包只有两套；Windows 抽样 4 行 |
## 附录：已知风险
以下均为**未解决**项。规格不为它们发明答案；实现遇到时按"处置"列执行。

| # | 风险 | 影响 | 处置 |
|---|---|---|---|
| **R1** | **Windows 非 ASCII 路径与跨平台行为未实测**：`{{packagePath}}` 作为真实目录名在 Windows 大小写不敏感 + `core.protectNTFS` + git 的交互、CRLF、`gradlew` 权限位、`gradlew.bat` 行为只在 Linux 验证过；`processResources` 的 `\$` 转义（含 `$` 的 `pluginDescription`/`authors`）也只来自 SimpleTemplateEngine 的通用行为 | Windows 是官方支持环境，这是 §12.3 "Windows 必跑"的直接风险 | 实现阶段补 Windows 真机用例：生成路径（含空格/中文）+ CRLF + `gradlew.bat` + `$` 转义，并在 Windows CI 上验证 |
| **R2** | **minijinja 细节待验**：`{% raw %}` 可用性、`UndefinedBehavior::Strict` 与 `render_named_str` 组合下的错误位置精度、空列表 `{% for %}` 行为 | §7.2 的措辞可能需微调（语义不变） | 实现前先做 20 行最小实验确认，再写死 §7.2 的措辞；结论不改变对用户可见行为 |
| **R3** | **质量工具降级地板待真机构建**：checkstyle 9.3 / spotbugs 4.8.6 / junit 5.14.4 的 Java 8 上界来自字节码探测；同一份 `checkstyle.xml` 同时喂 9.3 与 13.x（`LineLength`/`ImportOrder` 等在 10/11/13 间有弃用变更）未实跑 | §9.5 的版本可能在首次真机构建时失败 | 以生成物 `--verify` 的**第一次真机构建**为准替换 §9.5 的取值；同一份 config 针对两个 checkstyle 主版本各跑一次 |
| **R4** | **上游漂移**：平台坐标、Java 地板、Gradle 与插件版本、BungeeCord/Sponge/Minestom 的发布节奏都不由 vinoa 控制；`26.3` 目前只有 `ALPHA`，Sponge API 18/19/20/21 仍是快照/RC 线 | A 级保证行可能因上游变化变红；B 级组合可能静默失效 | 夜间 cron 跑 `versions --refresh` 后再跑一次全矩阵以提前发现；坐标只从矩阵出，漂移时改数据不改代码 |
| **R12** | **`folia` + `--metadata paper-plugin` 无法声明 `folia-supported`**（上游 `paper-plugin.yml` 无该字段，硬塞未知字段可能被拒） | 该组合下 Folia 可能拒绝加载插件（B 级组合） | 在 plan 里给 `warnings`；若上游未来支持该字段则改模板 |
| **R13** | **`commands.aliases:` 未生成**：模板无法从 `commandName` 安全派生短别名（受限表达式语言禁止切片/算术） | 用户可能期待示例命令带别名 | engine 提供 `commandAlias` 变量后补齐 |
| **R14** | **`paper-plugin.yml` 的 `api-version` 固定为 `'1.19'`**：这是最宽松合法值，与用户目标 MC 版本无关 | 看似“没跟进目标版本”，实为有意为之（最大化可加载范围） | 矩阵提供 api-version 数据后改为变量 |
| **R15** | **可选模块模板尚未全部实现**（`sqlite`/`bstats`/`update-check`/`placeholderapi`/`gui`、`integrationTest`、`spotbugs`/`coverage`/`release-ci`、`libraries:` 块）——task-7 进行中 | `--features X` 目前可能静默不生成，用户以为已有 | engine 在清单声明“已实现 feature”，`init` 对未实现/缺坐标的 feature **硬报错**；全部实现后删除本条 |
| R5 | 外部 git 模板的信任边界只有四项（无 hook 执行、记录 commit、尺寸上限、二次确认）；**不做签名校验，不透传私有仓库凭据** | 使用第三方模板有供应链风险 | 已在 §7.9 与 README 明说；不新增机制（v1 决策） |
| R6 | `[thirdparty]` 中 bStats / sqlite-jdbc / GUI 库的坐标值尚未做 primary-source 核对（只有 `placeholderapi` 有草稿来源） | 勾选这三个特性时可能解析失败 | 实现时按 primary source 补全矩阵数据；矩阵是唯一写入点 |
| R7 | Sponge / Minestom 的示例代码 API 名与注册时机随版本跳变（SpongeAPI 4.2.0 → 20.0.0），本轮只给出形状 | B 级组合上的示例可能编译不过 | 逐版本实测后再定稿示例；B 级不承诺构建（§12.1） |
| R8 | `paper-plugin.yml` 的 `api-version` 取值（`'1.19'` vs 目标 MC 版本）草稿未拍；该文件的未知键行为（Configurate 是否严格）未验证 | 取 `1.19` 是按"最宽松合法值"原则的推导 | 首次真机构建时确认；若上游行为不同，改矩阵/模板数据而非契约 |
| R9 | 矩阵内部表示粒度（保留全量 66 行 vs 断点区间 + 例外表）未定；两者查询结果一致 | 只影响二进制体积与实现复杂度，不影响对外契约 | 由实现选择；§6.1/§6.2 已给出两种形式的等价数据；`--json` 不得暴露内部表示 |
| R10 | 保证行"离线"的可行性：`spigot-api:1.8.8` 的传递依赖 `net.md-5:bungeecord-chat:1.8-SNAPSHOT` 只能从 Spigot `public` 组或 `repo.papermc.io` 取 | CI warm-up 若不稳定命中这两个源，A1 会不稳定 | 生成物统一用 `repo.papermc.io/maven-public`（已实测同时覆盖 legacy Spigot 与 `net.md-5` 快照）；CI 用 artifacts 与重试兜底 |
| R11 | `--verify` 时间预算（6 min/行含冷启动配置阶段）基于估计，尚无第一轮 CI 数据；SnakeYAML 对未加引号 `version:` 的标量强转行为也未实测 | 预算可能临时调高；强转风险已在契约层规避 | 用第一轮 CI 实测收紧预算（超时本身就是设计内失败模式）；保持 `version: "${version}"` 强制加引号 |
