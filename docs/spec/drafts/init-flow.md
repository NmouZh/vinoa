# vinoa `init` 执行流程（草稿 v8）

> 供 ticket [#17 init 的执行流程与阶段划分](https://github.com/NmouZh/vinoa/issues/17) 与 [#10 旧版本与多 Java 目标兼容路径](https://github.com/NmouZh/vinoa/issues/10) 讨论用。
> 交互形态按用户要求对齐 **IDEA 的"新建项目"向导**：逐页填写，字段与顺序照它。内部阶段划分见附录。

## 1. 向导（交互式默认路径）

```
$ vinoa init
┌─ 新建 vinoa 工程 ─────────────────────────────────┐
│ ① 工程                                           │
│   名称        my-plugin                          │
│   位置        /root/projects/my-plugin           │
│   包名        com.example.myplugin               │
│                                                  │
│ ② 构建                                           │
│   语言        Java                               │
│   构建系统    Gradle                             │
│   Gradle DSL  Kotlin (build.gradle.kts)          │
│   JDK         21  ✓ 已检测到      [下载/选择…]    │
│                                                  │
│ ③ 目标服务端                                      │
│   MC 版本     1.21.11  ▾（只列我们支持的版本）      │
│   平台        [x] paper  [ ] velocity  [ ] …      │
│               （勾 paper 自动带上 bukkit 兼容模块）  │
│   元数据      (•) plugin.yml   ( ) paper-plugin   │
│                                                  │
│ ④ 附加                                           │
│   [x] 示例代码（一条命令 + 一个监听器 + config）    │
│   [x] 权限声明（permissions + 权限常量类）          │
│   [x] 质量工程（checkstyle + JUnit + CI）          │
│   [ ] SQLite 持久化     [ ] bStats 统计            │
│   [ ] 更新检查          [ ] PlaceholderAPI         │
│   [ ] GUI 菜单骨架                                 │
│   [x] git init + 首次提交                          │
│   生成物语言   中文 / English / 双语               │
└──────────────────────────────────────────────────┘
  ← → 切页   Tab 下一项   Enter 确认   Esc 取消
```

确认与执行：

```
即将创建（共 N 个文件）:
  my-plugin/
  ├─ settings.gradle.kts · build.gradle.kts · gradle/libs.versions.toml
  ├─ core/                    共享抽象（命令 / 配置 / 消息 / 日志）
  ├─ platforms/paper/         Paper 模块
  ├─ platforms/bukkit/        勾 paper 时自动带上
  └─ README.md · LICENSE · .github/workflows/build.yml

? 确认创建？ (Y/n) y

  渲染模板 ✓    写入落盘 ✓    git init + 首次提交 ✓
✓ 完成: /root/projects/my-plugin

下一步:
  cd my-plugin
  ./gradlew build          # 构建插件 jar
  ./gradlew runServer      # 起本地测试服（首次需联网）
  注意: 当前目标是 1.21.11（Java 21）。
```

**缺 Java 时**（多平台勾选、本机只装了其中一个）：

```
本机环境预检:
  Java 21  ✓ /usr/lib/jvm/java-21-openjdk        （paper 模块需要）
  Java 25  ✗ 未检测到                            （velocity 模块需要）

? 缺少的 Java 25 是否让 Gradle 首次构建时自动下载？ (Y/n) y
  → 已在生成的工程中启用 toolchain 自动下载（foojay-resolver-convention）
（答 n：不加该插件，并提示“构建前需自行安装 Java 25，否则 ./gradlew build 会失败”）
```

## 2. 命令行界面（非交互 / AI / CI 路径）

设计原则：**向导负责交互，参数只服务“无人可问”的场景**。高频的给短参数，其余全部走配置文件；不搞一族 `--with-xxx`。

```
$ vinoa init --help
用法: vinoa init [名称] [选项]

参数:
  [名称]                  工程名；省略则在当前目录生成

常用:
  -p, --package <包名>     Java 包名（默认 com.example.<名称>）
  -m, --mc <版本>          目标 Minecraft 版本（如 1.21.11）
      --platform <平台>    目标平台，可重复
      --features <列表>    附加模块，逗号分隔：sqlite,bstats,update-check,placeholderapi,gui
  -y, --yes               全部用默认值，不询问
      --dry-run           只打印将要生成的内容
      --json              以 JSON 输出（给脚本 / agent 解析）
      --verify            生成后跑一次构建验证

高级:
  -c, --config <文件>      从 TOML 读取全部答案（与向导等价，可版本化、可进仓库）
      --print-config       把本次选择导成 TOML，配合 -c 用
      --metadata / --license / --author / --description / --lang
      --no-git / --no-example / --no-permissions / --no-quality
```

等价命令示例：

```bash
vinoa init my-plugin -p com.example.myplugin \
  -m 1.21.11 --platform paper --features sqlite,bstats -y
```

- `--dry-run`：只打印"预检 + 即将创建"那一段（同一份计划，保证"预览即实际"）。
- `--json`：把计划 / 预检结果 / 结果输出成结构化 JSON，供 agent 解析。
- `--yes`：跳过确认；非 TTY 环境下自动等价于 `--yes`。
- 缺必填参数且无法询问 → 报错退出并列出缺失项（不挂起）。
- 非交互下"缺 Java 是否自动下载"由 `--download-jdk / --no-download-jdk` 决定（`--yes` 默认取保守值：不下载）。

## 3. 失败时用户看到什么

| 情形 | 用户看到 |
|---|---|
| 目标目录非空 | `目录已存在且非空`，列出冲突文件，给出 `--output <other>` 或明确拒绝（默认不覆盖） |
| 平台 × 版本组合不存在 | 交互路径下**根本选不到**（框里只列支持项）；非交互路径**硬报错 + 列出可用项**，绝不静默裁剪 |
| 本机缺所需 Java | 列出"哪个模块需要哪个 Java、当前装了什么"，并**询问是否让 Gradle 自动下载**；不阻塞生成 |
| 版本矩阵联网刷新失败 | 回退内置矩阵并警告；离线不阻塞生成 |
| 中途中断 / 渲染失败 | 不留半个工程（原子落盘），说明已回滚 + 可复现命令 |
| `--verify` 构建失败 | 工程**保留**，报告失败原因与日志路径，退出码标记"生成成功、验证失败" |

## 4. 已确认的流程（2026-09-25）

1. **入口形态**：`vinoa init` 作用于当前目录；`vinoa init <name>` 新建子目录。
2. **向导形态**：逐页填写（工程 / 构建 / 目标服务端 / 附加），字段与顺序对齐 IDEA 新建项目向导；页间可回退。
3. **确认步骤**：默认打印摘要 + `Y/n`；`--yes` 与非 TTY 跳过。
4. **完成提示**：路径 + `cd` + `./gradlew build` + `./gradlew runServer` + 版本注意事项 + 文档链接。
5. **失败语义**：照 §3。
6. **平台清单**：paper / bukkit / velocity / bungeecord / folia / sponge / minestom（**7 个**，Nukkit 已移出：基岩版）。
7. **版本下界**：MC **1.8.9 → 26.2**；1.8.9 走 Spigot/Bukkit（`spigot-api:1.8.8-R0.1-SNAPSHOT`），Paper 从 1.9.4 起。
8. **版本与平台的选择框**：先选 MC 版本（只列受支持版本），再选平台（只列该版本支持的平台；代理端不锁 MC 版本，恒在）。
9. **paper 与 bukkit**：分成两个模块，但勾 `paper` 时**自动带上** `platforms/bukkit`（无须用户再勾一次）。
10. **许可证**：工具自身与生成物都用 **Apache-2.0**（clean-room，README 注明参考来源，不复制代码正文）。
11. **④ 附加页默认值**：示例代码 / 权限声明 / 质量工程 / git 初始化 **默认勾**；SQLite / bStats / 更新检查 / PlaceholderAPI / GUI 菜单 **默认不勾**（挑上就生成）。
12. **界面语言**：跟随系统语言（中文系统显示中文，否则英文），可用 `--ui-lang` 覆盖。注意它与**生成物语言**是两个独立概念。
13. **生成物语言**：注释与 README 跟随向导里选的生成物语言（中文 / English / 双语）；玩家可见的消息也跟随它。
14. **Windows 正式支持**：原生 `cmd` / PowerShell 也是目标环境（向导界面需跨平台适配，路径与 `git init` 行为要一起考虑）。

**内部实现选择（用户不必关心，可随时推翻）**：采用"先算完整计划 → 落盘只执行计划"的单一代码路径，使 `--dry-run` 的输出与实际写入必然一致；阶段划分见附录。

## 5. 环境预检（Java）—— 用户口径，2026-09-25

**规则：init 不直接问 Java 版本。** 只问 MC 版本，Java 由版本矩阵推出：

1. 矩阵对每个 MC 版本给出 `java_min`（必须）与 `java_recommended`（推荐）。
2. 多平台勾选时**按模块分别列出**所需 Java（例：bukkit 需 8、paper 需 21、velocity 需 25）。
3. 检测本机（实现细节由我拍）：`JAVA_HOME` → PATH 上的 `java -version` → **常见安装目录扫描**（Linux `/usr/lib/jvm`、Windows `Program Files\Java` 与 Adoptium、macOS `/Library/Java/JavaVirtualMachines`、SDKMAN / asdf 目录）。不走 Gradle 进程（慢且离线不可用）；`--json` 里给出结构化结果。
4. 结果出现在确认摘要里（✓/✗ 一眼可见）。
5. **缺 Java 不静默、也不替用户决定**——缺失时**问一句**："是否让 Gradle 首次构建时自动下载？"答是 → 工程内启用 `foojay-resolver-convention`；答否 → 不加插件并提示"构建前需自行安装 Java X"。
6. 老版本提示：1.8.9 目标需 Java 8，且该版本**起不了本地 Paper 测试服**（Paper 没有 1.8.9），提示里必须说清。

## 附录：内部阶段划分（用户不可见）

| # | 阶段 | 职责 | 失败产物 |
|---|---|---|---|
| 0 | 上下文与 TTY 检测 | 目录占用、是否在 git 仓库内、是否离线、TTY 与否 | 无 |
| 1 | 输入收集 | 向导填写 / flags 补齐 | 无 |
| 2 | 校验 | 名称/包名合法性、目录占用、平台×版本组合 | 无 |
| 3 | 版本矩阵解析 | 锁定平台坐标 / Java 目标 / Gradle / 元数据格式 | 无 |
| 3.5 | 环境预检 | 按已解析的 Java 目标查本机 JDK，产出 ✓/✗ 清单；缺失时触发"是否自动下载"询问 | 无（不阻塞） |
| 4 | 计划生成 | 完整文件清单 + 内容 + 额外动作（这就是 `--dry-run` 的产物） | 无 |
| 5 | 渲染 | 模板树 → 内存文件集（变量替换、条件文件、重命名） | 无 |
| 6 | 落盘 | 临时目录 → 整体 rename（原子） | 回滚，无残留 |
| 7 | 收尾 | `git init` + 首次提交、可选 `--verify` | 见 §3 |
| 8 | 结果输出 | 人类可读摘要 / `--json` | — |
