# 共享外壳视觉规格（#24）

> **状态**：锁定。本文只**引用** token 名与降级断言编号，**不重复定义任何数值**——数值的唯一来源是 [tokens.md](tokens.md)，降级触发条件与优先级的唯一来源是 [degradation.md](degradation.md)。
> **上游**：[direction.md](direction.md)（#19 方向 REC）、[tokens.md](tokens.md)（#22 token）、[motion.md](motion.md)（#23 动效）、[degradation.md](degradation.md)（#25 降级契约）、[0001-render-stack.md](../../adr/0001-render-stack.md)（#21）。
> **下游**：实现票（把 §7 的断言表机械翻译成测试）。
> **本文范围**：向导四页**之外**、一次 `init` 里用户连续经历的那条非向导输出面——环境预检 / 计划摘要 / 完成横幅 / 错误路径。**不含**向导页内版式（#26/#27）与 `--help`/`versions`/`schema` 三个输出面（#28）。
> **实测环境**：`./target/debug/vinoa`，`TERM=dumb`、`NO_COLOR=1`。本文所有「现状」数值均来自该二进制的真实运行，复现命令见 §8。

---

## 0. 四条硬约束（不得违反）

| # | 约束 | 来源 | 本文如何遵守 |
|---|---|---|---|
| H1 | **说清发生了什么 / 给出下一步动作 / 给出可复现命令** 三要素 | spec §11.3 | 错误路径（§4）必须三要素齐全，版式不改语义 |
| H2 | **禁止「操作失败」式文案** | spec §11.3 | §4.4 给出逐条反例与替代表 |
| H3 | `--json` 时**所有人读内容走 stderr**，stdout **恰好一个** JSON 文档 | spec §11.5 | 本文四处**全部走 stderr**，见 §6 |
| H4 | `--verify` 失败报告顺序冻结：**先给动作（复现命令 + 日志路径），再给日志尾部** | spec §11.4 | §4.5 沿用该顺序，**不重排** |

**H3 是本文的边界线**：共享外壳全部是**人读面**，因此一律 `eprint!`/`eprintln!`。任何把本文版式写进 stdout 的改动都会破坏 spec §11.5 的单一 JSON 文档契约。

---

## 1. 环境预检（`src/init.rs::print_precheck`）

### 1.1 现状与缺陷

现状代码（`src/init.rs:358-375`）：

```rust
eprintln!("{header}");               // "本机环境预检:" / "Environment precheck:"
for check in &java.checks {
    eprintln!("  Java {}  ✓ {}   （{} 模块需要）", check.required, path, check.platform);
}
```

**实测缺陷**（复现：`vinoa init d -m 1.21.11 --platform paper --platform bukkit --dry-run`）：

```
本机环境预检:
  Java 21  ✓ /usr/lib/jvm/java-21-openjdk   （platforms:bukkit 模块需要）
  Java 21  ✓ /usr/lib/jvm/java-21-openjdk   （platforms:paper 模块需要）
```

**同一个 Java 21 打了两行，且两行逐字节相同**（paper 与 bukkit 各触发一次 `check`）。三个问题：

1. **重复**：`java.checks` 是**按平台**产出的，`N` 个平台要同一个 Java 就打 `N` 行。
2. **冗余**：`platforms:` 前缀在每个模块名里重复，而模块名才是信息、`platforms:` 是噪声。
3. **不对齐**：`✓` 与路径之间用**两个空格**手工分隔，路径长度不同 ⇒ 右边的`（… 模块需要）`列位随路径长度漂移。

### 1.2 决定：按 Java 版本去重，模块名聚合

**规则**：

