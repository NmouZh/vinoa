# CONTEXT.md —— vinoa 领域术语表

> **口径**：本仓库是 **single-context**（见 [`docs/agents/domain.md`](docs/agents/domain.md)）：仓库根一份 `CONTEXT.md` + `docs/adr/`，**没有** `CONTEXT-MAP.md`。
> **用法**：在 issue 标题、重构提案、假设、测试名里提到领域概念时，用本表的词，不要漂到本表明确避免的同义词（见 §4）。
> **取材规则**：所有术语**只从已存在的文档与代码抽取**，不发明新概念。权威来源一栏给的是**唯一事实来源**；冲突时以 [`docs/spec/vinoa-cli.md`](docs/spec/vinoa-cli.md)（正文）为准，产品裁定看 [`docs/spec/drafts/rulings.md`](docs/spec/drafts/rulings.md)。
> **标注约定**：来源带「**本次 effort 产出**」的，指该文件属于当前界面 effort（#19–#28）。本表写作时部分文件尚未落盘，故标注了预期路径；**现已全部落盘**（`docs/spec/ui/` 下 direction / tokens / motion / degradation / shell / wizard-1-2 / wizard-3-4 / output-surfaces 八份）。本文只引用它们，不修改。

---

## 1. 核心术语

| # | 术语 | 一句话定义 | 权威来源 |
|---|---|---|---|
| 1 | **生成物**（generated project） | vinoa 落盘产出的那棵 Minecraft 插件工程（Gradle Kotlin DSL、多模块），目标是 `./gradlew build` 直接绿。 | spec §1、§8 |
| 2 | **模板集**（template set） | 生成物的来源：一棵模板树 + 根下的 `vinoa-template.toml` 清单；内置模板集由 rust-embed 编译进二进制，也可整套替换。 | spec §2、§7.8、§7.9 |
| 3 | **计划（plan）** | §5 阶段 4 产出的完整文件清单 + 内容 + 额外动作；`--dry-run` 打印的就是它，落盘**只执行它**（"预览即实际"）。 | spec §2、§5、`src/template/plan.rs` |
| 4 | **相位 G / 相位 B** | G = generate-time（vinoa 用 minijinja 渲染并落盘定型）；B = build-time（Gradle `processResources`，**只展开 `version`**）。 | spec §2、§7.2、§7.3 |
| 5 | **能力开关**（`cap_*` / `is_*` / `has_*`） | Rust 侧把原始变量折叠成的布尔量；模板与清单**只认开关**，分支不得直接判 `mcVersion` / `platforms`。 | spec §2、§7.5；折叠实现 `src/template/vars.rs` |
| 6 | **版本矩阵**（matrix） | 内置的版本事实**唯一来源**：平台坐标、Java 目标、Gradle/插件地板、三方库版本；"上移支持窗口 = 更新矩阵数据，不是改代码"。 | spec §6（尤其 §6.1 数据模型、§6.3 查询语义、§6.4 离线/联网）；数据 `data/version-matrix.toml` |
| 7 | **A/B/C 承诺分级** | A = vinoa CI 每个 PR 真跑 `./gradlew build`（只覆盖 `paper`/`bukkit`/`velocity`/`bungeecord` 代表组合）；B = 尽力而为（渲染 + 依赖解析 + 静态断言，**不保证**构建通过）；C = 组合本身不存在（硬报错列可用项，绝不静默裁剪）。 | spec §1.1、§12.1、§12.2、§12.5；裁定 `rulings.md` B3 |
| 8 | **`--verify`** | opt-in 开关：生成成功后在生成物根目录跑一次 `./gradlew build`（默认离线），是**唯一**把"生成成功"与"构建成功"绑起来的手段；失败 = 工程保留 + 日志落 `.vinoa/verify/` + `exit 2`。 | spec §3.2、§11.4、§11.2；`src/verify.rs` |
| 9 | **向导四页** | TTY 下的交互向导，形态对齐 IDEA"新建项目"：① 工程 / ② 构建 / ③ 目标服务端 / ④ 附加；页间可回退，`--yes` 与非 TTY 全走默认值。 | spec §4（§4.1 ①②、§4.2 ③、§4.3 ④、§4.4 交互约定）；逐页视觉规格 [`docs/spec/ui/wizard-1-2.md`](docs/spec/ui/wizard-1-2.md)、[`docs/spec/ui/wizard-3-4.md`](docs/spec/ui/wizard-3-4.md)（**本次 effort 产出**，#26/#27） |
| 10 | **共享外壳**（shell） | 向导四页**之外**、一次 `init` 里用户连续经历的那条非向导输出面：环境预检 / 计划摘要 / 完成横幅 / 错误路径。 | [`docs/spec/ui/shell.md`](docs/spec/ui/shell.md)（**本次 effort 产出**，#24）；现状 `src/init.rs::print_precheck`、`print_done`，`src/report/mod.rs::plan_human`、`error`；原型 `docs/wayfinder/prototypes/shell.png`（`python3 docs/wayfinder/prototypes/render.py shell`） |
| 11 | **渲染方案** | 向导的呈现底座。已裁定：**改用 `ratatui 0.29 + crossterm` 全屏接管**，`inquire` 移出向导路径；**无色 / 非 TTY / 窄终端降级由 vinoa 自己保证**，不依赖 ratatui 提供。 | [`docs/adr/0001-render-stack.md`](docs/adr/0001-render-stack.md)（#21，已落盘，`status: proposed`）；输入见 [`docs/spec/ui/direction.md`](docs/spec/ui/direction.md) §4（保留预览面板 ⇒ 需要全屏接管）与 [`docs/research/terminal-render-capabilities.md`](docs/research/terminal-render-capabilities.md) §3.2、§8；降级契约 [`docs/spec/ui/degradation.md`](docs/spec/ui/degradation.md) |
| 12 | **设计 token** | 界面样式的最小完备常量集合（色值 / 字形 / 间距 / 层级）；token 服务方向锁定，**颜色只承载语义、不做装饰**。 | [`docs/spec/ui/tokens.md`](docs/spec/ui/tokens.md)（**本次 effort 产出**，#22）；语义位清单见 `direction.md` §4；方向 `direction.md` §1、§3 |
| 13 | **降级层级** | 颜色能力逐级放弃的档位：真彩 → 256 → 16 → 无色；每一档明确"放弃什么、保留什么语义"。 | [`docs/spec/ui/degradation.md`](docs/spec/ui/degradation.md)（**本次 effort 产出**，#25）；探测事实见 `docs/research/terminal-render-capabilities.md` §1、§6 |
| 14 | **无色语义** | 颜色被禁用时靠**字符与结构**保持信息可辨（失败靠 ✗、当前项靠 ❯、次要信息靠缩进而非灰），而不是"变淡即次要"。 | `docs/spec/ui/degradation.md`（**本次 effort 产出**，#25）；原则见 `direction.md` §3（"颜色只承载语义"）与 `docs/research/terminal-render-capabilities.md` §1 |
| 15 | **绘制模型** | 一屏只在**同一张网格**上绘制一次：vinoa 自己的 `eprintln!` 与 prompt 必须落在同一网格，禁止把同一块 UI 叠印多遍。 | [`docs/spec/ui/motion.md`](docs/spec/ui/motion.md)（**本次 effort 产出**，#23）；方向 `direction.md` §1、§4；根因 `docs/research/terminal-render-capabilities.md` §3.2（"样式层改不掉"） |

