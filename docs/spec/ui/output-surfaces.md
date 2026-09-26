# help / versions / schema 视觉规格（#28）

> **状态**：锁定。三个输出面共用 `tokens.md` 的一套 token；**数值一律引用 token 名，本文不重复定义**。
> **上游**：[direction.md](direction.md)（#19 方向）、[tokens.md](tokens.md)（#22 数值唯一来源）、[motion.md](motion.md)（#23 动效）、[degradation.md](degradation.md)（#25 降级契约）、[0001-render-stack.md](../../adr/0001-render-stack.md)（#21 已定 `ratatui 0.29 + crossterm` 全屏接管向导）。
> **下游**：实现票（本文 §7 的断言表可机械翻译）。
> **本文范围**：`vinoa init --help` / `vinoa versions --help` / `vinoa schema --help`、`vinoa versions`、`vinoa schema`。**不含** `init` 的执行路径（那是 #24 shell）。
> **实测环境**：`./target/debug/vinoa`，`TERM=dumb`、`NO_COLOR=1`（#20 已记录本机盲区）。本文所有「现状」数值均来自该二进制的一次真实运行，命令附在 §8。

---

## 0. 现状实测（决定本节全部结论的事实）

| # | 面 | 现状 | 证据 |
|---|---|---|---|
| 1 | `init --help` | clap 默认渲染 + 中文 `help_heading` 分组（`参数/常用/高级`）+ `after_help`（`示例/退出码`）；**最长行 121 列**（`--features` 行），**9 行超 76 列** | §8 复现 1 |
| 2 | `--help` 的宽度 | **`COLUMNS=40` 与 `COLUMNS=200` 输出逐字节相同**（不换行、不截断）。因为 `clap` 只启用了 `derive` feature，**没有 `wrap_help`**（`Cargo.toml:11`） | §8 复现 2 |
| 3 | `--help` 的流向 | **stdout**（`clap` 的 `DisplayHelp` 分支 `err.print()`，`src/cli.rs:380`）；`2>/dev/null` 仍有输出，`2>&1 >/dev/null` 为空 | §8 复现 3 |
| 4 | `--help` 的着色 | 由 `anstream/anstyle-query` 决定，**发生在 vinoa 的 `UiEnv::detect()` 之前**；`--color` 对它不生效（degradation.md §6.5 已裁定，本文引用不重定义） | degradation.md §6.5 |
| 5 | `versions` | **人类文本全部走 stderr**（`report::info` = `eprint!`，`src/report/mod.rs:70`）；stdout 在无 `--json` 时为**空** | §8 复现 4 |
| 6 | `versions` 的行宽 | `受支持版本:` 单行 **468 列**（61 个版本号）；`--matrix` 最长行 **87 列**（folia + 坐标 + `(experimental)`），**超 76 列的共 23 行** | §8 复现 5 |
| 7 | `versions` 的对齐 | `{p:<11}` 手工对齐（`src/cli.rs:951`）；最长平台名 `bungeecord`(10) ⇒ 11 恰好够，**但该常量不是 token**（`tokens.md` §5.3 的 `name.w = 11` 是同一个数，两处独立硬编码） | §8 复现 5 |
| 8 | `versions` 的坐标 | 最长坐标 **54 字符**（`com.destroystokyo.paper:paper-api:1.10.2-R0.1-SNAPSHOT`） | §8 复现 6 |
| 9 | `schema` | **`schema` 与 `schema --json` 输出逐字节相同**，都是紧凑 JSON 走 stdout；`schema`（无 `--json`）**没有人类形态** | §8 复现 7 |
| 10 | `schema` 的行宽 | 单行，**实测约 1900 列**（无换行） | §8 复现 8 |
| 11 | 三个面的 ANSI | `init --help` / `versions --help` / `schema --help` / `versions` / `versions --matrix` / `schema` 在非 TTY 下 **`\x1b` 计数全为 0**（`anstream` 与 `report` 已做到） | §8 复现 9 |

**三条从现状直接推出的结论**（本文 §1–§3 的骨架）：

1. `--help` 的**宽度行为不受控**（结论 2）：它既不换行也不截断，窄终端下靠终端自己折行 ⇒ 列位错位、长取值被折断。这是本文要修的第一件事。
2. `versions` 的**元信息与主体数据同等重量**（结论 6）：`矩阵来源:` 行与 `受支持版本:` 行都是裸行、同色、同缩进，且后者 468 列。
3. `schema` **只有机器形态**（结论 9）：`--json` 这个 flag 在 `schema` 上是**冗余**的（有无都一样），人读路径根本不存在。

---

## 1. `--help`：决定与版式

### 1.1 决定：**自定义 `help_template`，但只接管「宽度与分组」，不接管「着色」**

**保留**：`clap` 的 `help_heading` 分组机制、`after_help`、`override_usage`、以及既有的 29 项参数顺序（`src/cli.rs:1029-1058` 的 `help_has_spec_sections_and_flags` 测试已冻结该顺序，**不得破坏**）。
**新增**：在 `help_template` 之外，**显式设置 `Command::term_width`**（见 §1.3），把换行行为从"终端决定"改为"vinoa 决定"。

**理由**（三条，都是代价而非偏好）：

| 候选 | 代价 |
|---|---|
| **接受 clap 默认**（否掉） | 结论 2：`COLUMNS=40` 下 121 列的 `--features` 行会由终端硬折行，折出的续行**从第 0 列开始**，与 `常用:` 组其它行错位 ⇒ 表格语义丢失。而 `--help` 恰恰是最可能被窄终端/pager 打开的面。 |
| **完整接管渲染**（否掉） | 要把 29 个参数的 help 字符串、`value_name`、`help_heading`、`conflicts_with` 全部搬出 clap，等于放弃 clap 的参数模型；且 `clap` 仍会先解析、仍会自己打印 —— 除非绕开 `try_get_matches_from`，那会连带改掉退出码 64 的既有路径（`src/cli.rs:377`）。**收益（着色）与代价（重写参数面）不成比例。** |
| **只设 `term_width` + 保留默认模板**（采纳） | 改动最小：一个 `Command::term_width` 调用 + 在 `help_template` 里加一行「取值提示」。宽度可控，分组与顺序零改动。 |