| # | 规则 |
|---|---|
| R1 | **按 `required`（Java 版本）分组**，每个版本**只占一行**——这是去重的唯一键 |
| R2 | 同行内把需要它的模块**聚合为一个集合**，去掉每个模块名的 `platforms:` 前缀 |
| R3 | 模块集合**按固定顺序**输出（与 `spec` 的平台优先级一致：paper / bukkit / velocity / bungeecord / folia / sponge / minestom） |
| R4 | 模块数 ≤ **3** 时全列；≥ **4** 时折叠为前 2 个 + `等 N 个` |
| R5 | **分列对齐**：`版本` / `标记` / `路径` 三列各自定宽，路径列**左对齐**、模块列**右对齐**至 `col.content.right` |
| R6 | 缺失（未检测到）时路径列写`未检测到`，`err` 色；标记 `✗` |
| R7 | 全部满足时，**只打一行汇总**（`✓ 全部就绪`），不打明细——避免"一切正常时最吵" |

### 1.3 列位

| 列 | 起列 | 宽 | token |
|---|---|---|---|
| 缩进 | `indent.unit` × 1 | — | — |
| 版本 `Java {n}` | `col.content.left` | `label.w`（= 11） | `dim`（标签） |
| 标记 `✓` / `✗` | `col.content.left` + `label.w` + `gap.col` | 1 | `ok` / `err` |
| 路径 | 上一列 + 2 | 至 `col.content.right` − 模块列宽 | `fg`（找到）/ `faint`（未检测到） |
| 模块集合 | 右对齐至 `col.content.right` | 自适应 | `faint` |

### 1.4 渲染样例

**正常态（去重后，有彩）**

```
本机环境预检:

  Java 21   ✓ /usr/lib/jvm/java-21-openjdk         paper, bukkit
  Java 25   ✗ 未检测到                             velocity

```

**R7：全部就绪时**

```
本机环境预检:

  ✓ 全部就绪（Java 21 · Java 25 均已检测到）

```

**降级态（无色 + ASCII，`LC_ALL=C`）** —— 语义靠 `✓`/`✗` 与**缩进**保持：

```
本机环境预检:

  Java 21   + /usr/lib/jvm/java-21-openjdk         paper, bukkit
  Java 25   x 未检测到                             velocity

```

> 无色档的 `✓` → `+`、`✗` → `x` 见 [tokens.md](tokens.md) §4 的 ASCII 回退表。**注意**：`err` 在无色档**必须**保留一个可辨字形（本例 `x`），不允许"只靠删掉颜色"——这是 [degradation.md](degradation.md) §2.4 的 R1/R2。

**窄终端（< 80 列）** —— 模块集合**换行**而非截断（信息优先，[degradation.md](degradation.md) §5.3）：

```
本机环境预检:

  Java 21  ✓ /usr/lib/jvm/java-21-openjdk
           · paper, bukkit
  Java 25  ✗ 未检测到
           · velocity

```

---

## 2. 计划摘要（`src/report/mod.rs::plan_human`）

### 2.1 现状与缺陷

现状代码（`src/report/mod.rs:167-187`）：

```rust
out.push_str(&format!("即将创建（共 {} 个文件）: {}\n", plan.files.len(), plan.root.display()));
for f in &plan.files {
    out.push_str(&format!("  {}{mark}\n", f.path));     // 43 个文件 → 43 行
}
out.push_str(&format!("  跳过 {} 项（条件未命中）:\n", plan.skipped.len()));
for s in &plan.skipped { ... }                          // 每条塞着长 when: 表达式
```

**实测**（`--platform paper --platform bukkit`，43 个文件）：

- **43 行平铺**，无分组、无目录层级 —— `core/src/main/java/...` 与 `build.gradle.kts` 同级并列。
- 「跳过 N 项」每条是**一整条 `when:` 表达式**，实测有 **>150 字符**的条目。

### 2.2 决定：目录树折叠 + 跳过项按原因分组

**规则**：