## 2. 支撑术语

| # | 术语 | 一句话定义 | 权威来源 |
|---|---|---|---|
| 16 | **MC 版本 / 平台** | MC 版本是 `1.8.9` / `1.21.11` / `26.2` 这类发布号（日期式与 `1.x.y` 同轴）；平台是目标运行环境，分**服务端平台**（paper/bukkit/folia/sponge/minestom）与**代理端**（velocity/bungeecord，不锁 MC 版本、恒可选）。 | spec §2、§1（7 个平台清单）、§6.3.5 |
| 17 | **目标目录** | `vinoa init <name>` → `./<name>`；`vinoa init` → 当前目录（就地初始化）。非空即报错，默认**不覆盖、不合并、不跳过**。 | spec §2、§3.2、§11.1 |
| 18 | **core 边界** | `core/` 是平台无关层（抽象接口 + 纯逻辑），**禁止 import 任何平台 API**；其 Java 目标 = 所有已启用模块中最低的那个。 | spec §8.2、§8.1、§7.11 R5 |
| 19 | **变量契约** | 生成期变量一律 **camelCase**，与 `-c/--config` 的 TOML 键同名同义、不做别名；派生量在 Rust 侧算好（"只读"列）。 | spec §7.1（§7.1 表末的固定推导顺序） |
| 20 | **渲染后自检（A 断言）** | 渲染完成、写盘之前跑的一组静态断言（无残留占位符 / 无模板集默认身份串 / 主类可寻 / 变量集合双向核对 / 模块集合一致），`--dry-run` 也执行。 | spec §7.10；实现 `src/template/plan.rs`（注意 §5 的编号口径差异） |
| 21 | **原子落盘** | 先写临时目录，再整体 `rename`；中途失败**回滚、无残留**，绝不留下半个工程。 | spec §5 阶段 6、§11.1、§12.4 |
| 22 | **可复现（字节相同）** | 同一 CLI 版本 + 同一输入 → **字节相同**的文件树；时间戳、年份、随机种子、机器路径一律不得进入产物。只约束**生成阶段**（阶段 0–6），不约束构建阶段。 | spec §5、`rulings.md` C2 |
| 23 | **不静默原则** | 组合不存在 → 硬报错并列出可用项；缺 Java → 询问；缺 bStats id → 硬报错。绝不静默裁剪、绝不假 id 兜底。 | `rulings.md` C3；spec §6.5、§11.1 |
| 24 | **`--dry-run` / "预览即实际"** | 跑完校验 / 矩阵 / 预检 / 渲染 / 自检，但一个文件都不写、不 `git init`；与真实运行**同判定、同退出码**。 | spec §3.2、§5、§12.4 |
| 25 | **`--json` 单文档 / 退出码** | `--json` 时 stdout **恰好一个** JSON 文档（人类文本走 stderr）；机器判定只看 `exit_code`，与人类路径同一套；`2 = VERIFY_FAILED` 已冻结。 | spec §11.5、§11.2；`rulings.md` B4；三个输出面（`--help` / `versions` / `schema`）的版式见 [`docs/spec/ui/output-surfaces.md`](docs/spec/ui/output-surfaces.md)（**本次 effort 产出**，#28） |
| 26 | **矩阵来源三态** | `versions` 输出的矩阵来源：`builtin`（默认）/ `cache`（`--refresh` 落的用户缓存）/ `online`（`--online` 单次联网）。`init` 默认**只用内置矩阵**。 | spec §6.4、§3.4；`src/refresh_cache.rs`、`src/matrix/refresh.rs` |
| 27 | **feature 白名单** | 模板集用 `[features] implemented` 声明**真正实现**的能力；`init` 对未实现/缺坐标的 feature **硬报错**（exit 65），不再静默生成。 | `templates/vinoa-template.toml` `[features]`；spec §3.3、§11.6、附录 R15 |
| 28 | **界面语言 vs 生成物语言** | `--ui-lang` 只影响 vinoa 自己的界面与 `message`；`--lang` 只影响生成物的注释 / README / 玩家消息。**两者是独立概念**。 | spec §4、§7.1（`lang`）、§7.7 |
| 29 | **溯源不落盘** | 不生成 `.vinoa.toml` 之类溯源文件；模板 id / commit / 生成时间只出现在 `--json` 与首次提交的 message 里。`.vinoa/` 目录只用于 `--verify` 日志。 | `rulings.md` A5；spec §13、§11.4 |
| 30 | **clean-room** | 参考社区模板的**结构与工程纪律**，**不复制任何代码正文/文本**；vinoa 与生成物均 Apache-2.0；README 必须注明参考来源。 | spec §13；`rulings.md` C4 |