**为什么不接管着色**（对 degradation.md §6.5 的回应）：`--help` 的着色由 `anstream` 在 vinoa 代码运行前完成。要接管必须自己渲染 help 或用 `anstream::AutoStream` 包一层输出——**本文裁定不做**，理由是：

1. degradation.md §6.5 已给出**保证**：`NO_COLOR`（非空）/ `TERM=dumb` / 非 TTY 下 `--help` 零 ANSI，实测为 0。这三个正是 CI 与 pipe 场景的全部。
2. 唯一分叉是 `--color=never` 对 `--help` 不生效。而 `--color=never` 的用户意图是「别给我颜色」，在**交互式** TTY 下 `--help` 仍会着色——这是一处**已知且已声明**的边界（degradation.md §6.5 明写「那是 #28 的决定」）。
3. 本文的决定：**接受该边界，但把它写进 `--help` 的 `after_help` 里不可能**（鸡生蛋），因此改为**登记为已知限制**（§5 第 3 条），并要求 `--help` 的着色**只使用 bold**（`anstyle` 对 heading 的默认行为），不新增任何颜色语义——这样即使着色，`--help` 也不承载「颜色 = 语义」的约定，与 `tokens.md` §6 的层级规则不冲突。

> **一句话**：`--help` 自定义的是**宽度**，不是**颜色**；颜色边界照 degradation.md §6.5 如实保留。

### 1.2 分组与强调：哪些参数突出

**不新增分组，不重排顺序**（`help_has_spec_sections_and_flags` 已冻结）。强调通过**两处最小改动**实现：

| 位 | 现状 | 规格 | token |
|---|---|---|---|
| 分组标题（`参数:` / `常用:` / `高级:` / `示例:` / `退出码:`） | 裸文本，无标记 | **保留裸文本**（clap 的 heading 已由 `anstyle` 加粗；vinoa 不再叠加） | — |
| **必填 / 高频参数** | 与其它参数完全同级 | 在 `常用:` 组的**首行之前**插一行极次要说明：`  最常用：只需 -m 与 --platform 即可生成` | `faint`（无色档 → `· ` 前缀，见 §4.2） |
| `--features` 的取值列表 | 内联在 help 字符串里，**121 列** | **移出 help 字符串**，改为 `after_help` 里的一张独立取值表（§1.4） | `faint` |
| `--platform` 的取值列表 | 内联，**97 列** | 同上 | `faint` |

**为什么用「一行 faint 说明」而不是给参数加标记**：`tokens.md` §6 的硬约束是「相邻两级之间至少有一个非色轴」。若给 `-m` / `--platform` 加 `❯` 或加粗，它们会与**参数名列**形成第二套层级，而 `--help` 的列位是 clap 管的、vinoa 改不动；新增标记会破坏 clap 的两列对齐。一行说明不触碰列位，且无色档下 `· ` 前缀即可承载。

### 1.3 宽度：显式 `term_width`，分四档

**规格**：`Command::term_width(w)` 的取值 = `min(detect_width(), 100)`，其中 `detect_width()` 复用 degradation.md §5.4 的探测（`crossterm::terminal::size()` → `COLUMNS` → `screen.w.full`）。

| 档 | `W` | `--help` 形态 |
|---|---|---|
| `full` | ≥ 100 | 现状形态（参数名与 help 分两列，clap 默认缩进）；`term_width = 100` |
| `no-preview` | 80–99 | 同上（`--help` 无预览面板可丢，**两档形态相同**）；`term_width = W` |
| `no-border` | 60–79 | 同上；`term_width = W` |
| `single-column` | 40–59 | **参数名单独一行，help 文本续行缩进 `indent.unit`**（clap 的 `next_line_help` 行为）；`term_width = W` |
| `minimal` | < 40 | 同 `single-column`，`term_width = 40`（**下限**，不再收窄） |

**`minimal` 档取 40 而不是 `W` 的理由**：`term_width` 太小会让 clap 把每个词单独折一行，输出行数爆炸且完全不可读；`40` 是 degradation.md §5.1 算出的「单栏最小可用宽度」，取它作为下限，代价是**在 < 40 列终端上允许横向溢出**——这与 degradation.md §5.1 的契约 (b)「不横向滚动」冲突，**因此本文把它列为对 #25 的一处显式豁免申请**（§5 第 1 条），而不是默默违反。

**`--help` 不受 `COLUMNS` 之外的宽度源影响**：`--help` 在 `UiEnv::detect()` 之前打印（§0 结论 4），因此它**读不到** `UiEnv.width`。规格要求把 `detect_width()` 做成**可独立调用的纯函数**（degradation.md §5.4 已是），在 `cli::command()` 里调用一次——**不经过 `UiEnv`**。

### 1.4 长取值列表：移出 help 字符串，进 `after_help`

**现状问题**：`--platform` 的 7 个取值 + `--features` 的 8 个取值内联在 help 字符串里，产生 97 / 121 两行超长文本（§0 结论 1）。窄终端下被折断后，用户**读不出到底有哪些合法取值**。

**规格**：两个参数的 help 字符串**只留一句功能描述**，取值表移到 `after_help`，作为独立的两行：

```
取值:
  --platform   paper bukkit velocity bungeecord folia sponge minestom
  --features   sqlite bstats update-check placeholderapi gui spotbugs coverage release-ci
```

- 每行以 `--platform` / `--features` 开头，后接**空格分隔**的取值（不用逗号——逗号在窄终端折行时容易被误读为行尾）。
- 该两行的**行宽上限 = 72 列**（`term_width` 的 72% 取整）。若某行超过，按取值**边界**折行，续行缩进 `indent.unit`，**绝不在取值中间断开**（取值是「hard：标识性内容」，degradation.md §5.3）。
- 取值顺序**照 `PLATFORMS` 与 `FEATURE_NAMES` 常量的声明顺序**（`src/cli.rs:17-30`），不手工重排——这样常量与文档不会漂移。