| # | 规则 |
|---|---|
| T1 | 输出**目录树**而非平铺列表：同一目录下的文件折叠为 `目录/  N 个文件` |
| T2 | **折叠阈值**：一个目录下文件数 ≤ **3** 时**展开**逐条列出；≥ **4** 时折叠为一行 `目录名/  N 个文件` |
| T3 | 顶层（`<root>/`）**永不折叠其直接子项中的目录条目**，但顶层文件按 T2 处理 |
| T4 | 树形字符用 `tree.branch` / `tree.last` / `tree.vertical`（`├─` / `└─` / `│`），ASCII 档回退见 [tokens.md](tokens.md) §4 |
| T5 | **跳过项按「原因」分组**，不逐条列 `when:` 表达式。原因 = 表达式里的**顶层开关名**（如 `cap_gui`、`is_paper_metadata`、`lang`） |
| T6 | 每组**最多列 3 个文件名**，超出写 `等 N 项` |
| T7 | 跳过项**总数为 0 时整块省略**；总数 ≤ **6** 时不折叠（直接列文件名）；> 6 时按 T5 分组 |
| T8 | 标题行 `即将创建（共 N 个文件）:` 保留——**文件总数是确定的**（计划已算完），与 #26 ① 页「不许诺未确定的文件数」**不冲突**（那里平台未选） |

### 2.3 渲染样例

**正常态（有彩，43 个文件）**

```
即将创建（共 43 个文件）: /root/projects/my-plugin

  my-plugin/
  ├─ .editorconfig
  ├─ .gitattributes
  ├─ .github/workflows/build.yml      3 个文件
  ├─ .gitignore
  ├─ CHANGELOG.md
  ├─ LICENSE
  ├─ README.md
  ├─ build.gradle.kts
  ├─ config/checkstyle/               2 个文件
  ├─ core/                            12 个文件
  ├─ gradle/                          2 个文件
  ├─ gradlew
  ├─ gradlew.bat
  ├─ platforms/paper/                 7 个文件
  └─ platforms/bukkit/                6 个文件

  跳过 16 项
  · cap_gui 未命中              Menu.java 等 3 项
  · is_paper_metadata 未命中    paper-plugin.yml 等 2 项
  · cap_dual_lang 未命中        README.en.md 等 4 项
  · 其它                        7 项

```

**降级态（无色 + ASCII 树形）** —— 层级靠**缩进**保持：

```
即将创建（共 43 个文件）: /root/projects/my-plugin

  my-plugin/
  |- .editorconfig
  |- .gitattributes
  |- .github/workflows/build.yml      3 个文件
  |- core/                            12 个文件
  |- platforms/paper/                 7 个文件
  `- platforms/bukkit/                6 个文件

  跳过 16 项
  - cap_gui 未命中              Menu.java 等 3 项
  - is_paper_metadata 未命中    paper-plugin.yml 等 2 项
  - cap_dual_lang 未命中        README.en.md 等 4 项
  - 其它                        7 项

```

> **关键点**：无色档下「跳过项」与「创建项」的区分**不能靠颜色**，靠的是**分组标题 + 缩进 + `-` 前缀**。这是 [degradation.md](degradation.md) §2.4 R6（次要信息靠缩进）的落实。

---

## 3. 完成横幅（`src/init.rs::print_done`）

### 3.1 现状与缺陷

现状代码（`src/init.rs:377-383`）**只有四行**：

```
✓ 完成: /root/projects/my-plugin

下一步:
  cd /root/projects/my-plugin
  ./gradlew build        # 构建插件 jar
