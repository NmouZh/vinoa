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

## 仍未定（下一轮要拍）

1. **paper 与 bukkit 是否合并成一个模块**：Paper 是 Bukkit 超集，勾 paper 时 bukkit 模块可能是冗余的一层；合并则少一层目录、但"纯 Bukkit 兼容"的语义会模糊。
2. **每个平台模块的 Java 目标**：Velocity 要 Java 25，BungeeCord release 线还是 Java 8——平台模块不能共用 toolchain 已确定，但**具体每个模块写多少**要等 [老版本构建可行性](https://github.com/NmouZh/vinoa/issues/8) 的结论。
3. **质量工程在旧版本模块上的降级**：checkstyle / spotbugs / JUnit 版本随 Java 目标变化（同样等 #8）。
4. **冷门平台（sponge / minestom / nukkit / folia）是否标实验性**：影响 `settings.gradle.kts` 注释与 init 交互里的提示文案。
5. **`gradle/libs.versions.toml` 的组织方式**：平台坐标是按平台分节，还是集中一节（要能被未来的 `build` 命令复用）。
6. **`core/` 的边界**：只放接口，还是允许放共享业务代码（当前草稿按"接口 + 示例业务代码"）。