**被否掉的两种做法**：

| 否掉 | 理由 |
|---|---|
| 保留内联、只靠 `term_width` 折行 | 折行点由 clap 按词边界决定，会把 `update-check` 这类取值**折成两行**（它在 help 文本里是 `sqlite,bstats,update-check,…` 的一个逗号段，clap 视整段为一个词时不断，视为多词时断在连字符处）——取值被折断即不可复制粘贴。 |
| 把取值表放进 `vinoa schema` 而不进 `--help` | 用户在敲 `--help` 时拿不到合法取值，必须再跑一条命令；`--help` 的核心用途正是「我现在能填什么」。 |

> **与 spec §3.1 的关系**：spec §3.1 的 `--features <列表>` 写「见 §3.3」，`--platform <平台>` 内联 7 个取值。本文**不删** `--platform` 的内联（spec 正文有它），而是**同时**在 `after_help` 里给一张可读的表——两处信息一致，`--help` 因此更长但可读。spec 正文的修订建议见 §5 第 2 条。

### 1.5 `--help` 的完整版式样例（`full` 档，无色）

```
用法: vinoa init [名称] [选项]

参数:
  [名称]  工程名；省略时取当前目录名（就地初始化）

常用:
  最常用：只需 -m 与 --platform 即可生成
  -p, --package <包名>   Java 包名（默认 com.example.<名称去连字符>）
  -m, --mc <版本>        目标 Minecraft 版本（如 1.21.11 / 26.2）
      --platform <平台>  目标平台，可重复（取值见下）
      --features <列表>  附加模块，逗号分隔（取值见下）
  -o, --output <目录>    输出到指定目录（目标目录非空时的首选处置）
  -y, --yes            全部用默认值，不询问
      --dry-run        只打印将要生成的内容（走同一条渲染路径）
      --json           以 JSON 输出（给脚本 / agent 解析），恰好一个文档
      --verify         生成后跑一次构建验证（默认离线）

高级:
  （…29 项，顺序与现状逐字不变…）

取值:
  --platform   paper bukkit velocity bungeecord folia sponge minestom
  --features   sqlite bstats update-check placeholderapi gui spotbugs coverage release-ci

示例:
  vinoa init my-plugin -p com.example.myplugin -m 1.21.11 --platform paper --features sqlite,bstats -y

退出码: 0 成功 · 2 验证失败 · 64 用法 · 65 数据 · 73 无法创建 · 74 IO · 75 临时失败 · 78 配置 · 130 中断
```

**与现状的差异只有三处**（便于评审）：新增 `最常用：…` 一行；`--platform` / `--features` 的 help 文本改为「（取值见下）」；新增 `取值:` 段。

### 1.6 `versions --help` / `schema --help`

两者现状已符合规范（`tokens.md` 无需介入：无分组、无长取值）。**规格**：

- `versions --help`：保持现状（`用法:` + 6 个选项）。`--refresh` 的 help 已提「写入用户缓存目录」，不动。
- `schema --help`：保持现状，但**必须补一行说明人读形态**（§3 新增）：`      不带 --json 时打印人类可读摘要`。理由：§3 引入人读形态后，`--json` 与不带 `--json` 的行为**不再相同**，help 必须说清。
- 两者同样受 §1.3 的 `term_width` 约束。

---

## 2. `versions`：版式

### 2.1 决定：元信息降级为一行 `faint`，主体数据升为两级

**现状**（§0 结论 5、6）：`矩阵来源: …` 与 `受支持版本: …` 都是裸 `eprintln!`、同色同缩进，且后者 468 列。元信息（来源 + 时间戳）与主体数据（版本号）**视觉重量完全相同**。

**规格**：三层，权重递减，全部用 token 名表达：

| 层 | 内容 | 版式 | token | 无色替代 |
|---|---|---|---|---|
| **L1 主体** | MC 版本号 / 平台行 | 无前缀、起始列 `col.content.left` | `fg` | 默认前景（不发色码） |
| **L2 标签** | `矩阵来源` / `受支持版本` / `取值` 等行首标签 | 标签在 `col.content.left`，值在 `col 16`（`label.w = 13`） | `dim` | 列位承担（标签列 vs 取值列） |
| **L3 元信息** | `generated_at`、来源三态、`(experimental)` | 行内 `·` 分隔，或右对齐到 `col.form.right` | `faint` | `· ` 前缀 + 缩进 |

**来源与时间戳的具体形态**（把 L2/L3 落地）：

```
矩阵来源  builtin · 2026-09-25
受支持版本  1.8.9 · 1.9 · … · 26.2（共 61 个）
```

- 标签 `矩阵来源` 用 `dim`，值 `builtin` 用 `fg`，`· 2026-09-25` 用 `faint`——**时间戳不再是括号里的同级文本**，而是行尾极次要信息。
- `（generated_at 2026-09-25）` 的括号去掉，改 `·` 分隔（`tokens.md` §4 的 `sep`；无色档 → `-`）。
- 来源三态 `builtin` / `cache` / `online` 的值本身**不着色**：它不是状态，是事实（`tokens.md` §6「颜色只承载语义」——没有语义对应「来源」，故不给颜色）。

### 2.2 `受支持版本` 的长列表：折叠 + 计数

**现状**：61 个版本号铺成一行 468 列（§0 结论 6）。

**规格**：**`≥ 8` 个时折叠为「首 · 次 · … · 末（共 N 个）」**；`< 8` 个时全部列出。

| 元素 | 规格 |
|---|---|
| 折叠阈值 | 列表元素数 `≥ 8` |
| 折叠形态 | `首 · 次 · … · 末` + `（共 N 个）` |
| 分隔符 | `·`（`sep` token；无色 → `-`） |
| `（共 N 个）` | `faint`；**右对齐到 `col.form.right`**（宽度不足时紧随列表，中间至少 2 空格） |
| 指路命令 | 折叠时在下一行给一条 `faint` 行：`  · 全表: vinoa versions --matrix` |