```

spec §4.4 要求**五件**：`cd` / `build` / `runServer` / 版本注意事项 / 文档指路。现状**缺三件**。

### 3.2 决定：五行固定版式，缺件用等价说明占位

**规则**：

| # | 规则 |
|---|---|
| B1 | 五件**固定顺序**：`cd` → `build` → `runServer` → 版本注意事项 → 文档指路 |
| B2 | 标题行 `✓ 完成: {root}`，`ok` 色；副信息 `N 个文件 · {耗时}` 用 `faint`，右对齐 |
| B3 | **每一件都是一行**，前缀两个空格；命令用 `fg`，右侧 `#` 注释用 `faint` |
| B4 | `runTask = none` 时第 3 行**不省略**，换成**等价说明**（见 §3.3）——保持"五件"的行数稳定，避免版式跳动 |
| B5 | 版本注意事项行用 `warn`，内容是**该目标版本需要的 Java 版本** |
| B6 | 文档指路行用 `faint`，右对齐；内容指向生成物的 README 与对应平台章节 |
| B7 | 全部走 **stderr**（H3） |

### 3.3 `runTask = none` 的等价说明

`runTask` 由 `src/template/vars.rs:167-173` 推导：勾了 `paper` → `run-paper`；勾了 `velocity` → `run-velocity`；**否则 `none`**。

**`none` 的实际含义**：本工程没有可一键启动的本地测试服。典型场景是 **MC 1.8.9**——1.8.9 无 Paper 上游（`run-paper` 要求 Paper ≥ 1.8.8），也没有 velocity。

**表达规则**：**不要说「无法启动」**（那是 H2 禁止的「操作失败」式文案），要**给出替代动作**：

| runTask | 第 3 行 |
|---|---|
| `run-paper` | `  ./gradlew runServer           # 起本地 Paper 测试服（首次需联网）` |
| `run-velocity` | `  ./gradlew runVelocity          # 起本地 Velocity 代理端` |
| `none` | `  ./gradlew runServer 不适用      # 1.8.9 无 Paper 上游；用 BuildTools/vanilla 起服，见 README` |

### 3.4 渲染样例

**正常态（`runTask = run-paper`，有彩）**

```
✓ 完成: /root/projects/my-plugin                    43 个文件 · 0.3s

  cd /root/projects/my-plugin
  ./gradlew build              # 构建插件 jar
  ./gradlew runServer          # 起本地 Paper 测试服（首次需联网）
  注意: 目标 1.21.11 需要 Java 21
  README.md · paper 章节                                        docs/README.md

```

**`runTask = none`（MC 1.8.9）** —— 注意第 3 行换成等价说明，**行数不变**：

```
✓ 完成: /root/projects/my-plugin                    41 个文件 · 0.2s

  cd /root/projects/my-plugin
  ./gradlew build              # 构建插件 jar
  ./gradlew runServer 不适用    # 1.8.9 无 Paper 上游；用 BuildTools/vanilla 起服
  注意: 目标 1.8.9 需要 Java 8
  README.md · bukkit 章节                                       docs/README.md

```

**降级态（无色）** —— 语义靠 `✓`、缩进、列位保持：

```
+ 完成: /root/projects/my-plugin                    43 个文件 · 0.3s

  cd /root/projects/my-plugin
  ./gradlew build              # 构建插件 jar
  ./gradlew runServer          # 起本地 Paper 测试服（首次需联网）
  注意: 目标 1.21.11 需要 Java 21
  README.md · paper 章节                                        docs/README.md

```

---

## 4. 错误路径（`src/report/mod.rs::error`）

### 4.1 现状与缺陷

**实测**（复现：`vinoa init d -m 1.8.9 --platform paper --dry-run`）：

```
✗ 平台与版本组合不存在：paper × 1.8.9
  该版本可用平台: bukkit, sponge
  该平台可用版本: 1.9.4, 1.10.2, 1.11.2, …（52 个，共 395 字符挤在一行）
  错误码: matrix.unsupported_combination
  hint: 复现: vinoa versions --matrix --mc 1.8.9
```

**问题**：「该平台可用版本」把 **52 个版本号**塞进一行（395 字符），终端 wrap 成三行后**读不出边界**——这正是 spec §11.3 的**负向样例**。

（注：issue 原文写「51 个」，实测为 **52 个**——矩阵已演进，本文以实测为准。）

### 4.2 决定：长列表折叠 + 指路命令

**规则**：

