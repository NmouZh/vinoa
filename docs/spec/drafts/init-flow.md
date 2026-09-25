# vinoa `init` 执行流程（草稿 v5）

> 供 ticket [#17 init 的执行流程与阶段划分](https://github.com/NmouZh/vinoa/issues/17) 与 [#10 旧版本与多 Java 目标兼容路径](https://github.com/NmouZh/vinoa/issues/10) 讨论用。
> 主体是**用户可见的完整流程**；内部阶段划分见附录。本文件随决策持续更新。

## 1. 一次真实会话（交互式默认路径）

```
$ vinoa init
vinoa 0.1.0 · 交互式创建 Minecraft 服务端插件工程
当前目录 /root/projects/my-plugin   ·   空目录 ✓

── 工程身份 ──────────────────────────────
? 插件名称: my-plugin
? 包名: com.example.myplugin
? 作者: NmouZh
? 描述: 一个示例插件

── 目标版本与平台 ────────────────────────
? 目标 Minecraft 版本（只列我们支持的版本）:
  1.8.9 · 1.12.2 · 1.16.5 · 1.17.1 · 1.19.4 · 1.20.6 · 1.21.4 · 1.21.11 · 26.1.2 · 26.2
  → 选中 1.21.11
? 目标平台（只列 1.21.11 支持的平台；代理端不锁 MC 版本，恒在）:
  [x] paper      [ ] bukkit     [ ] velocity   [ ] bungeecord
  [ ] folia      [ ] sponge     [ ] minestom
   → 已解析: paper-api 1.21.11-R0.1-SNAPSHOT · Java 21 · Gradle 9.8.0
? 元数据格式: ( ) plugin.yml（推荐）   ( ) paper-plugin.yml（实验性）

── 可选项 ────────────────────────────────
? 生成物语言: 中文 / English / 双语
? 可选模块: [ ] 持久化(SQLite)  [ ] 运行时多语言  [ ] GUI 骨架  [ ] 生态集成
? 初始化 git 仓库: 是

── 确认 ──────────────────────────────────
本机环境预检:
  Java 21  ✓ /usr/lib/jvm/java-21-openjdk        （paper 模块需要）
  Gradle   ✓ 9.8.0

即将创建（共 27 个文件）:
  my-plugin/
  ├─ settings.gradle.kts · build.gradle.kts · gradle/libs.versions.toml
  ├─ core/              共享抽象（命令 / 配置 / 消息 / 日志）
  ├─ platforms/paper/   Paper 模块 + plugin.yml + 示例命令 + 示例监听器
  └─ README.md · LICENSE · .github/workflows/build.yml

? 确认创建？ (Y/n) y

── 执行 ──────────────────────────────────
  渲染模板     27 个文件 ✓
  写入落盘     ✓
  git init + 首次提交 ✓

✓ 完成: /root/projects/my-plugin

下一步:
  cd my-plugin
  ./gradlew build        # 构建插件 jar
  ./gradlew runServer    # 起本地测试服（首次需联网）
  注意: 当前目标是 1.21.11（Java 21）。
  文档: docs/usage.md · 模板变量说明见 README
```

**缺 Java 时**（多平台勾选、本机只装了其中一个）的样子：

```
本机环境预检:
  Java 21  ✓ /usr/lib/jvm/java-21-openjdk        （paper 模块需要）
  Java 25  ✗ 未检测到                            （velocity 模块需要）

? 缺少的 Java 25 是否让 Gradle 首次构建时自动下载？ (Y/n) y
  → 已在生成的工程中启用 toolchain 自动下载（foojay-resolver-convention）

（答 n 时：不加该插件，并提示“构建前需自行安装 Java 25，否则 ./gradlew build 会失败”）
```

## 2. 非交互等价命令（AI / CI 路径）

```bash
vinoa init --name my-plugin --package com.example.myplugin \
  --platform paper --mc 1.21.11 --lang zh --license MIT \
  --git --yes
```

- `--dry-run`：只打印上面"预检 + 即将创建"那一段（同一份计划，保证"预览即实际"）。
- `--json`：把计划 / 预检结果 / 结果输出成结构化 JSON，供 agent 解析。
- `--yes`：跳过确认；非 TTY 环境下自动等价于 `--yes`。
- 缺必填参数且无法询问 → 报错退出并列出缺失项（不挂起）。
- 非交互下"缺 Java 是否自动下载"由 `--download-jdk / --no-download-jdk` 决定（默认跟随 `--yes` 的保守值：不下载）。

## 3. 失败时用户看到什么

| 情形 | 用户看到 |
|---|---|
| 目标目录非空 | `目录已存在且非空`，列出冲突文件，给出 `--output <other>` 或明确拒绝（默认不覆盖） |
| 平台 × 版本组合不存在 | 交互路径下**根本选不到**（框里只列支持项）；非交互路径**硬报错 + 列出可用项**，绝不静默裁剪 |
| **本机缺所需 Java** | 列出"哪个模块需要哪个 Java、当前装了什么"，并**询问是否让 Gradle 自动下载**；不阻塞生成 |
| 版本矩阵联网刷新失败 | 回退内置矩阵并警告；离线不阻塞生成 |
| 中途中断 / 渲染失败 | 不留半个工程（原子落盘），说明已回滚 + 可复现命令 |
| `--verify` 构建失败 | 工程**保留**，报告失败原因与日志路径，退出码标记"生成成功、验证失败" |

## 4. 已确认的流程（2026-09-25）

1. **入口形态**：`vinoa init` 作用于当前目录；`vinoa init <name>` 新建子目录。
2. **提问节奏**：分三组（身份 / 平台与版本 / 可选项），组间可回退。
3. **确认步骤**：默认打印摘要 + `Y/n`；`--yes` 与非 TTY 跳过。
4. **完成提示**：路径 + `cd` + `./gradlew build` + `./gradlew runServer` + 版本注意事项 + 文档链接。
5. **失败语义**：照 §3 —— 非空目录不覆盖、原子落盘无残留、联网刷新失败回退内置矩阵并警告、`--verify` 失败保留工程并用单独退出码。
6. **平台清单**：paper / bukkit / velocity / bungeecord / folia / sponge / minestom（**7 个**，Nukkit 已移出：基岩版）。
7. **版本下界**：MC **1.8.9 → 26.2**；1.8.9 走 Spigot/Bukkit（`spigot-api:1.8.8-R0.1-SNAPSHOT`），Paper 从 1.9.4 起。
8. **版本与平台的选择框**：**先选 MC 版本**（只列受支持的版本），**再选平台**（只列该版本支持的平台；代理端是协议范围、不锁 MC 版本，恒在）。交互路径下不存在“无效组合”；非交互路径传了不支持的组合则**硬报错并列出可用项**，不静默裁剪。

**内部实现选择（用户不必关心，可随时推翻）**：采用"先算完整计划 → 落盘只执行计划"的单一代码路径，使 `--dry-run` 的输出与实际写入必然一致；阶段划分见附录 0–8。

## 5. 环境预检（Java）—— 用户口径，2026-09-25

**规则：init 不直接问 Java 版本。** 只问 MC 版本，Java 由版本矩阵推出：

1. 矩阵对每个 MC 版本给出 `java_min`（必须）与 `java_recommended`（推荐）。
2. 多平台勾选时**按模块分别列出**所需 Java（例：bukkit 需 8、paper 需 21、velocity 需 25）。
3. 检测本机（实现细节由我拍）：`JAVA_HOME` → PATH 上的 `java -version` → **常见安装目录扫描**（Linux `/usr/lib/jvm`、Windows `Program Files\Java` 与 Adoptium、macOS `/Library/Java/JavaVirtualMachines`、SDKMAN / asdf 目录）。不走 Gradle 进程（慢且离线不可用）；`--json` 里给出结构化结果。
4. 结果出现在确认摘要里（✓/✗ 一眼可见）。
5. **缺 Java 不静默、也不替用户决定**——预检发现缺失时**问一句**："是否让 Gradle 首次构建时自动下载？"
   - 答 **是**：在生成的工程里启用 `foojay-resolver-convention`，首次构建自动拉取所需 JDK；
   - 答 **否**：不加该插件，并明确提示"构建前需自行安装 Java X，否则 `./gradlew build` 会失败"；
   - 无论哪条，都同时说清"哪个模块缺哪个 Java、可以怎么装"。
6. 老版本提示：1.8.9 目标需 Java 8，且该版本**起不了本地 Paper 测试服**（Paper 没有 1.8.9），提示里必须说清。

## 附录：内部阶段划分（用户不可见）

| # | 阶段 | 职责 | 失败产物 |
|---|---|---|---|
| 0 | 上下文与 TTY 检测 | 目录占用、是否在 git 仓库内、是否离线、TTY 与否 | 无 |
| 1 | 输入收集 | flags + 交互补齐 | 无 |
| 2 | 校验 | 名称/包名合法性、目录占用、平台×版本组合 | 无 |
| 3 | 版本矩阵解析 | 锁定平台坐标 / Java 目标 / Gradle / 元数据格式 | 无 |
| 3.5 | 环境预检 | 按已解析的 Java 目标查本机 JDK，产出 ✓/✗ 清单；缺失时触发"是否自动下载"询问 | 无（不阻塞） |
| 4 | 计划生成 | 完整文件清单 + 内容 + 额外动作（这就是 `--dry-run` 的产物） | 无 |
| 5 | 渲染 | 模板树 → 内存文件集（变量替换、条件文件、重命名） | 无 |
| 6 | 落盘 | 临时目录 → 整体 rename（原子） | 回滚，无残留 |
| 7 | 收尾 | `git init` + 首次提交、可选 `--verify` | 见 §3 |
| 8 | 结果输出 | 人类可读摘要 / `--json` | — |