**为什么阈值是 8**：`tokens.md` §5.1 的 `col.form.right = 63`，减去标签列 `label.w = 13` 与起始列 3 ⇒ 可用 47 列。版本号平均 6 字符 + ` · ` 3 字符 = 9 列/项 ⇒ 47 / 9 ≈ 5 项；取 8 是因为**8 项时已超宽**、且 8 是「人一眼能扫完」的上界（`direction.md` §2.3 的密度口径）。**阈值 8 是一个可断言的常量**（§7 A11）。

**为什么不按宽度动态决定折叠点**：动态折叠会让「同一命令在不同终端下给出不同信息量」，违反 `tokens.md` §3.5「降级只允许换轴，不允许减信息」的**精神**——折叠是减信息。固定阈值 + 恒定的「共 N 个」把**信息量固定**，只有版式随宽度变。

### 2.3 `--matrix`：坐标列与 `(experimental)`

**现状**：`{p:<11} java N  <坐标>` + `  (experimental)` 后缀，最长 87 列（§0 结论 7、8）。

**规格**（把 §2.1 的三层落到 `--matrix`）：

| 列 | 内容 | token | 说明 |
|---|---|---|---|
| 1 | MC 版本号（独立一行） | `fg` | 起 `col.content.left` |
| 2 | 平台名，宽 `name.w`（= `tokens.md` §5.3 的 11） | `fg` | **改为引用 token，不再硬编码 `{p:<11}`** |
| 3 | `java N` | `dim` | 现状是裸文本；改 `dim` 后与平台名形成 L2↔L3 的列位轴 |
| 4 | Maven 坐标 | `fg` | **hard 类内容，绝不截断**（degradation.md §5.3） |
| 5 | `experimental` 标记 | `faint` | 见下 |

**坐标不撑爆宽度**：坐标是 hard 内容 ⇒ **换行，续行缩进 `indent.unit`**，**不截断**（degradation.md §5.3 硬规则 1/2/4）。规格给出：

```
  folia       java 21  io.papermc.paper:paper-api:1.21.11-R0.1-SNAPSHOT
                       · 实验性
```

- 宽度足够（`W ≥ 坐标末列`）时，标记**同行右对齐到 `col.form.right`**。
- 宽度不足时，标记**另起一行**，缩进 = 平台名末列 + 1，前缀 `· `（`faint` 的无色形态）。
- 坐标本身若仍超 `W`：按 degradation.md §5.3 的 hard 规则**换行到续行**（续行缩进 `indent.unit`），**不出现 `…`**。

**`(experimental)` 标记的呈现**（三个决定）：

1. **改词为 `实验性`**（跟随 `--ui-lang`，英文 `experimental`）——现状是硬编码英文 `(experimental)`，与中文界面其余部分不一致。**注意**：`--json` 里的字段名 `experimental` **不变**（§6）。
2. **去掉圆括号**：括号让它看起来像 help 文本的一部分；改为 `· 实验性` 或右对齐的 `实验性`，与 §2.1 的 L3 一致。
3. **不着色为 `warn`**：`tokens.md` §1 只有 8 个语义位，`warn` 的语义是「警告」（如 JDK 缺失）。`experimental` 是**事实标注**不是警告 ⇒ 用 `faint`。这条与 spec §1「experimental 不进 A 级承诺」一致：它是分类信息，不是错误。

### 2.4 `versions` 的完整版式样例

**默认（`full` 档，无色）**：

```
矩阵来源    builtin · 2026-09-25
受支持版本  1.8.9 · 1.9 · … · 26.2（共 61 个）
            · 全表: vinoa versions --matrix
```

**`--matrix`（`full` 档，无色）**：

```
矩阵来源    builtin · 2026-09-25

1.8.9
  bukkit      java 8   org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT
  velocity    java 25  com.velocitypowered:velocity-api:4.2.0
  bungeecord  java 8   net.md-5:bungeecord-api:1.21-R0.4
  sponge      java 8   org.spongepowered:spongeapi:4.2.0-SNAPSHOT   实验性
```

**窄终端（`no-border`，`W = 70`）**：坐标续行、标记另起一行：

```
矩阵来源    builtin · 2026-09-25

1.8.9
  bukkit      java 8
              org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT
  sponge      java 8
              org.spongepowered:spongeapi:4.2.0-SNAPSHOT
              · 实验性
```

**`minimal`（`W < 40`）**：标签与取值分行（degradation.md §5.2 的 `single-column` 形态）：

```
矩阵来源
  builtin
  · 2026-09-25
受支持版本
  1.8.9 · 1.9 · …
  （共 61 个）
```

### 2.5 `versions --refresh` 的交互边界

`--refresh` 的**动效**（阶段 A 不确定式 spinner / 阶段 B 确定式进度条）归 [motion.md](motion.md) §2.4，**本文不定义**。本文只定**收尾后的静态版式**：

- 刷新成功后 `report::info` 的既有行 `矩阵已刷新并写入缓存: <path>` 改为 §2.1 的 L2 形态：`缓存        <path>`（标签 `dim`，路径 `fg`，路径是 hard 内容 ⇒ 换行不截断）。
- 刷新失败后的 `report::warn` 行**结构不变**（`⚠` 前缀 + 文案），但 `⚠` 与文案之间**不加空格**（现状 `"⚠ {msg}"` 已如此，保持）。
- `--offline` + `--refresh` 不联网 ⇒ 无进度行，直接给 `矩阵来源    builtin · <date>`。

---

## 3. `schema`：人读形态

### 3.1 决定：**保留 `--json` 为唯一机器契约，新增人类摘要（走 stderr）**

**现状**（§0 结论 9、10）：`schema` 与 `schema --json` 逐字节相同，都是 ~1900 列的单行 JSON 走 **stdout**。`--json` 这个 flag 在 `schema` 上**没有作用**。

**规格**：

| 调用 | stdout | stderr |
|---|---|---|
| `vinoa schema --json` | **恰好一个 JSON 文档**（现状不变，见 §6） | 空 |
| `vinoa schema` | **空** | 人类可读摘要（新增） |

**理由**（为什么人读摘要走 stderr 而不是 stdout）：