| # | 规则 |
|---|---|
| E1 | **长列表折叠为「首 … 尾」+ 计数**：`1.9.4 … 26.2（共 52 个）` |
| E2 | **折叠阈值**：候选数 ≤ **8** 时全列；> 8 时折叠 |
| E3 | 折叠后**必须**附**指路命令**（`vinoa versions --matrix --mc <版本>`），让用户能拿到全量 |
| E4 | 三要素版式：**发生了什么**（`✗` 行）/ **下一步**（可用项）/ **可复现命令**（hint 行） |
| E5 | 「可用平台」与「可用版本」**分两行**，各自带 `dim` 标签列，值列对齐 |
| E6 | 错误码行用 `faint`，格式 `错误码: {code}` |
| E7 | **全部走 stderr**（H3） |

### 4.3 三要素版式

```
✗ {发生了什么}                                      ← err 标记 + fg 正文
  {标签}: {可用项（折叠后）}                          ← dim 标签 + fg 值
  {标签}: {可用项（折叠后）}
  错误码: {code}                                     ← faint
  hint: 复现: {命令}                                 ← faint（H1 第三要素）
```

### 4.4 文案反例（H2）

| 反例（禁止） | 替代表达 |
|---|---|
| `操作失败` | `✗ 平台与版本组合不存在：paper × 1.8.9`（说清**什么**不成立） |
| `无法生成工程` | `该版本可用平台: bukkit, sponge`（给**下一步**） |
| `参数错误` | `错误码: matrix.unsupported_combination` + 指路命令 |

### 4.5 渲染样例

**正常态（折叠后，有彩）**

```
✗ 平台与版本组合不存在：paper × 1.8.9

  该版本可用平台: bukkit, sponge
  该平台可用版本: 1.9.4 … 26.2（共 52 个）
                 vinoa versions --matrix --mc 1.8.9
  错误码: matrix.unsupported_combination
  hint: 复现: vinoa init d -m 1.8.9 --platform bukkit --dry-run

```

**候选 ≤ 8 时不折叠**

```
✗ 未知平台 `nope`

  可用平台: paper, bukkit, velocity, bungeecord, folia, sponge, minestom
  错误码: usage.invalid

```

**降级态（无色 + 窄终端 60 列）**

```
x 平台与版本组合不存在：paper × 1.8.9

  该版本可用平台: bukkit, sponge
  该平台可用版本: 1.9.4 … 26.2（共 52 个）
                 vinoa versions --matrix --mc 1.8.9
  错误码: matrix.unsupported_combination
  hint: 复现: vinoa init d -m 1.8.9 --platform bukkit --dry-run

```

> 无色档 `✗` → `x`（[tokens.md](tokens.md) §4）。**指路命令不得被截断**——它在窄终端下换行而非 soft 截断（[degradation.md](degradation.md) §5.3 硬规则 3 对命令的豁免：命令是"可复现"要素，属于 H1 必备，不可省）。

---

## 5. 与上游的差异（必须记）

| # | 差异 | 处置 |
|---|---|---|
| C1 | issue #24 原文写「51 个版本号」，实测 **52 个**（矩阵已演进） | 本文以**实测**为准，并在 §4.1 注明差异 |
| C2 | `print_precheck` 的 `（platforms:bukkit 模块需要）` 含 `platforms:` 前缀 | §1.2 R2 **去掉前缀**——模块名才是信息 |
| C3 | `plan_human` 的跳过项逐条列 `when:` 表达式 | §2.2 T5 改为**按原因分组** |
| C4 | `print_done` 缺三件 | §3.2 补齐五件，`runTask = none` 用等价说明占位（B4） |
| C5 | 现状 `动作: {action:?}` 用 Rust `Debug` 格式输出 | **实现缺陷**，不在本文版式内；建议改 `Display`。本文不定义其版式（属 `Plan` 的调试输出，待实现票处置） |

---

