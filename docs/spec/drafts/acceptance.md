# 生成物验收标准（必须构建通过的组合）（草稿 v1）

> 供 ticket [#11 生成物验收标准（必须构建通过的组合）](https://github.com/NmouZh/vinoa/issues/11) 使用。
> 上游口径：[init-flow §3/§4](init-flow.md)（失败语义、`--verify`）、[version-matrix §3](version-matrix.md)（查询与裁剪）、[legacy-mc-build-feasibility §6](../../research/legacy-mc-build-feasibility.md)（硬地板）。

## 1. 承诺等级

| 等级 | 含义 | 谁保证 |
|---|---|---|
| **A 保证** | vinoa 自己的 CI 每个 PR 真跑一次 `./gradlew build`，绿了才算验收 | 下表 §2 全部行 |
| **B 尽力而为** | 生成物按同一套规则渲染、坐标经矩阵校验，但**不保证**通过 `./gradlew build` | 其余所有合法组合 |
| **C 拒绝** | 组合本身不存在，非交互路径硬报错（§6.1） | 矩阵未命中的 `(mc, platform)` |

**用户决定已确认**：只有**代表性组合**保证 `./gradlew build` 通过，其余为尽力而为。生成成功 ≠ 构建成功；`--verify` 是唯一能把二者绑在一起的手段（§4）。

## 2. 保证矩阵（每条 = CI 里真跑 Gradle 的一行）

统一条件：元数据 `plugin.yml`、示例代码 + 权限声明 + 质量工程全开（向导默认）、Gradle wrapper 9.8.0、**离线依赖已预热**（§5.3）。

| # | MC 版本 | 平台（连带模块） | 模块 toolchain（生成物） | vinoa CI 实际执行 build 的 JDK | 依赖坐标（矩阵查表，非拼接） |
|---|---|---|---|---|---|
| A1 | 1.8.9 | bukkit（单独勾选） | **8** | **21**（用 `options.release = 8`） | `org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT` |
| A2 | 1.12.2 | paper + bukkit | 8 | 21 | `com.destroystokyo.paper:paper-api:1.12.2-R0.1-SNAPSHOT` |
| A3 | 1.16.5 | paper + bukkit | 8 | 21 | `com.destroystokyo.paper:paper-api:1.16.5-R0.1-SNAPSHOT` |
| A4 | 1.21.11 | paper + bukkit | 21 | 21 | `io.papermc.paper:paper-api:1.21.11-R0.1-SNAPSHOT` |
| A5 | 26.2 | paper + bukkit | 25 | **25** | `io.papermc.paper:paper-api:26.2.build.+`（钉 `26.2.build.129-stable`） |
| A6 | 26.2 | folia | 25 | 25 | 同 A5（`paper-api`；`folia-supported: true`） |
| A7 | 26.2 | sponge | 25 | 25 | `org.spongepowered:spongeapi:20.0.0-SNAPSHOT` |
| A8 | 26.2 | minestom | 25 | 25 | `net.minestom:minestom:2026.09.12-26.2` |
| A9 | — （代理端不锁 MC） | velocity | 25 | 25 | `com.velocitypowered:velocity-api:4.2.0` |
| A10 | — （代理端不锁 MC） | bungeecord | **17** | 21 | `net.md-5:bungeecord-api:26.1-R0.1-SNAPSHOT` |

要点（每条都来自已确认决定，不重新讨论）：

1. **1.8.9 不等价于 Paper 1.8.9**：Paper 从 1.9.4 起；1.8.9 只能走 `bukkit` 模块（`spigot-api:1.8.8`，因为**没有** `1.8.9` 制品）。
2. **CI 的 JDK ≠ 生成物的 toolchain**：Gradle 9.8 要求 JVM 17–27，所以 Java 8 目标用 JDK 21 跑 Gradle + `options.release = 8`；这是研究里**实测过**的路径（Gradle 9.7.1/21 → major 52）。
3. **第 3 列是"生成物默认 toolchain"**（矩阵 `java_recommended`，无推荐值时取 `java_min`）。1.16.5 的推荐值是 Java 16（非 LTS、CI 难装），因此**允许**把 A3 的生成物 toolchain 降到 8；两种取值都必须进 CI 矩阵（见 §3 的 `toolchain-override` 维度）。
4. A6/A7/A8 是"每个平台至少一条"的兜底行：`folia` 取它唯一稳定锚点 26.2，`sponge`/`minestom` 只有现代窗口（minestom 全部可用版本 = 1.21.11 / 26.1.1 / 26.1.2 / 26.2）。
5. A10 取当前发布的 BungeeCord 快照线（Java 17）；`1.21-R0.4`（Java 8）是次要有力组合，进 best-effort 清单而不是保证行。

### 2.1 同为"保证但降级"的组合

`--no-quality`（关质量工程）与 `--no-example`（空骨架）在 A1（1.8.9 bukkit）上各跑一条 A 级子行，证明"关掉后仍能构建"；`1.16.5` 的 toolchain=11/16、A1–A5 的 `--metadata paper-plugin` 变体、A1–A10 的 `--lang en|both` 均为 **B 级**（只验证渲染与依赖解析，不额外跑 build）。

## 3. vinoa 自己的 CI

`.github/workflows/acceptance.yml`（**vinoa 仓库**的工作流，不是生成物的）。

| 维度 | 取值 |
|---|---|
| Runner | `ubuntu-latest`（必跑）、`windows-latest`（**必跑**，Windows 是官方支持环境）、`macos-latest`（仅 `--dry-run` + 渲染检查，不跑 Gradle） |
| Gradle 运行 JDK | `17`、`21`、`25` 三个（17 覆盖 Gradle 下界；LTS 用 Temurin） |
| 生成物 toolchain | 由矩阵推导；额外扫 `8 / 11 / 16 / 21 / 25` 作为覆盖维度 |
| 元数据格式 | `plugin.yml`（保证行默认）+ `paper-plugin`（现代行至少 2 条） |
| 语言 | `zh`（默认）+ 每 PR 至少 1 条 `en`、1 条 `both` |
| 真跑 Gradle 的行 | §2 的 A1–A10 × `ubuntu` = 10 个 job；`windows` 上抽 A1 / A4 / A5 / A10 共 4 个 job |
| 其余组合 | `--dry-run` + 渲染 + `libs.versions.toml` 解析 + 静态断言（不跑 Gradle） |

时间预算（单 job 硬超时，超时即失败）：

| 环节 | 上限 |
|---|---|
| vinoa 自身 `cargo build` + 单测 | 3 min |
| 生成 + `--verify`（paper/bukkit 行，含配置阶段） | 6 min |
| 生成 + `--verify`（proxy 行） | 4 min |
| 整个 workflow（含并行 fan-out） | 20 min |

实现要点：

- JDK 由 `actions/setup-java` 的 `java-version` 矩阵装好；生成物内的 `foojay-resolver-convention` 在 CI 里**不下网**（`-Porg.gradle.java.installations.auto-download=false`），逼出"本机 JDK 必须在场"的真实路径。
- Gradle 依赖在矩阵前用一次 warm-up 任务灌进 `~/.gradle`，然后 `--offline` 跑保证行；**保证行不允许联网**（否则"保证"依赖上游可用性）。
- 每个 job 产物：`vinoa-report.json`、生成物 `build/reports/**`、失败时的 `*.log` 尾 200 行，作为 artifact 上传。
- 夜间（cron，非 PR 阻塞）额外跑"全矩阵 best-effort"和 `versions --refresh` 后再跑一次，用来**提前发现**上游坐标漂移。

## 4. `--verify` 精确语义（opt-in）

| 项 | 约定 |
|---|---|
| 触发 | 仅当显式传 `--verify`（向导第 ④ 页不出现该开关；`--yes` 不隐含开启） |
| 执行位置 | 生成物根目录（`cwd = <目标目录>`），**不**改环境变量、不清理既有 `build/` |
| 执行命令 | Linux/macOS：`./gradlew build`；Windows：`gradlew.bat build`（走 `cmd`，不做 WSL 假设） |
| 附加 flag | `--offline`（默认开，除非 `--verify-online`）；`--no-daemon`；`--console=plain`；`--stacktrace` 只写日志不进 stdout |
| 环境 | 强制继承本机 JDK；未装所需 JDK 时**不**自动下载（下载只由生成物里的 foojay 决定，且 `--yes` 下保守为关） |
| 超时 | 默认 **600 s**（`VINOA_VERIFY_TIMEOUT` 可覆盖，硬上限 1800 s）；超时 → `reason="timeout"`，杀进程树 |
| 离线 | `--offline` 只在 Gradle 缓存已预热时可能成功；失败不区分"离线导致"与"真的构建失败"的**退出码**，但 `reason` 字段区分：`network_unavailable` / `build_failed` |
| 日志 | 永远落盘：`<目标目录>/.vinoa/verify/verify-<UTC时间戳>.log`；stdout 只打印**最后 20 行** + 路径 |
| 失败后 | 工程**保留**、不写 `.vinoa/state`、不做任何回滚；提示"可复现命令" |
| 退出码 | 见下表 |

| 退出码 | 名字 | 何时 |
|---|---|---|
| `0` | `SUCCESS` | 生成成功；带 `--verify` 时构建也通过 |
| `2` | `VERIFY_FAILED` | **生成成功、验证失败**（构建失败 / 超时 / 离线取不到依赖）——本次任务专属退出码 |
| `64` | `USAGE` | 非法 flag / 缺必填参数且无法询问（`sysexits EX_USAGE`） |
| `65` | `DATAERR` | 名称/包名非法，或 `(mc, platform)` 组合不存在（`EX_DATAERR`） |
| `73` | `CANTCREAT` | 目标目录非空且未给 `--output`，或无写权限（`EX_CANTCREAT`） |
| `74` | `IOERR` | 落盘中途失败，已回滚（`EX_IOERR`） |
| `75` | `TEMPFAIL` | JDK 缺失且用户拒绝自动下载、且**继续生成**不允许时；或渲染依赖的临时资源不可用（`EX_TEMPFAIL`） |
| `78` | `CONFIG` | 内置模板/矩阵损坏，或 `-c` 配置文件语法错（`EX_CONFIG`） |
| `130` | `INTERRUPTED` | Ctrl-C / SIGINT：原子落盘已回滚，退出码按 POSIX 128+2 |

> 老版本特有：对 A1（1.8.9），`--verify` 的提示文案里必须写明"该版本**起不了本地 Paper 测试服**"（Paper 无 1.8.9），但 `build` 仍然会跑。

## 5. 失败如何上报

### 5.1 人可读

```
✗ 生成成功，验证失败（exit 2）
  工程:      /root/projects/my-plugin
  命令:      ./gradlew build --offline --no-daemon
  退出码:    1
  原因:      build_failed
  日志:      .vinoa/verify/verify-20260925T102233Z.log
  复现:      cd /root/projects/my-plugin && ./gradlew build

  --- 日志尾部 20 行 ---
  > Task :platforms:paper:compileJava FAILED
  error: cannot find symbol  …
```

规则：**先给动作（复现命令 + 日志路径），再给日志尾部**，绝不只打印一句"构建失败"。日志尾部行数 `--verify-tail <N>`（默认 20，上限 200），第 1 节 §3 的 CI artifact 固定 200 行。

### 5.2 `--json` 形状（所有路径同构；`vinoa.init/v1`）

```json
{
  "schema": "vinoa.init/v1",
  "vinoa": "0.1.0",
  "generated_at": "2026-09-25T10:22:33Z",
  "status": "verify_failed",
  "exit_code": 2,
  "project": {
    "name": "my-plugin", "path": "/root/projects/my-plugin",
    "package": "com.example.myplugin", "lang": "zh",
    "mc": "1.21.11", "java_target": 21, "gradle": "9.8.0",
    "platforms": ["paper", "bukkit"], "metadata": "plugin.yml",
    "features": [], "quality_engineering": true, "git": {"initialized": true, "branch": "main"}
  },
  "precheck": {
    "jdk": [
      {"module": "platforms:paper", "required": 21, "found": true, "path": "/usr/lib/jvm/java-21-openjdk"},
      {"module": "platforms:bukkit", "required": 8, "found": false, "path": null}
    ],
    "auto_download": { "enabled": true, "plugin": "org.gradle.toolchains.foojay-resolver-convention" }
  },
  "template_source": "builtin",
  "files": {"count": 41, "written": 41},
  "verify": {
    "ran": true, "ok": false, "reason": "build_failed",
    "command": "./gradlew build --offline --no-daemon",
    "timeout_s": 600, "duration_ms": 91234,
    "log_path": ".vinoa/verify/verify-20260925T102233Z.log",
    "tail": ["…最后 20 行…"]
  },
  "warnings": [],
  "errors": [{"code": "verify_failed", "message": "Gradle build failed (exit 1)"}]
}
```

- 机器判定只看 `exit_code`（与人类路径**同一套**），`status`/`reason` 只做分类，便于 agent 决定重试策略。
- 未跑 `--verify` 时 `verify = {"ran": false, "ok": null, "reason": null}`；`--dry-run` 时 `files` 变为 `planned`，`git` 与 `verify` 恒为 `ran:false`。

## 6. 负向验收（错误质量本身就是验收项）

三个硬性要求：**说清发生了什么**、**给出下一步动作**、**给出可复现命令**。禁止"操作失败"式文案。

### 6.1 不支持的 `platform × version`

```
$ vinoa init my-plugin -m 1.8.9 --platform paper -y
✗ 平台与版本组合不存在：paper × 1.8.9
  原因: Paper 最早的构建是 1.8.8；1.8.9 没有 Paper/paper-api 制品
  1.8.9 可用平台: bukkit, sponge
  该平台可用版本: 1.9.4, 1.10.2, 1.11.2, 1.12.2, … , 26.2
  复现: vinoa versions --matrix --mc 1.8.9
exit 65
```

- 交互路径：**根本选不到**（选择框只列支持项），所以这条只对非交互/AI 路径有意义。
- 列出"该 MC 可用平台"与"该平台可用版本"两条**都**要给（矩阵 `--json` 已带），绝不静默换版本或裁掉平台。
- 非交互下 `--yes` 不做任何补救推断。

### 6.2 目标目录非空

```
$ vinoa init my-plugin -m 1.21.11 --platform paper -y     # my-plugin/ 已有 src/、pom.xml
✗ 目标目录已存在且非空: /root/projects/my-plugin
  冲突项（前 10 条）: pom.xml, src/, .idea/
  处置选项:
    --output <other-dir>     换一个空目录
    cd <dir> && vinoa init    确认"就地初始化"意图（仍会逐文件询问）
  复现: vinoa init my-plugin -m 1.21.11 --platform paper --output /root/projects/my-plugin-2 -y
exit 73
```

- 默认**不覆盖**、不合并、不跳过；`--dry-run` 同样按此判定（"预览即实际"）。

### 6.3 缺少所需 JDK

```
本机环境预检:
  Java 21  ✓ /usr/lib/jvm/java-21-openjdk   （platforms:paper 需要）
  Java 25  ✗ 未检测到                        （platforms:velocity 需要）

? 缺少的 Java 25 是否让 Gradle 首次构建时自动下载？ (Y/n)
```

| 用户选择 | 结果 |
|---|---|
| `y` | 生成物 `settings.gradle.kts` 写入 `foojay-resolver-convention`；`precheck.auto_download.enabled = true`；退出码仍为 `0` |
| `n` | 不加插件；打印"构建前需自行安装 Java 25，否则 `./gradlew build` 会失败"；**不阻塞生成**，退出码 `0` |
| 非交互 + `--no-download-jdk`（`--yes` 的保守默认） | 同上，且 `warnings[]` 里带 `missing_jdk` |
| 非交互 + `--verify` + JDK 缺失 | 预检阶段直接失败，`exit 75`，**不生成半个工程** |

检测顺序固定：`JAVA_HOME` → PATH 上的 `java -version` → 常见安装目录扫描（Linux `/usr/lib/jvm`、Windows `Program Files\Java` 与 Adoptium、macOS `/Library/Java/JavaVirtualMachines`、SDKMAN/asdf）。走该顺序而非 Gradle 进程（慢且离线不可用）。

### 6.4 其它负向项（点名验收）

| 负向场景 | 期望 |
|---|---|
| `versions --refresh` 联网失败 | 回退内置矩阵 + 警告，**不阻塞**；`template_source = "builtin"` |
| 落盘中途 SIGINT | 原子回滚，无残留目录，`exit 130`；`--json` 仍能完整输出（写到 stderr 的旁路） |
| `-c config.toml` 语法错 | 指出行/列 + 期望的键；`exit 78` |
| Windows 下路径含空格 / 中文 | 生成、`--verify`、`git init` 全部成功（原生路径与编码，不回退 WSL） |
| `--dry-run` 于非空目录 | 与真实运行**同**判定、同退出码 |

## 7. 我们**不**承诺

- **不承诺** best-effort 层的任何组合能构建通过；也不承诺它们是稳定接口（可能随上游坐标变化而失效，仅保证"要么渲染成功、要么硬报错"）。
- **不承诺**在任何环境都能构建：首次构建需要网络（Gradle 发行包、平台 API、插件）；离线只在缓存预热后成立。
- **不承诺** 1.8.9 能起本地测试服（Paper 无 1.8.9；`runServer` 在该目标下不可用或不可靠）。
- **不承诺**旧版构建路径的长期寿命：JDK 25 上 `--release 8` 已带"obsolete"警告并将被移除；届时 Java 8 目标会先在 CI 里变成红色再谈支持。
- **不承诺**跨平台交叉构建（Windows 上跑 Linux 目标等），也不承诺 CI 在 Windows 覆盖全部 A 行（抽样 4 行，其余 best-effort）。
- **不承诺** NMS/`paperweight`、`libraries:` 在 <1.16.5、`api-version` 在 <1.13 等现代能力的降级等价物（矩阵能力位已明确标出）。
- **不承诺** `--verify` 的耗时下界：超时是设计内的失败模式（`reason="timeout"`），不是缺陷。
- **不承诺**生成物格式稳定：模板与文件清单仍会在 1.x 内演进（见 `template-content.md`）。

## 待定 / 风险

1. **A3 的生成物 toolchain 究竟写 8 还是 16**：Java 16 非 LTS、`actions/setup-java` 能装但生态罕见；倾向写 8、把 16 降为 B 级，待用户拍。
2. **`exit 2` 是否会和未来命令冲突**：当前只有 `init` 用到；若加 `build` 命令需重新分配（可能改为 `128+N` 区间或 sysexits 之外的独立表）。
3. **保证行"离线"的可行性**：`spigot-api:1.8.8` 的传递依赖 `net.md-5:bungeecord-chat:1.8-SNAPSHOT` 只能从 Spigot `public` 或 `repo.papermc.io` 取；CI warm-up 是否稳定命中这两个源、是否要自带镜像未验证。
4. **Windows 上 Gradle wrapper + 非 ASCII 路径**未实测（研究阶段没有 Windows 环境）；这是 §3 "Windows 必跑"的直接风险。
5. **时间预算是否现实**：6 min/行含冷启动配置阶段；若 CI 无缓存，paper 行的实测时间需要第一轮 CI 数据后再收紧。
6. **Sponge/Minestom/Folia/bungeecord 的"保证行"是否配得上"保证"二字**：它们的组合窗口窄、上游发布节奏不同于 Paper；是否应整体降为 B 级、只保留"渲染 + 依赖解析"验证，待用户决定。
7. **`--verify-tail` 与 `--json` 的字段冻结时机**：字段一旦被 agent 依赖就很难改；建议在 ticket #11 定稿时冻结 `vinoa.init/v1`。