1. **`--json` 的契约是「stdout 恰好一个 JSON 文档」**（spec §11.5、degradation.md §6.1）。若 `schema`（无 flag）把摘要写 stdout，则**同一命令有无 flag 时 stdout 内容类型不同**——这与 `versions` 的既有口径（人类文本走 stderr）不一致，且会让「`vinoa schema | jq` 能工作吗」变成依赖 flag 的问题。
2. `versions` 已经确立了「人类走 stderr、JSON 走 stdout」的分工（§0 结论 5）。`schema` 跟随同一分工，**三个面的 sink 规则统一**。
3. `--json` 在 `schema` 上从此**有意义**：它是「给我机器形态」的显式开关。这也修掉了 §0 结论 9 那处冗余。

> **注意**：`schema` 无 `--json` 时 **stdout 为空**。这与「`schema` 是给机器读的命令」一致——机器读就该显式写 `--json`；不写就得到给人看的摘要。

### 3.2 人类摘要的版式

摘要是**索引**，不是 JSON 的换行版。它只回答「这个命令描述了什么、去哪里拿全量」。

```
命令契约（vinoa.schema/v1）

命令        init · versions · schema
退出码      9 个（0 成功 · 2 验证失败 · 64 用法 · 65 数据 · 73 无法创建 · 74 IO · 75 临时失败 · 78 配置 · 130 中断）
错误码      21 个
JSON 文档   vinoa.init/v1 · vinoa.versions/v1 · vinoa.schema/v1
            · 全量: vinoa schema --json
```

| 行 | 标签（`dim`，`col.content.left`） | 值（`fg`，`col 16`） | 极次要（`faint`） |
|---|---|---|---|
| 1 | 标题 `命令契约` + schema 版本 | — | `（vinoa.schema/v1）` |
| 2 | `命令` | `init · versions · schema`（`sep` 分隔） | — |
| 3 | `退出码` | `9 个` + `（0 成功 · … ）` | 括号内列表 |
| 4 | `错误码` | `21 个` | — |
| 5 | `JSON 文档` | 三个 schema id | `· 全量: vinoa schema --json` |

**关键取舍**：

- **不打印错误码清单**：21 个码铺开会是又一面版本墙（`#24` 正在修的错误路径同类问题）。摘要只给**计数**，全量走 `--json`。
- **退出码清单保留**：它是 9 项、且是用户最常查的信息（`--help` 的 `after_help` 已有同一条，此处**逐字复用同一字符串常量**，避免两处漂移）。
- **计数是断言对象**：`9 个` / `21 个` 必须与 `EXIT_TABLE.len()` / `error_codes.len()` 一致（§7 A21）——**不得硬编码数字**。

### 3.3 `schema` 的降级形态

| 档 | 形态 |
|---|---|
| 无色 | 标签列 vs 取值列（`tokens.md` §6 的 L2↔L3 列位轴）；`· ` 前缀承载 `faint` |
| ASCII 字形 | `·` → `-`（`sep` 的 ASCII 回退）；其余无字形 |
| `no-border` / `single-column` | 标签与取值**分行**（degradation.md §5.2）；`退出码` 的括号列表按取值边界折行 |
| `minimal`（< 40） | 同上 + `JSON 文档` 行只保留第一个 id + `…`（soft 类，degradation.md §5.3） |
| 非 TTY | **仍打印摘要**（stderr 不是数据通道）。零 ANSI（degradation.md §6.2） |
| `--json` | 摘要**完全不出现**（stderr 空），stdout 一个 JSON 文档 |

---

## 4. 三个面共用的降级形态

**触发条件与优先级不在本文定义**——一律引用 degradation.md §1.8 的优先级链与 §5.1 的宽度档位。

### 4.1 无色档（任一宽度）

| 元素 | token | 无色替代（照 `tokens.md` §3.5） |
|---|---|---|
| 参数名 / 版本号 / 坐标 / 路径 | `fg` | **完全不发色码**（不是"发默认前景色"） |
| 标签（`矩阵来源` / `常用:` / `命令`） | `dim` | 缩进或列位承担（`col.content.left` vs `col 16`） |
| `·` 分隔符、`（共 N 个）`、`实验性` | `faint` | `· ` 前缀 + 缩进 |
| `⚠` 警告（`--refresh` 失败） | `warn` | `!`（已是 ASCII） |
| `✗` 错误（`versions --platform nope`） | `err` | `✗` 字形（degradation.md §2.4 R1） |
| `❯` / `✓` | `accent` / `ok` | 本文三个面**都不使用** `❯`（无「当前项」概念）；`✓` 仅出现在 `--refresh` 成功收尾（归 #24） |

### 4.2 ASCII 字形档（与无色是独立轴）

本文三个面用到的字形**只有 `·`（`sep`）**，回退为 `-`。`--help` 的分组标题、`versions` 的缩进、`schema` 的标签全部是纯文本 + 空格，**不依赖任何制图字符**。这是刻意的：三个输出面是**最可能被 pipe 的面**，不引入字形即不引入字形降级面。

### 4.3 宽度档位

照 degradation.md §5.1 的五档，三个面的形态：

| 档 | `--help` | `versions` | `schema` |
|---|---|---|---|
| `full` ≥100 | §1.5 | §2.4 | §3.2 |
| `no-preview` 80–99 | 同 `full` | 同 `full` | 同 `full` |
| `no-border` 60–79 | 同 `full`（`term_width = W`，clap 自行折行） | 坐标续行、标记另起一行（§2.4） | 标签/取值仍同行 |
| `single-column` 40–59 | 参数名单独一行 | 标签与取值分行 | 标签与取值分行 |
| `minimal` <40 | `term_width = 40`（**下限**，见 §1.3 与 §5 第 1 条） | 分行 + 坐标续行 | 分行 + `JSON 文档` 截断 |

### 4.4 非 TTY / pipe