## 3. 三处判定的口径（易混）

| 易混点 | 口径 |
|---|---|
| "生成成功" ≠ "构建成功" | 生成成功只表示计划执行完毕；只有 `--verify` 会把两者绑起来（spec §1.1、§11.4）。 |
| 相位 G 的"定型" | 除 `version` 外，§7.1 的**所有**变量都在 G 相位落盘定型；生成物构建时不再查矩阵、不再联网（spec §7.3）。 |
| "跳过 N 项" | 计划里 `when` 未命中的条目，进 `skipped[]`（`--dry-run` 的 `--json` 里带 `when` 原因）；它不是错误，也不进 `warnings[]`。 |
| experimental 标注 | `folia` / `sponge` / `minestom` 标 experimental，且**不进 A 级承诺**；承诺级别与标注必须一致（`rulings.md` B3、spec §12.1）。 |

## 4. 已废弃 / 明确避免的同义词

| 不要用 | 用 | 依据 |
|---|---|---|
| `common/`（共享模块名） | `core/` | `docs/spec/drafts/project-layout.md` 的「已确认（2026-09-25）」表（"共享模块名 `common/` → **`core/`**"）；spec §2、§8 |
| `messages_*.yml` | `lang/zh_CN.yml` / `lang/en.yml` | spec §7.7（"是同一文件的早期命名"） |
| "模板"泛指用户拿到的东西 | **生成物** | spec §1（vinoa 给的是能直接构建的工程，不是待改的模板文件） |
| Nukkit / PowerNukkitX / Waterfall | —— | spec §1、§14（基岩版版本轴不同；Waterfall 已 EOL） |
| 用"尽力而为"以外的模糊词指 B 级 | **B 级** | spec §1.1、§12.1 |

## 5. 已知口径不一致（观察，不改任何权威文档）

> 以下为抽取术语时发现的**文档/代码/数据之间的不一致**。此处只记录，便于 `/domain-modeling` 或对应 owner 裁定；本文件不改变任何 spec 或代码。

