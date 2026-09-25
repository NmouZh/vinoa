# 生成工程的目录结构（草稿 v1）

> 供 ticket [#5 多模块工程骨架与共享抽象](https://github.com/NmouZh/vinoa/issues/5) 使用。
> 本文记录**已与用户确认**的部分与**仍未定**的部分，避免下一个会话重复提问。

## 已确认（2026-09-25）

| 项 | 结论 |
|---|---|
| 模块分组 | `core/` + `platforms/<平台>/`（**不拍平**，平台模块收在 `platforms/` 下） |
| 共享模块名 | `common/` → **`core/`** |
| 平台层目录名 | `platform/` → **`platforms/`** |
| Gradle 模块路径 | `include("core", "platforms:paper", "platforms:velocity", …)` |
| 包内结构 | 保留 `<包名>/paper/…` 平台子包（不做平铺） |

## 样例树（`vinoa init --platform paper --platform velocity`）

```
my-plugin/
├─ settings.gradle.kts          # 按勾选条件 include 平台模块
├─ build.gradle.kts             # 根：公共插件与质量工程配置
├─ gradle.properties
├─ gradle/
│  ├─ libs.versions.toml        # 版本目录：平台坐标 / Java 版本都从这里出
│  └─ wrapper/
├─ gradlew · gradlew.bat
├─ .gitignore · README.md · LICENSE · CHANGELOG.md
├─ .github/workflows/build.yml  # CI 矩阵
├─ config/checkstyle/checkstyle.xml
├─ core/                        # 平台无关：抽象接口 + 共享逻辑，不 import 任何平台 API
│  ├─ build.gradle.kts
│  └─ src/main/java/<包名>/
│     ├─ command/               # 命令抽象
│     ├─ config/                # 配置模型与加载接口
│     ├─ message/               # 消息 / 本地化接口
│     └─ ExampleService.java    # 示例业务代码
└─ platforms/
   ├─ paper/                    # toolchain 25（现代 Paper）
   │  ├─ build.gradle.kts
   │  └─ src/main/
   │     ├─ java/<包名>/paper/
   │     │  ├─ PaperPlugin.java
   │     │  ├─ command/ExampleCommand.java
   │     │  └─ listener/ExampleListener.java
   │     └─ resources/{plugin.yml, config.yml}
   └─ velocity/                 # toolchain 25（Velocity 4.x）
      ├─ build.gradle.kts
      └─ src/main/java/<包名>/velocity/VelocityPlugin.java
         # 注解处理器直接生成 velocity-plugin.json 到 jar 根，无需 resources
```

## 已定（2026-09-25 补充：上一轮六个未定项全部收口）

### 1. paper 与 bukkit：两个模块，勾 paper 自动带 bukkit

（用户拍板）`platforms/paper` 用 `paper-api`（可用 Paper 专属能力），`platforms/bukkit` 用 `spigot-api`（也能跑 Spigot）。init 里勾 `paper` 时自动 `include("platforms:bukkit")`；只勾 bukkit 时不带 paper。

### 2. 每个模块的 Java 目标

| 模块 | Java 目标 |
|---|---|
| `paper` / `folia` | 该 MC 版本的 `java_min`（1.21.11 → 21；26.2 → 25） |
| `bukkit` | 1.8.9–1.16.x → **8**；之后用该版本的 `java_min` |
| `velocity` | **25**（Velocity 4.x） |
| `bungeecord` | **8**（release 线 `1.21-R0.4`；快照线已到 17） |
| `sponge` | 按所用 `spongeapi` 版本的要求 |
| `minestom` | **25** |
| `core` | **所有已启用模块中最低的那个目标** |

**关键约束**：`core/` 必须编译在启用模块里最低的 Java 目标上——勾了 bukkit(8) + paper(21) 时，`core` 就得是 8，否则低版本平台模块根本无法依赖它。

### 3. 质量工程在旧版本模块上的降级

| 目标 | Checkstyle | SpotBugs | JUnit |
|---|---|---|---|
| Java 8 | 9.3（最后支持） | 4.8.6（最后支持） | 5.14.4（最后一条 Java 8 线） |
| Java 11+ | Gradle 默认 10.24.0 | 最新 | 5.x |
| Java 17+ | 13+ | 最新 | 6.x |

注意：Checkstyle / SpotBugs 跑在 **Gradle 的 JVM** 上，不要求目标 JDK；上表是"能分析 Java 8 字节码"的下界。**JUnit 6 需要 Java 17**，所以老版本模块的测试只能用 5.x。

### 4. 冷门平台标实验性

`sponge` / `minestom` / `folia` 在向导里带 **experimental** 标注，生成的 README 里也注明，且**不进承诺矩阵**（best-effort）。`minestom` 另加提示：只支持 `1.21.11` / `26.1.1` / `26.1.2` / `26.2` 四个版本。

### 5. `gradle/libs.versions.toml` 的组织

```toml
[versions]
paperApiVersion  = "1.21.11-R0.1-SNAPSHOT"   # 由版本矩阵写入，不手改
spigotApiVersion = "1.8.8-R0.1-SNAPSHOT"
checkstyle       = "10.24.0"
junit            = "5.14.4"

[libraries]
paper-api  = { module = "io.papermc.paper:paper-api",  version.ref = "paperApiVersion" }
spigot-api = { module = "org.spigotmc:spigot-api",     version.ref = "spigotApiVersion" }

[plugins]
shadow    = { id = "com.gradleup.shadow",        version = "9.6.1" }
run-paper = { id = "xyz.jpenilla.run-paper",     version = "3.1.0" }
```

规则：
- 每个启用的平台一个 `[versions]` 条目，**模块只引用、不硬编码**——这是未来 `build` 命令能复用版本矩阵的前提。
- 平台坐标一律来自矩阵的查找表（见 [version-matrix.md](version-matrix.md)），禁止字符串拼接。
- 不再额外生成 `gradle.properties` 里的版本号，避免两处真相。

### 6. `core/` 的边界

- **放**：平台无关接口（`Command` / `Config` / `Message` / `Scheduler` / `Sender` / `MenuService` 能力接口）+ 纯逻辑（参数解析、消息格式化、配置模型）+ 示例业务代码（示范"共享逻辑长什么样"）。
- **禁止**：import 任何平台 API（`org.bukkit` / `io.papermc` / `com.velocitypowered` / `net.md_5`）。由质量工程里的 import 检查保证，并写进验收。
- 平台模块只做三件事：bootstrap 入口、平台 API 适配、资源文件。

### 生成出来的 `settings.gradle.kts`

```kotlin
rootProject.name = "my-plugin"
include("core")
include("platforms:paper")
include("platforms:bukkit")   // 勾 paper 自动带上
```

## 仍未定

- 无（本票的决策项已全部收口）。