- 三个面在非 TTY 下**零 `\x1b`**（实测已成立，§0 结论 11）——由 `anstream`（`--help`）与 `report`（`versions`/`schema`）保证，本文不改机制，只**加断言**（§7 A1–A3）。
- `--help` 走 **stdout**，可能被 pipe（`vinoa init --help | less`）⇒ 必须遵守 degradation.md §6.2 与断言 F6/F9。**本文的 `term_width` 改动不得引入任何 ANSI**。
- `versions` / `schema` 的人类文本走 **stderr** ⇒ 被 pipe 的是 stdout（`versions` 无 `--json` 时为空）。**这是既有事实**（§0 结论 5），本文确认并加断言（§7 A5）。

---

## 5. 已知限制与对 spec 正文的修订建议（**本文不改 spec**）

### 5.1 对 #25 的一处豁免申请

`minimal` 档（`W < 40`）下 `--help` 取 `term_width = 40` ⇒ 在 < 40 列的终端上**允许横向溢出**，与 degradation.md §5.1 的契约 (b)「不横向滚动」冲突。

**申请**：把 `--help` 排除在契约 (b) 之外。理由：`--help` 的内容是**不可截断的 hard 内容**（参数名与取值），折到 40 列以下会变成每行一个词、完全不可读；而用户在这种终端下可以 `| less` 或重定向到文件。**这是本文唯一的豁免**，需 #25 或 lead 确认；若不被接受，回退方案是 `minimal` 档下 `--help` 只打印 `用法:` 行 + 一行提示 `终端过窄（< 40 列），请重定向到文件查看完整 --help`。

### 5.2 对 spec §3.1 的修订建议（登记，不执行）

本文 §1.4 让 `--platform` / `--features` 的取值表进 `after_help`。spec §3.1 的 `--help` mockup 里 `--platform` 内联 7 个取值、`--features` 写「见 §3.3」。**建议**：spec §3.1 的 mockup 补一个 `取值:` 段，并把 `--features` 的「见 §3.3」改为「取值见下」——这样 spec 的 mockup 与实现一致。**本文不执行**（spec 正文是唯一事实来源，不在本 effort 范围）。

### 5.3 `--help` 的着色边界（如实保留）

`--color=never` 对 `--help` **不生效**（degradation.md §6.5）。本文裁定接受该边界（§1.1），理由与代价已在 §1.1 写明。**不新增任何机制**。

### 5.4 spec 命令面数量不一致：处置建议（lead 在 task-10 里点名的问题）

**事实**（本票复核）：

- spec §3 正文：「两个子命令：`vinoa init [名称] [选项]`、`vinoa versions [选项]`」；§3.1/§3.4 各一节，**没有 `schema` 章节**。
- 实现：`src/cli.rs` 有三个子命令（`TopCommand::{Init, Versions, Schema}`），`vinoa schema` 打印 `vinoa.schema/v1`。
- `README.md` 的常用命令列出 `vinoa schema --json`；`AGENTS.md` 未提。
- 旁证：`vinoa schema` 已被 degradation.md 的**断言 F3/F6** 覆盖（`schema --json | cat`、`schema --help | cat`），即**下游已把它当既有事实消费**。

**处置建议：补 spec，不在规格里标注待修。** 三条理由：

1. **它已经是被依赖的既有契约**，不是待定项。degradation.md 的断言 F3/F6 若因 spec 说「只有两个子命令」而无法落地，会是**文档挡住测试**。spec §1 的「两个子命令」是**过期陈述**，不是范围约束。
2. **补 spec 的成本极低**：新增 `### 3.5 vinoa schema`，写清（a）它是自描述命令；（b）`--json` 时 stdout 恰好一个 `vinoa.schema/v1` 文档（与 §11.5 同构）；（c）不带 `--json` 时人类摘要走 stderr；（d）它**不改任何 `--json` 形状**。四条都在本文 §3 已定，可直接引用。
3. **不建议「标注待修」**：把一条**已实现且已被下游断言依赖**的事实标为待修，会让实现者在写测试时犹豫以哪个为准。spec 的权威性来自「与实现一致」，不来自「条目少」。

**归属与边界**：spec 正文的修订**不在本 effort 的写作用域**（lead 已明确「不要改 `docs/spec/vinoa-cli.md`」）。本文只**登记建议**，并把四条契约完整写在 §3 与 §6，便于 lead 或 spec owner 直接搬进 §3.5。**本票不碰 spec 正文。**

**顺带登记两处同源不一致**（本票实测发现，供 lead 分派）：

| # | 事实 | 建议 |
|---|---|---|
| 1 | `schema --help` 的 `after_help` **缺失**（`src/cli.rs:322` 的 `help_template` 只有 `{usage}\n\n{all-args}`，无 `{after-help}`），而 `init`/`versions` 都有 | §1.6 已要求补一行说明；实现时同时补 `{after-help}` 占位 |
| 2 | `METADATA_FORMATS` 有 **3** 个值（`plugin.yml` / `paper-plugin` / `paper-plugin.yml`，`src/cli.rs:30`），而 `schema` 输出与 spec §4.2 只承认 **2** 个（`plugin.yml` / `paper-plugin`） | `paper-plugin.yml` 是内部规范名，**不应出现在对外契约里**。建议 `schema` 输出的 `metadata` 数组改为只含对外两值——**这是 `--json` 形状的潜在变更，需 lead 裁定后再动**（本文 §6 只声明「现状不变」） |

---

## 6. `--json` 形状不变（硬契约）

**本文不改变任何 `--json` 形状。** 具体：

| 命令 | `--json` 形状 | 本文的影响 |
|---|---|---|
| `vinoa init … --json` | `vinoa.init/v1`（spec §11.5 冻结） | **无**（本文不碰 init 执行路径） |
| `vinoa versions --json` | `vinoa.versions/v1`，键 `schema/vinoa/ok/command/status/exit_code/source/generated_at/versions/refresh/warnings` | **无**。§2 只改**人类**版式；`source`/`generated_at` 字段与 `versions[].platforms[].experimental` 的**布尔字段名不变**（§2.3 只改人类面的措辞） |
| `vinoa schema --json` | `vinoa.schema/v1` | **无**（§3 只新增**不带 flag** 时的人类摘要；带 flag 时输出逐字节不变） |

**附带确认的两条既有事实**（本文不修，登记为已知）：