1. **自检断言编号（A-IDs）错位一号，且存在同名冲突。**
   - spec §7.10 表：`A1` 残留占位符 · `A3` 默认身份串 · `A4` 入口类可寻 · `A5` 变量双向 · `A6` 模块集合（**无 A2**）。
   - `src/template/plan.rs`：`A1` 残留占位符 · `A2` 默认身份串 · `A3` 入口类可寻 · `A4` 变量双向 · `A5` 模块集合（**无 A6**）。即代码整体比 spec 少一号。
   - `templates/vinoa-template.toml` 的 `vars` 注释采用**代码**编号（"A4 做双向核对"）。
   - `src/template/vars.rs` 的注释 `ruling A2` 指的是 `rulings.md` 的 **A2（bStats id）**，与断言 `A2` 同名不同义 —— 两套 A 编号在同一个仓库里并存。
2. **版本矩阵数据文件路径：spec 说 `templates/`，实现用 `data/`。**
   - spec §6.1 代码块注释：`# templates/version-matrix.toml（编译进二进制）`。
   - 实现：`src/matrix/builtin.rs` 的 `include_str!("../../data/version-matrix.toml")`，`src/matrix/query.rs` 注释同样写 `data/`。`templates/version-matrix.toml` **不存在**。
3. **命令面数量：spec 说"两个子命令"，实现与 README 有三个。**
   - spec §3 正文："两个子命令：`vinoa init`、`vinoa versions`"，§3.1/§3.4 也只写这两个。
   - 实现有第三个 `vinoa schema`（`src/cli.rs::run_schema`，`"schema": "vinoa.schema/v1"`）；`README.md` 的常用命令列出 `vinoa schema --json`。spec 正文没有 `schema` 章节。
4. **模板清单 schema 键名已演进，spec §7.8 未跟进。**
   - spec §7.8：`[[files]]` + `template` / `target` / `render` / `when` / `foreach`+`paths` / `build_time_vars`；顶层 `[variables.<name>]` / `[conditions]` / `[dependencies]` / `[[assertions]]`。
   - `templates/vinoa-template.toml`（其自身注释声明"schema 见 §7.8"）：用 `[[entries]]`（45 条）、`path` 而非 `target`，并新增 `mode`、`vars`、`name`、`[features] implemented`。
5. **附录 R15 已过期。**
   - spec 附录 R15 称"可选模块模板尚未全部实现（sqlite/bstats/update-check/placeholderapi/gui、integrationTest、spotbugs/coverage/release-ci、`libraries:` 块）——task-7 进行中"，并注明"全部实现后删除本条"。
   - `templates/vinoa-template.toml` 的 `[features] implemented` 已列全 12 项（含上述全部）。`README.md` 仍保留"正在补齐中"的说法。R15 与 README 均落后于当前工作树。
6. **颜色档位：任务口径写"真彩→256→16→无色"，一手事实里 `crossterm` 的探测默认档是 8。**
   - `docs/research/terminal-render-capabilities.md` TL;DR #6 / §1.2：`crossterm::style::available_color_count()` **默认返回 8**（不是 16），探测失败时退到 8 色。
   - 因此"16"这一档需要 #25 在 [`docs/spec/ui/degradation.md`](docs/spec/ui/degradation.md) 里显式裁定，不能从 `crossterm` 的默认行为直接推出。
7. **错误码 `template.feature_unimplemented` 未进任何码表。**
   - 实现：`src/init.rs` 对未实现 feature 硬报错，用码 `template.feature_unimplemented`（`EXIT_DATA` = 65），与术语表 §1 第 27 条「feature 白名单」一致。
   - 但 spec §11.6 的错误码表**没有**这一项；`vinoa schema` 输出的 `error_codes` 数组（`src/cli.rs::run_schema`）也**没有**它。附录 R15 只说"硬报错"，未给码名。

---

## 6. 相关索引

- 规格（唯一事实来源）：[`docs/spec/vinoa-cli.md`](docs/spec/vinoa-cli.md)
- 裁定层：[`docs/spec/drafts/rulings.md`](docs/spec/drafts/rulings.md)
- 事实研究：[`docs/research/`](docs/research)
- 决策记录：[`docs/adr/`](docs/adr)（约定与索引见 [`docs/adr/README.md`](docs/adr/README.md)）
- 本次界面 effort 的产出面：[`docs/spec/ui/`](docs/spec/ui)（direction / tokens / motion / degradation / shell / wizard-1-2 / wizard-3-4 / output-surfaces）
- 工作方式：[`AGENTS.md`](AGENTS.md)、[`docs/agents/`](docs/agents)