## 6. 对 stdout 的零影响约束（H3 的落实）

本文四处**全部**是 `eprint!` / `info()`（后者 = `eprint!`，见 `src/report/mod.rs:70`）。实现时必须保证：

| 面 | 流向 | 断言 |
|---|---|---|
| 环境预检 | stderr | `vinoa init … --json 2>/dev/null` → stdout 仅一个 JSON 文档 |
| 计划摘要 | stderr | 同上 |
| 完成横幅 | stderr | 同上 |
| 错误路径 | stderr | `vinoa init … --json --platform nope 2>/dev/null` → stdout 仅一个 JSON 文档 |

引用 [degradation.md](degradation.md) 的既有断言：**F1 / F2 / F3 / F4 / F5 / F6**（`--json` 单文档 + 零 ANSI）。本文不另立。

---

## 7. 可执行验收断言

| ID | 断言 | 期望 |
|---|---|---|
| S1 | `vinoa init d -m 1.21.11 --platform paper --platform bukkit --dry-run 2>&1` 中 `Java 21` 出现次数 | **== 1**（当前为 2） |
| S2 | 同上，输出中不含 `platforms:` 前缀 | 0 处 |
| S3 | `plan_human` 输出中，目录条目含 `个文件` 的比例 | 折叠后 **顶层行数 ≤ 15**（当前 43 行） |
| S4 | `plan_human` 跳过项区块中，单条 `when:` 表达式长度 | **≤ 40 字符**（当前 >150） |
| S5 | `print_done` 输出行数（含空行） | **≥ 7**（五件 + 标题 + 空行） |
| S6 | `vinoa init d -m 1.8.9 --platform bukkit --dry-run` 的完成横幅含 `runServer` 替代说明 | 1 处，且**不含**「无法」「失败」 |
| S7 | `vinoa init d -m 1.8.9 --platform paper --dry-run 2>&1` 中「该平台可用版本」单行字符数 | **≤ 60**（当前 395） |
| S8 | 同上，含 `（共 52 个）` 与 `vinoa versions --matrix --mc` | 各 1 处 |
| S9 | 错误路径三要素齐备（发生了什么 / 下一步 / 可复现命令） | 3/3 |
| S10 | 四处输出在 `--json` 下全部走 stderr（§6） | stdout 仅一个 JSON 文档 |

---

## 8. 复现

```bash
cargo build

# 1. 环境预检的重复行（当前 2 行）
./target/debug/vinoa init d -m 1.21.11 --platform paper --platform bukkit --dry-run 2>&1 | head -4

# 2. 计划摘要的平铺行数（当前 43 行）
./target/debug/vinoa init d -m 1.21.11 --platform paper --platform bukkit --dry-run 2>&1 | wc -l

# 3. 跳过项中的长 when: 表达式
./target/debug/vinoa init d -m 1.21.11 --platform paper --dry-run 2>&1 \
  | awk '/跳过/ {f=1} f && length($0)>100 {print length($0), $0}'

# 4. 完成横幅（当前 4 行，缺三件）
./target/debug/vinoa init d -m 1.21.11 --platform paper -o /tmp/d --yes 2>&1 | tail -6

# 5. 错误路径的版本墙（当前 395 字符单行）
./target/debug/vinoa init d -m 1.8.9 --platform paper --dry-run 2>&1 \
  | awk '/该平台可用版本/ {print length($0)}'

# 6. 版本号计数
./target/debug/vinoa init d -m 1.8.9 --platform paper --dry-run 2>&1 \
  | sed -n 's/.*该平台可用版本: //p' | tr ',' '\n' | wc -l

# 7. 四处全部走 stderr（stdout 应为单个 JSON 文档）
./target/debug/vinoa init d -m 1.21.11 --platform paper --dry-run --json 2>/dev/null | python3 -c \
  "import json,sys; json.load(sys.stdin); print('stdout = 1 JSON doc')"
```