1. `versions --platform nope --json` 输出的 `schema` 是 **`vinoa.init/v1`**、`command` 是 **`init`**（实测，§8 复现 10）——因为它走的是 `report::error` 的通用错误文档，而不是 `vinoa.versions/v1`。spec §11.5 说「所有路径同构」，此处 `command`/`schema` 两个字段与该原则有出入。**属 `--json` 形状问题，本文不动，登记给 lead。**
2. `schema` 的 `metadata` 数组含 3 值（见 §5.4 第 2 条）。

---

## 7. 可执行验收断言

**夹具**：`assert_cmd::Command::cargo_bin("vinoa")`（非 TTY）+ `scripts/termcap/ptycap.py`（真 TTY，`--cols <n>`）+ `ratatui::backend::TestBackend`（逐格，`W` 注入 `UiEnv::from_parts`）。SGR 判定复用 degradation.md §7.2 的 `sgr_set()` / `color_sgr()` / `has_esc()`。**颜色与宽度的触发条件一律引用 degradation.md 的断言编号，本文不重定义。**

| ID | 命令 / 输入 | 期望 | 落点 |
|---|---|---|---|
| **A1** | `vinoa init --help \| cat` | `has_esc(stdout) == false` 且 stdout 非空 | pipe |
| **A2** | `vinoa versions --help \| cat`、`vinoa schema --help \| cat` | 同 A1 | pipe |
| **A3** | `vinoa versions \| cat`、`vinoa versions --matrix \| cat`、`vinoa schema \| cat` | `has_esc(stderr) == false` | pipe |
| **A4** | `vinoa versions \| cat` | stdout **为空**（人类文本全在 stderr） | pipe |
| **A5** | `vinoa schema \| cat` | stdout **为空**；stderr 含 `命令契约` 与 `vinoa.schema/v1` | pipe |
| **A6** | `vinoa schema --json \| cat` | stdout 恰好 1 行、`json.loads` 成功、`d["schema"] == "vinoa.schema/v1"` | pipe（= degradation F3） |
| **A7** | `vinoa schema --json` 与改动前逐字节对比 | **相同**（§6 的形状冻结） | pipe |
| **A8** | `vinoa versions --json \| cat` | stdout 恰好 1 行 JSON，键集与 §6 表一致 | pipe |
| **A9** | `COLUMNS=100 vinoa init --help` 的最长行显示宽度 | ≤ 100 | pipe |
| **A10** | `COLUMNS=60 vinoa init --help` | 最长行 ≤ 60，**且无取值被折断**（`update-check` / `placeholderapi` 各自完整出现） | pipe |
| **A11** | `vinoa versions` 的人类输出 | 含 `（共 61 个）`；**不含**连续 8 个以上版本号 | pipe |
| **A12** | `vinoa versions --mc 1.21.11`（1 项，< 8） | **不折叠**，直接列出该版本 | pipe |
| **A13** | `vinoa versions` 第一行 | 匹配 `^矩阵来源\s+builtin · \d{4}-\d{2}-\d{2}$`（**无括号**） | pipe |
| **A14** | `vinoa versions --matrix` | 每行含 `实验性` 的行**不含** `(experimental)`（措辞已改） | pipe |
| **A15** | `vinoa versions --matrix` 的坐标行 | **不出现** `…`（hard 内容不截断） | pipe |
| **A16** | `vinoa versions --matrix` 在 `W = 70` | 存在坐标续行，且续行缩进 ≥ `indent.unit` | unit（TestBackend） |
| **A17** | `W = 50` 的 `versions` 帧 | 标签与取值**分行**（不存在同行「标签在 col 3、取值在 col 16」） | unit |
| **A18** | `W = 30` 的 `schema` 摘要帧 | 每行显示宽度 ≤ 30 | unit（= degradation E7 的子集） |
| **A19** | `vinoa init --help` 输出 | 依次含 `参数:`、`常用:`、`高级:`、`取值:`、`示例:`、`退出码:`；29 项参数顺序与 `help_has_spec_sections_and_flags` 一致 | unit |
| **A20** | `vinoa init --help` 输出 | `取值:` 段的两行分别含全部 7 个平台名与全部 8 个 feature 名，且与 `PLATFORMS` / `FEATURE_NAMES` 常量逐项相同 | unit |
| **A21** | `vinoa schema` 的人类摘要 | `退出码` 行的计数 == `EXIT_TABLE.len()`，`错误码` 行的计数 == `error_codes.len()`（**不得硬编码**） | unit |
| **A22** | `vinoa schema --help` | 含一行说明 `不带 --json 时打印人类可读摘要` | unit |
| **A23** | `vinoa versions --platform nope` 的人类输出 | 首行以 `✗ ` 开头，且 `✗` 恰好 1 次（degradation §2.4 R1） | pipe |
| **A24** | `NO_COLOR=1` + pty 跑三个面 | `color_sgr(stderr) == []`；`bold` 允许存在（degradation A5） | pty |
| **A25** | `--color=always` + pty 跑 `versions` | `color_sgr(stderr) != []`（本文的面受 `--color` 管） | pty |
| **A26** | `--color=always vinoa init --help` + pty | **不保证**含颜色（degradation §6.5 的边界）；本断言只要求**不 panic 且 stdout 非空** | pty |
| **A27** | `LC_ALL=C` + pty 跑三个面 | 不含 `·`（全部为 `-`）；无 `U+2800–U+28FF` | pty |
| **A28** | `vinoa versions --json 2>/dev/null \| json.load` | exit 0（stdout 是完整文档，未与 stderr 交错） | pipe（= degradation F11） |

**A7 是本文最重要的一条**——它把「`--json` 形状不变」变成一条回归门禁，而不是一句承诺。

---

## 8. 复现

