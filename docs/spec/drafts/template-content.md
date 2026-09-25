# 模板内容与质量工程基线（草稿 v1）

> 供 ticket [#12 模板内容与质量工程基线](https://github.com/NmouZh/vinoa/issues/12) 使用。
> 目录口径见 [project-layout.md](project-layout.md)；参数来源见 [version-matrix.md](version-matrix.md)；平台细节见两篇 research（[paper](../../research/paper-plugin-project-shape.md) / [proxy](../../research/proxy-plugin-project-shape.md)）与 [legacy 可行性](../../research/legacy-mc-build-feasibility.md)。
> **clean-room**：全部内容按公开 schema 自写；参考过的项目只用于确定"哪些 key / 哪些检查项存在"，不复制其文本（含 CrimsonWarpedcraft `docs/customization.md`，仅借其"必须替换、零残留"的思路，见 §6）。

## 1. 文件清单（`vinoa init my-plugin -m 1.21.11 --platform paper --platform velocity`）

`✓` 恒在；`P` 勾选 paper 时；`C` 勾选对应平台时；`Q` 质量工程开启时（默认开）；`G` git init 开启时（默认开）。

```
my-plugin/
├─ settings.gradle.kts            ✓  # 只 include 已选平台；含 rootProject.name 与 foojay（按预检答案）
├─ build.gradle.kts               ✓  # 根：插件版本统一、子模块公共配置（不写平台依赖）
├─ gradle.properties              ✓  # org.gradle.jvmargs / caching / parallel / encoding
├─ gradle/libs.versions.toml      ✓  # 版本目录（§2）
├─ gradlew · gradlew.bat          ✓  # wrapper 脚本（§5）
├─ gradle/wrapper/gradle-wrapper.{jar,properties}  ✓  # 9.8.0（§5）
├─ .gitignore · .gitattributes    G  # 见 §5
├─ README.md                      ✓  # §4；双语时 = README.md(zh) + README.en.md
├─ LICENSE                        ✓  # Apache-2.0 全文（英文，不可改语言）
├─ CHANGELOG.md                   ✓  # §4
├─ .github/workflows/build.yml    Q  # 生成物 CI（§3.4）
├─ .github/workflows/release.yml  Q  # 打 tag 时跑 build（可选默认关）
├─ config/checkstyle/checkstyle.xml   Q
├─ config/checkstyle/suppressions.xml Q
├─ .editorconfig · CONTRIBUTING.md    Q  # 可有可无；建议 Q
├─ core/
│  ├─ build.gradle.kts            ✓  # toolchain=min(平台)/release=8；不 import 任何平台 API
│  └─ src/
│     ├─ main/java/<pkg>/
│     │  ├─ ExampleService.java             ✓  # 共享业务逻辑（不引用平台类型）
│     │  ├─ command/CommandSpec.java        ✓  # 平台无关的命令描述
│     │  ├─ config/PluginConfig.java        ✓  # 配置模型 + 加载接口
│     │  ├─ message/Messages.java           ✓  # 本地化取值
│     │  └─ permission/Permissions.java     ✓  # 权限常量类（`--no-permissions` 时删）
│     ├─ main/resources/config.yml          ✓  # 唯一 config.yml（§2 示例）
│     ├─ main/resources/messages_zh.yml     ✓  # 按生成物语言裁剪
│     ├─ main/resources/messages_en.yml     ✓
│     └─ test/java/<pkg>/ExampleServiceTest.java   Q
└─ platforms/
   ├─ paper/    (P)  build.gradle.kts · PaperPlugin.java · command/ExampleCommand.java
   │                 listener/ExampleListener.java · resources/{plugin.yml|paper-plugin.yml} · test/…
   ├─ bukkit/   (P 自动带上)  build.gradle.kts · BukkitPlugin.java · command/ExampleCommand.java
   │                 listener/ExampleListener.java · resources/plugin.yml · test/…
   ├─ velocity/ (C)  VelocityPlugin.java（`@Plugin`）· command/ExampleCommand.java
   │                 listener/ExampleListener.java（**无 resources/**，描述符由注解处理器生成）
   ├─ bungeecord/(C) BungeePlugin.java · command/ExampleCommand.java · listener/ExampleListener.java
   │                 resources/plugin.yml
   ├─ folia/    (C)  同 paper + `folia-supported: true` + regionised 调度器说明
   ├─ sponge/   (C)  SpongePlugin.java（`@Plugin`/`@Inject`）· command/ExampleCommand.java · listener/…
   └─ minestom/ (C)  MinestomPlugin.java（入口 + 初始化）· command/ExampleCommand.java · listener/…
```

规则：模块路径用 `include("core", "platforms:paper", …)`；包内保留 `<包名>/<平台>/` 子包，不平铺；`core` 的 public 签名里**不出现**任何平台类型（否则老版本模块会因懒加载链接失败而崩）。

## 2. `gradle/libs.versions.toml` 与根构建

```toml
[versions]
mc = "1.21.11"          # 由向导的 MC 版本写入，唯一事实来源
java = "21"             # 矩阵 java_recommended，缺省回落 java_min
gradle = "9.8.0"
checkstyle = "13.0.0"   # Q；Java 8 目标模块下降级见 §3.1
junit = "6.1.3"         # Q；Java ≤16 目标模块降级见 §3.2
runPaper = "3.1.0"
shadow = "9.6.1"

[libraries]
paper-api = { module = "io.papermc.paper:paper-api", version.ref = "..." }   # 坐标由坐标表映射，不拼接
spigot-api = { module = "org.spigotmc:spigot-api", version = "1.8.8-R0.1-SNAPSHOT" }  # 1.8.9 专用
velocity-api = { module = "com.velocitypowered:velocity-api", version = "4.2.0" }
bungeecord-api = { module = "net.md-5:bungeecord-api", version = "26.1-R0.1-SNAPSHOT" }

[plugins]
run-paper = { id = "xyz.jpenilla.run-paper", version.ref = "runPaper" }
```

- 版本目录**只出现在根**；平台模块用 `project(":core")` + `libs.*`，不各自写死坐标。
- `settings.gradle.kts` 里 `pluginManagement { repositories { gradlePluginPortal(); mavenCentral() } }`；仓库统一 `mavenCentral()` + `maven("https://repo.papermc.io/repository/maven-public/")`（该源同时代理 legacy Spigot 与 `net.md-5` 快照，是"一个源覆盖两端"的关键）。
- **禁止**字符串拼 MC 版本成坐标：`1.8.9 → spigot-api:1.8.8` 这类映射必须走显式查找表（否则 404）。
- 每个平台模块自己写 `java.toolchain` 与 `options.release`，根不设固定 toolchain（Gradle 运行 JVM 由用户环境决定，生成物只约束编译目标）。
- `processResources` 用 `expand(mapOf("version" to project.version, "apiVersion" to …))` 注入版本号（`plugin.yml` 里 `version: '${version}'` 必须加引号，避免 SnakeYAML 标量强转）。
- `paper-plugin.yml` 是 `@Required` 语义，**必须**写 `api-version`；`plugin.yml` 允许省略但会以 legacy 模式加载并警告，故默认写 `api-version: '1.13'`（兼容下界，老版本忽略）。

## 3. 示例代码与质量工程

### 3.1 示例代码的四件套（默认全开，`--no-example` 可关）

| 件 | 位置 | 内容 |
|---|---|---|
| 示例命令 | `core/command/CommandSpec.java` + 每平台一个 adapter | `/example <message>`：无参打印用法；有参把消息广播给在线玩家；权限 `myplugin.command.example` |
| 示例监听器 | 每平台 `listener/ExampleListener.java` + `core/ExampleService.java` | 玩家加入事件：读 `config.yml` 的 `welcome-message`，套 `messages_*.yml` 模板，发消息 |
| `config.yml` | `core/src/main/resources/config.yml` | 三个键：`welcome-message: "&a欢迎, {player}!"`、`broadcast-prefix: "[Example]"`、`debug: false` |
| 权限声明 | `core/permission/Permissions.java`（常量）+ 平台元数据 | `myplugin.command.example`（default `op`）、`myplugin.admin`（default `op`，children 含上者） |

`--no-permissions` 只删常量类与元数据里的 `permissions:` 段，命令内的 `permission()` 调用改为**不判断**（不是留悬空常量）。配置键名与权限节点由 `pluginName` 推导，不出现 `com.example`。

### 3.2 每个平台的"元数据 × 注册方式"（**必须成对**，混用即双重注册或命令不可见）

| 平台 | 元数据文件 | 命令声明 | 命令注册 | 监听器注册 | 备注 |
|---|---|---|---|---|---|
| bukkit | `resources/plugin.yml` | `commands:` 段（description/usage/aliases/permission/permission-message） | `CommandExecutor`/`TabCompleter` 实现在 `onEnable` 用 `getCommand("example").setExecutor(...)` | `getServer().getPluginManager().registerEvents(...)` | 元数据与代码**都要**有 |
| paper（`plugin.yml`） | `resources/plugin.yml` | 同上（`commands:`） | `registerCommand("example", new ExampleCommand())`（Brigadier `BasicCommand`） | 同上 | 与下一行**互斥**，靠 `--metadata` 选择 |
| paper（`paper-plugin.yml`） | `resources/paper-plugin.yml` | **不写 `commands:`**（Paper 插件忽略它） | `getLifecycleManager().registerEventHandler(LifecycleEvents.COMMANDS, e -> e.registrar().register("example", …))` | 同上 | 该文件 `api-version` 为必填；建议放在 `onEnable`，不用 bootstrapper |
| folia | 同 paper | 同所选 metadata 格式 | 同 paper | 同 paper | **adapter 用 regionised 调度**：`Bukkit.getRegionScheduler()` / `getEntityScheduler()`，禁用 `runTaskAsynchronously` 之外的全局调度假设 |
| velocity | **无 resources** | 无（描述符由 `@Plugin` 生成 `velocity-plugin.json` 到 jar 根） | `commandManager.metaBuilder("example").plugin(this).build()` + `register(meta, cmd)`（`SimpleCommand`） | 主类自动成为监听器；其他类 `proxy.getEventManager().register(this, listener)` | `@Subscribe` 必须 import `com.velocitypowered.api.event.Subscribe`；构造器里**不注册任何东西**，等 `ProxyInitializeEvent` |
| bungeecord | `resources/plugin.yml`（POJO：`name`/`main`/`version`/`author`/`depends`/`softdepends`/`description`/`libraries`） | **没有 `commands:` 字段**（会被忽略），只在代码里注册 | `getProxy().getPluginManager().registerCommand(this, new ExampleCommand())` | `getProxy().getPluginManager().registerListener(...)` | 主类 `extends net.md_5.bungee.api.plugin.Plugin`；YAML 里 `main` 指向它 |
| sponge | 无独立资源文件（`@Plugin` 注解 + 代码注册） | 代码内 `CommandManager` 注册，权限在 builder 上给 | 在插件构建/初始化阶段拿 `CommandManager`，`commandManager.command(...)` | 事件监听器同样在初始化阶段注册 | API 线由矩阵的 `sponge_api` 决定；**标注实验性** |
| minestom | 无（库，不是打包服务端） | 代码内 `CommandManager` | 初始化时注册 `Command` 节点 | `GlobalEventHandler`/`EventNode` 注册 | 生成的是"服务端骨架"而非插件 jar；**标注实验性** |

> **paper 的陷阱**：`plugin.yml` 的 `commands:` 与 `LifecycleEvents.COMMANDS` 同时使用 = 双重注册。模板按 `--metadata` 只生成一条路径，另一个文件都不生成（`--metadata paper-plugin` 时不产出 `plugin.yml`，反之亦然）。

### 3.3 质量工程：默认开 / 可选

| 项 | 默认 | 归属 | 说明 |
|---|---|---|---|
| Checkstyle | **开** | 根 `build.gradle.kts` + `config/checkstyle/` | `maxWarnings = 0`；`check` 依赖 `checkstyleMain`/`checkstyleTest`；配置文件自带中文注释行宽放宽（`LineLength` 120） |
| 单元测试（JUnit） | **开** | 每模块 `src/test/java` | `core` 测 `ExampleService`（纯逻辑，无平台依赖）；平台模块默认只放一个冒烟测试 |
| `integrationTest` source set | **开**（骨架、默认无用例） | 每模块 | 独立 source set + `integrationTest` 任务，`check` **不**依赖它（默认 `./gradlew build` 不跑集成测试） |
| 生成物 CI（`.github/workflows/build.yml`） | **开** | 根 | `ubuntu-latest` + `windows-latest` × 生成物 toolchain 的 JDK；`./gradlew build` |
| SpotBugs | **关**（`--features spotbugs`） | 根 | 只对 `core` + 现代模块开；Java 8 目标需 `4.8.6` |
| JaCoCo / 覆盖率 | **关**（`--features coverage`） | 根 | 与 `integrationTest` 一起用时需额外 aggregate 任务 |
| `release.yml`（tag → build + artifact） | **关**（`--features release-ci`） | 根 | 需要 `GITHUB_TOKEN` 权限说明 |
| `spotless` / ktlint | **不用** | — | 避免与 checkstyle 职责重叠（记录在此以免下轮重复讨论） |

### 3.4 旧版本模块上的降级（硬地板，来自 legacy 研究）

| 工具 | 目标 Java ≤ 8 | 目标 Java 11–16 | 目标 Java 17+ |
|---|---|---|---|
| Checkstyle | **9.3**（最后一个能跑在 Java 8 上的版本；Gradle 9 默认 10.24 需 Java 11） | 10.x（默认 `toolVersion` 即可） | 13.x / 14.x（需运行 JVM 21） |
| SpotBugs（若开） | **4.8.6** | 4.9.x/4.10.x | 4.10.x |
| JUnit | **5.14.4**（JUnit 6 是 major 61，Java 8 加载不了） | 5.14.4 | 6.1.x |
| 运行 Gradle 的 JVM | 17–27（Gradle 9.8 硬要求，**不是** Java 8） | 同左 | 同左 |

模板把上表做成 `libs.versions.toml` 里的**按模块条件引用**，而不是全局单一版本；这样 `1.8.9` 的 `platforms/bukkit` 用 checkstyle 9.3，`platforms/paper`（现代）用 13.x，同一工程内并存。

## 4. README / LICENSE / CHANGELOG 与语言

| 文件 | 必含 | 语言行为 |
|---|---|---|
| `README.md` | 插件是什么；前置（目标 MC / 所需 Java / 支持的平台）；`./gradlew build` + `./gradlew runServer`（老版本注明无本地测试服）；产物路径；权限表；配置表；"由 vinoa 生成"的一句 | `zh` → 中文；`en` → 英文；`both` → `README.md`(zh) + `README.en.md`（顶部互链） |
| `LICENSE` | Apache-2.0 英文全文，`Copyright (c) <year> <author>` | **不翻译**；`both` 也在 README 里挂中文说明链接，不生成 `LICENSE.zh` |
| `CHANGELOG.md` | Keep a Changelog 骨架 + `## [Unreleased]` + `0.1.0` 首条；语义化版本约定一句 | 跟随生成物语言；标题锚点（`Unreleased` 等）保持英文以便工具解析 |
| 代码注释 / Javadoc | 每个示例类头部一句用途；公共方法一行说明 | 跟随生成物语言 |
| 玩家可见消息（`messages_*.yml`、`config.yml`） | `zh` 只留 `messages_zh.yml`；`en` 只留 `messages_en.yml`；`both` 两个都留 + `default-locale` 键 | 与 README 同一选择，**一次决定、全工程一致** |
| `.github/workflows/*.yml` 与 `build.gradle.kts` 注释 | 仅保留必要英文注释 | **不**跟随语言（构建文件的注释保持英文，避免编码/工具解析差异） |

## 5. `git init` 默认与忽略规则

| 项 | 默认 | 说明 |
|---|---|---|
| 是否初始化 | **是**（`--no-git` 关） | 落盘**成功之后**才 init（原子落盘 → 再 `git init`） |
| 分支名 | `main` | 用 `git init -b main`；检测到旧 git 时回退 `git init` + `git symbolic-ref HEAD refs/heads/main` |
| 首次提交 | 一次提交，message：`chore: scaffold <project> with vinoa` | 提交前跑 `git add -A`；**不**代配 user.name/email（缺失时跳过提交并警告） |
| wrapper jar | **提交** `gradle/wrapper/gradle-wrapper.jar` | CI 与无网机器必须有它；`.gitignore` 明确不排除它 |
| 已在 git 仓库内 | 不重复 init，只在摘要里说明 | 避免把新工程塞进内层仓库 |
| 换行符 | `.gitattributes`：`* text=auto`、`gradlew text eol=lf`、`*.bat text eol=crlf`、`*.jar binary` | Windows 原生支持的必要项（否则 `gradlew.bat` 在 Linux/CI 上崩） |

`.gitignore` 内容（生成物，非 vinoa 自身）：

```
# Gradle
.gradle/
build/
!gradle/wrapper/gradle-wrapper.jar
!gradle/wrapper/gradle-wrapper.properties

# IDE
.idea/
*.iml
.vscode/
*.ipr *.iws

# 运行/日志
run/
logs/
*.log
.vinoa/

# OS
.DS_Store
Thumbs.db
```

## 6. "必须替换、零残留"检查清单（clean-room）

生成物交付前（以及 `--verify` 前）跑同一套断言。设计意图与同类项目的 customization 文档一致——**新工程里不应残留任何占位符或脚手架痕迹**——但检查项与文案为本项目自写。

| # | 检查 | 判定方式 |
|---|---|---|
| 1 | 无占位符 | 生成文件全文不含 `com.example`、`example.com`、`Example-Plugin`、`__NAME__`、`{{`、`}}`、`TODO`、`FIXME`、`XXX`（`ExampleService`/`ExampleCommand` 这类**示例类名**允许存在，正是"示例代码"的语义） |
| 2 | 无脚手架痕迹 | 不含 `vinoa`（README 的"由 vinoa 生成"一句除外）、不含 `Crimson`/其他工具名、不含 "generated template" 式注释 |
| 3 | 名称一致 | `settings.gradle.kts` 的 `rootProject.name`、`plugin.yml` 的 `name`、`Permissions` 常量前缀、命令名/别名、包路径四者互相对得上；不得出现同一工程的两种拼写（`my-plugin` vs `myplugin`） |
| 4 | 主类可寻 | `plugin.yml`/`paper-plugin.yml`/`bungee.yml` 的 `main` 指向的类文件**真实存在**于对应模块源码；类名不等于 `Main`（沿用公开命名建议） |
| 5 | 坐标可解析 | `libs.versions.toml` 里每个坐标都能在矩阵/仓库命中；不存在字符串拼接出来的 `{mc}-R0.1-SNAPSHOT` 型坐标（尤其 1.8.9） |
| 6 | 注册成对 | 每个命令名在"元数据声明"与"代码注册"里**恰好出现一次**（bukkit/paper+plugin.yml 两处都算一次；paper-plugin.yml/velocity/bungeecord/sponge/minestom 只在代码一处） |
| 7 | 权限闭环 | 元数据（或代码）里声明的每个节点都有对应常量；代码里检查的每个节点都在元数据/代码里声明；无悬空常量 |
| 8 | 语言闭环 | 语言选择为 `zh` 时不存在 `messages_en.yml`，`en` 时不存在 `messages_zh.yml`；README/LICENSE/CHANGELOG 的存在形态与 §4 表一致 |
| 9 | 文件清单一致 | `--dry-run` 输出的清单 == 实际落盘集合（含删除项：`--no-example` 不产出 adapter、`--no-permissions` 不产出常量类） |
| 10 | 工程能构建 | `--verify` 或 CI 的 `./gradlew build`（§3.3 的 `check` 已含 checkstyle 与单测） |
| 11 | 老版本专属 | 1.8.9 工程不含 `paper-api`、不含 `paper-plugin.yml`、不含 `runServer` 配置；`spigot-api` 是 `1.8.8-R0.1-SNAPSHOT` |
| 12 | Windows 可用 | 路径与文件内容不含硬编码绝对路径；`gradlew.bat` 在列且行尾为 CRLF；无 shell-only 脚本（`*.sh` 最多作为可选补充） |

## 待定 / 风险

1. **`config.yml` 归属**：放在 `core/src/main/resources` 由各平台模块共享，还是每平台各一份？当前草稿按共享（少重复、老版本也能读），代价是 `processResources` 需要跨模块复制资源的配置。
2. **`integrationTest` 的默认空转**是否值得：多一个 source set 与任务，但没有用例；也可能默认关、由 `--features integration-test` 开。
3. **Checkstyle 9.3 vs 10.x 的配置兼容性**：同一份 `checkstyle.xml` 要同时喂 9.3 和 13.x；`LineLength`/`ImportOrder` 等模块在 10/11/13 之间有弃用变更，需要一次实测（未验证）。
4. **BungeeCord/CONTRIBUTING 等可选文件的取舍**：文件越多，"零残留"检查面越大；倾向只保留 `.editorconfig`。
5. **Sponge / Minestom 的示例代码**：两者都是代码内注册、无资源描述符，且 SpongeAPI 线随 MC 版本跳变（1.8.9→4.2.0 … 26.2→20.0.0）。本轮只给出形状，**具体 API 名与注册时机必须逐版本实测**（否则会生成编译不过的"示例"）。
6. **`release.yml` 是否需要**：会引入 tag/权限/产物上传的复杂度，可能与"生成物 CI 只跑 build"的基线冲突。
7. **`.gitattributes` 是否默认生成**：对 Windows 体验有帮助，但对纯 Linux 用户是噪音；倾向默认生成（换取跨平台确定性）。
8. **示例代码的"可运行性"承诺等级**：属于 acceptance 中 A 行的证明对象，但 B 级组合上的示例代码**不保证**编译（见 `acceptance.md` §7）。