```bash
cargo build                                    # 本文全部现状数值来自此二进制

# 1. init --help 的现状与超宽行
./target/debug/vinoa init --help | python3 -c "
import sys,unicodedata
for i,l in enumerate(sys.stdin.read().split('\n'),1):
    w=sum(2 if unicodedata.east_asian_width(c) in ('W','F') else 1 for c in l)
    if w>76: print(f'{i:3d} w={w:3d} {l[:90]}')"

# 2. COLUMNS 不生效（无 wrap_help）
diff <(COLUMNS=40 ./target/debug/vinoa init --help) <(COLUMNS=200 ./target/debug/vinoa init --help) && echo IDENTICAL

# 3. --help 走 stdout
./target/debug/vinoa init --help 2>/dev/null | head -1     # 有输出
./target/debug/vinoa init --help 2>&1 >/dev/null | head -1 # 空

# 4. versions 的人类文本走 stderr
./target/debug/vinoa versions 2>/dev/null                  # 空
./target/debug/vinoa versions 2>&1 >/dev/null | head -1

# 5. 行宽
./target/debug/vinoa versions 2>&1 | python3 -c "
import sys,unicodedata
for l in sys.stdin.read().split('\n'):
    if l: print(sum(2 if unicodedata.east_asian_width(c) in ('W','F') else 1 for c in l), l[:40])"

# 6. 最长坐标
./target/debug/vinoa versions --matrix 2>&1 | grep -oE '[a-z0-9.-]+:[a-z0-9.-]+:[A-Za-z0-9.+-]+' | awk '{print length, $0}' | sort -rn | head -1

# 7. schema 与 schema --json 相同
diff <(./target/debug/vinoa schema 2>/dev/null) <(./target/debug/vinoa schema --json 2>/dev/null) && echo IDENTICAL

# 8. schema 单行宽度
./target/debug/vinoa schema 2>/dev/null | python3 -c "import sys; print(len(sys.stdin.readline()))"

# 9. 三个面的 ANSI 计数（非 TTY 应全为 0）
for c in "init --help" "versions --help" "schema --help" "versions" "versions --matrix" "schema"; do
  printf '%-18s ESC=%s\n' "$c" "$(./target/debug/vinoa $c 2>&1 | grep -c $'\x1b')"
done

# 10. versions 错误路径的 JSON schema 字段
./target/debug/vinoa versions --platform nope --json 2>/dev/null | python3 -c "
import json,sys; d=json.load(sys.stdin); print(d['schema'], d['command'], d['exit_code'])"
```

---

## 9. 与上游 / 下游的接口

| 方向 | 内容 |
|---|---|
| ← [direction.md](direction.md) | 宽度档位（§4）；「颜色只承载语义、不做装饰」；P3 减法 ⇒ 三个面不加边框、不加装饰 |
| ← [tokens.md](tokens.md) | §3.1 三档色值、§3.5 无色替代表、§4 字形（本文只用 `sep`）、§5.1 页边距与 `col.content.*`、§5.3 `label.w` / `name.w` / `indent.unit`、§6 层级表。**本文不重复定义任何数值** |
| ← [motion.md](motion.md) | `versions --refresh` 的动效（§2.4）归它；本文只定**收尾后的静态版式**（§2.5）。motion.md §6 已声明「`versions` 的元信息版式不改」——**本文对 §2.1 的元信息降级是该声明的一处修订**，需 motion.md owner 知悉（登记在 §5.4 同源不一致的邻域，不阻塞） |
| ← [degradation.md](degradation.md) | §1.8 优先级链、§1.7 `--color`、§2.4 R1/R5/R6、§3.2 ASCII 映射、§5.1 宽度档位、§5.3 截断规则、§6.1 `--json`、§6.2 非 TTY、§6.5 `--help` 着色边界。**断言编号直接引用，不重定义触发条件** |
| ← [ADR-0001](../../adr/0001-render-stack.md) | 渲染栈已定 `ratatui 0.29 + crossterm`；`--help` **不经** ratatui（clap 自渲染，§0 结论 4），`versions`/`schema` 的人类面是**行式输出**，也不需要全屏接管——三个面都不依赖 `Buffer::diff()` |
| → **spec §3.5（建议新增）** | §5.4 给出四条契约，供 spec owner 搬入。**本文不改 spec** |
| → **实现票** | §7 的 28 条断言 + §1.3 的 `term_width` + §1.4 的 `after_help` 取值表 + §3.1 的 `schema` 人读分支 |

**下游必须遵守的两条**：

1. **`--json` 形状不得改**（§6）。`versions` / `schema` / `init` 的 JSON 键集与 schema id 冻结。
2. **数值只引用 token 名**。本文出现的 `col.content.left` / `label.w` / `name.w` / `indent.unit` / `col.form.right` / `sep` / `fg` / `dim` / `faint` / `warn` / `err` 全部指向 `tokens.md`，**实现时不得再硬编码**（现状 `{p:<11}` 是本文要修的一处硬编码，见 §0 结论 7）。

---

## 10. 盲区与未决

1. **`--help` 的实际着色观感未验证**：本机 `TERM=dumb` + `NO_COLOR=1`（#20 已记录），`anstyle` 的 heading 加粗在真彩终端下的观感只能靠 CI 或用户确认。
2. **`term_width` 与 clap 内部换行的交互未实测**：`clap 4.6.7` 未启用 `wrap_help` 时 `term_width` 是否仍生效，需在实现时先做最小验证（若无效，则必须启用 `wrap_help` feature，这会引入 `terminal_size` 依赖——**一处新增依赖，需 lead 裁定**）。这是本文唯一可能改变依赖树的地方。
3. **`versions` 折叠阈值 8 未经人眼验收**：它是按 §2.2 的列宽算式 + `direction.md` §2.3 的密度口径推出的，**没有原型图**。若要人眼验收，需在 `render.py` 增一个 `outputs` 原型——该文件属 #22 的写作用域，**本票不改**，在此登记为待办。
4. **`schema` 的人类摘要版式未经人眼验收**（同上，无原型）。
5. **`minimal` 档 `--help` 的豁免**（§5.1）待 #25 或 lead 确认。
6. **spec §3.5 的补写**（§5.4）不在本 effort 写作用域，待 lead 分派。
7. **`versions --platform nope --json` 的 `schema`/`command` 字段**（§6 附带确认 1）是一处既有的 `--json` 形状不一致，本文登记不动。
