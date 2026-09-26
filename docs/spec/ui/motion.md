# vinoa 动效规格（#23）

> **状态**：锁定。本文只定**动效**（motion）——触发、时长、帧率、降级。版式与数值引用 [tokens.md](tokens.md)，降级触发与优先级引用 [degradation.md](degradation.md)（#25），渲染底座引用 [0001-render-stack.md](../../adr/0001-render-stack.md)（#21，**已定：`ratatui 0.29 + crossterm` 全屏接管**），方向引用 [direction.md](direction.md)（#19）。
> **读者**：#24 共享外壳、#26/#27 向导逐页、#25 降级契约。
> **一句话**：**动效只出现在三处——长任务进度、页面切换的单帧切换、以及被修掉的伪动效（重绘抖动）。框内不做动画。**

---

## 0. 范围与三条硬规则

用户已拍板的边界，本文不重开：

1. **动效只出现在三处**：① 长任务真进度；② 消除重绘抖动（这是**修伪动效**，不是加动效）；③ 页面切换的轻微过渡。
2. **明确不做**（清单见 §6）：打字机 / 逐字 reveal / 彩虹渐变 / ASCII logo 庆典 / 完成庆祝动画。
3. **框内不做动画**（#19 采纳 P3 减法的直接后果）。三处动效**全部位于 shell 表面**（`--verify` 与 `versions --refresh` 的输出行），**没有任何一处在向导的边框内**。向导的边框内只有静态字形。

另有三条贯穿全文的硬规则：

- **H1 进度永远走 stderr。** stdout 只在 `--json` 下承载恰好一个 JSON 文档（spec §11.5）。任何进度字节都不得进入 stdout。
- **H2 原地重绘上限 10 次/秒**，且同一行一个 tick 内只重绘一次（§2）。
- **H3 无色 ≠ 不动。** 颜色与运动是两条正交的降级轴：`NO_COLOR` 下动画照常，只去掉 SGR；非 TTY 下动画才关闭（§3.6）。

---

## 1. 全局动效预算

所有数值集中在此，下文只引用名字。**单位：毫秒**（除注明者）。

| 常量 | 值 | 含义与理由 |
|---|---|---|
| `motion.tick.spinner` | `125`（8 Hz） | 不确定式指示器的帧间隔。8 Hz 足以读出「还活着」，又明显低于「刷屏」阈值 |
| `motion.tick.elapsed` | `1000`（1 Hz） | 耗时计数的最小刷新率；即使没有任何事件，秒数也必须走动 |
| `motion.repaint.min` | `100` | **上限约束**：任意表面在 1 秒内最多 10 次原地重绘（H2） |
| `motion.coalesce` | `16` | 同一窗口内的多个事件合并成一帧（约 60 fps 上限） |
| `motion.fps.cap` | `60` | 帧率硬上限，任何表面都不得超过 |
| `motion.frame.budget` | `16` | 单帧 compose 预算（100×40 网格上远超所需） |
| `motion.resize.debounce` | `50` | 吞掉 resize 风暴（SIGWINCH 连发）后再出一帧 |
| `motion.transition.duration` | `0` | 页面切换 = 单帧硬切，**没有插值时长**（§5） |
| `bar.w` | `24` | 确定式进度条宽度（单元格）。66 步 / 24 格 ≈ 2.8 步每格，精确度由 `n/N` 数字承担 |
| `spin.frames.unicode` | `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` | 10 帧，全部 East Asian Width = N（宽 1）。10 帧 @8 Hz = 1.25 s 一圈 |
| `spin.frames.ascii` | `\|/-\` | 4 帧 ASCII 回退。**ASCII 档下不得出现盲文字形** |
| `bar.full` / `bar.empty` | `█` / `░`（ASCII `#` / `-`） | 引用 tokens.md §4，本文不重复定义 |
| `step.maxw` | 内容区宽度 | 「当前步骤」截断到 `col.content.right - col.content.left`（= 94），单行，超出用 `…` 截断 |

---

## 2. 第 1 处：长任务真进度

### 2.1 先查证：Gradle 到底给不给得出真实完成度

**结论：给不出。在 vinoa 当前的（且已被 spec §11.4 冻结的）调用方式下，Gradle 不可能向 vinoa 提供真实完成度。** 证据分四层：

**证据 1 — Gradle 确实有进度条，但它是「任务计数」而不是「工作量」。**
Gradle 官方文档 *Rich console* 一节列出的特性原文是：`Progress bar and timer visually describe the overall status`。
其实现见 Gradle 源码 `platforms/core-runtime/logging/src/main/java/org/gradle/internal/logging/console/ProgressBar.java`：

```java
public void update(boolean failing) { this.current++; ... }          // 每完成一个任务 +1
public void moreProgress(int totalProgress) { total += totalProgress; } // 任务图扩张时抬高分母
int progressPercent = (int) (current * 100.0 / total);               // 百分比 = 已完成任务 / 已知任务
```

也就是说百分比的分母是**任务图里的可执行任务数**，不是时间、不是字节、不是依赖解析进度。它天然在构建中途跳变（`moreProgress` 会改分母），也不是「整体完成度」。

**证据 2 — vinoa 主动关掉了它。**
`src/verify.rs:239-265` 的 `build_command()` 固定追加 `--console=plain`：

```rust
flags.push("--console=plain");
```

Gradle 文档对 `plain` 的定义是：*"Set to plain to generate plain text only. This option disables all color and other rich output in the console output."* —— **plain 明确禁用 rich output，包括进度条。** 这一组 flag 由 spec §11.4 冻结（「附加 flag：`--offline`（默认开）；`--no-daemon`；`--console=plain`；`--stacktrace`」），不属于本票可改范围。

**证据 3 — 进程输出被重定向到文件，Gradle 根本不在终端上。**
`src/verify.rs:282-287`：

```rust
cmd.stdin(Stdio::null())
   .stdout(Stdio::from(log))
   .stderr(Stdio::from(err_log));
```

Gradle 文档：*"Set to plain … This is the default when Gradle is not attached to a terminal."* —— 即便不显式给 `--console=plain`，重定向到文件也已使它退化为 plain。**两条路径独立地都得到 plain。**

**证据 4 — 不能改走 `--console=rich` 绕过。**
文档说 `rich` 在未接终端时会 *"use ANSI control characters to generate the rich output"*。若采纳，会 (a) 违反 §11.4 冻结的 flag 集；(b) 把 ANSI 控制序列写进 `.vinoa/verify/verify-*.log`，而这份日志的尾部正是 vinoa 要打印给用户看的 `tail`（spec §11.4）——日志会被污染；(c) 即便如此，拿到的仍是证据 1 的任务计数，而非真实完成度。

**附带发现（同样否决「从日志数任务」这条路）**：`--console=plain` 在默认 lifecycle 日志级别下**不会为每个任务打印 `> Task :x`**。Gradle 文档对 `verbose` 的定义是 *"enable color and other rich output like rich with **output task names and outcomes at the lifecycle log level**, (as is done by default in Gradle 3.5 and earlier)"* —— 这反证了现代 Gradle 在 lifecycle 级别**不**逐任务打印，只有产生输出的任务与失败任务才出现。所以 vinoa 也无法靠数日志行得到可靠分母。

> **本机盲区（如实声明）**：本沙箱无法真正执行 Gradle（`libnative-platform.so` 加载失败，Gradle 报 `Could not initialize native services`），因此上述结论建立在 **Gradle 官方文档 + Gradle 源码 + vinoa 自身源码（冻结的调用方式）** 三者之上，而非本地实测运行。这与本票的判定无关紧要：**证据 2 与 3 是 vinoa 自己的代码，属于确定性事实**——只要 §11.4 的 flag 集与重定向不变，Gradle 就必然处于 plain 模式。

### 2.2 裁定：两处采用不同的形态

因为两处的**信息可得性不同**，形态也不同——这正是「先查证再定」的结果：

| 长任务 | 形态 | 依据 |
|---|---|---|
| `--verify`（`./gradlew build`） | **不确定式 spinner + 实时耗时 + 当前步骤**（Gradle 日志尾行） | §2.1：真实完成度不可得，**禁止伪造百分比** |
| `versions --refresh` | **确定式进度条 `n/N`** | §2.4：请求总数由 vinoa 自己的循环决定，**真实可得** |

**硬禁止**：不得为 `--verify` 编造任何百分比、不得使用 `--console=rich`、不得解析 Gradle 内部进度。不确定就是不确定——用 spinner 如实表达。

### 2.3 `--verify` 规格

**触发条件**（全部满足才出现动效）：
`plan.actions` 含 `PlannedAction::Verify` ∧ `--json` 未给出 ∧ stderr 是 TTY。
（`src/init.rs:160-178` 是调用点；`verify::run_with` 的 `spawn` 之后到 `try_wait` 返回之间是长任务区间。）

**形态**（单行，原地重绘，**位于 shell 表面而非向导框内**）：

```
⠹ 构建中… 42s · > Task :platforms:paper:compileJava
```

三段：`spinner` + 固定前缀与耗时 + `·` 分隔的当前步骤。

- **当前步骤**取自 vinoa 已经写盘的日志（`log_abs`，`src/verify.rs:134`）：读最后一条非空行，去 ANSI、截断到 `step.maxw`。**只在内容变化时更新**（内容寻址，避免无谓重绘）。这是 Gradle 的**真实**输出，不是推测。
- **耗时**格式（`fmt_elapsed`）：`< 60s` → `42s`；`60s ≤ t < 3600s` → `1m32s`；`≥ 3600s` → `1h02m`。
- **超时升级**：当 `elapsed ≥ 0.8 × timeout_s`（默认 600 s → 480 s）时，该行切到 `warn` token 并追加 `超时上限 600s`。超时发生时的收尾沿用既有语义（杀进程树、`reason="timeout"`、退出码 2），进度行以 `✗` 终结。
- **收尾**：进度行必须被**终结**（写成最终态）而不是留在屏上——成功 `✓ 验证通过（42s，构建日志: …）`，失败 `✗`（沿用 spec §11.4 既有版式，本票不改其结构）。`src/init.rs:169/172` 两行是收尾点。

**`--verify-tail` / 日志**：不变。进度行只是运行期的**单行**提示，完整尾部仍由既有报告打印 20 行（spec §11.4）。

### 2.4 `versions --refresh` 规格

`src/matrix/refresh.rs:246-294` 的 `refresh()` 是**顺序**循环：

```rust
for project in PROJECTS {                 // paper, folia, velocity
    let versions = client.project_versions(project)?;   // 1 次请求
    for mc in &versions {                               // 每个 release 版本 1 次请求
        if let Some(build) = client.first_stable(project, mc)? { … }
    }
}
```

以 `data/version-matrix.toml` 的 release 版本数计：paper 52 → 53 次、folia 12 → 13 次、velocity 走 `model = "protocol"`（版本由 API 返回，静态不可知）→ 1 次发现请求。**即 67 次左右的顺序 HTTP 请求**，单请求 `ureq` 全局超时 30 s（`src/matrix/refresh.rs:28`）——这就是「联网时全程零输出」的规模。

因此刷新分两段，形态不同：

**阶段 A（发现，3 次请求）— 不确定式**
```
⠼ 正在获取项目版本 (1/3) · paper
```

**阶段 B（逐版本取 STABLE）— 确定式**
```
[████████░░░░░░░░░░░░░░░░] 23/66 · paper 1.21.4
```
- 分母在阶段 A 结束后**精确可知**（= 三个项目返回的 release 版本数之和）。**阶段 B 的进度条是真实完成度，不是估算。**
- 阶段 B 的推进是**事件驱动**：每完成一次请求推进一格并重绘一次。进度条不做缓动、不做插值——**只在事件发生时跳变**（避免为动画引入 tick 循环，见 §5 的同一理由）。
- **存活信号**：单个请求可能耗时数秒（30 s 超时）。若距上次推进已超过 `motion.tick.elapsed`，仍按 1 Hz 重绘该行以走动秒数：`… 23/66 · paper 1.21.4 · 12s`。
- **失败**：刷新失败从不致命（spec §6.4）。进度行先以 `!`/`✗` 终结，再走既有警告路径 `矩阵联网刷新失败，回退内置矩阵（不阻塞 init）`（`src/cli.rs:871-877`）。
- **不触发**：`--offline` + `--refresh` 同时给出时**从不联网**（`decide_source`，`src/refresh_cache.rs:184-190`）→ 无进度、无 spinner。

**实现缝隙**：`refresh()` 目前是单个函数、无回调。规格要求它接受一个进度回调（或返回一个可迭代的步骤序列），由调用方 `src/cli.rs:857` 渲染。这是本票对实现的唯一接口要求。

### 2.5 刷新率上限、耗时与当前步骤

- **刷新率上限**：任何进度表面 ≤ `motion.repaint.min` 的倒数 = **10 次/秒**（H2）。实现方式：把「重绘请求」投进一个受 `motion.coalesce`（16 ms）节流的出口，同一 tick 内对同一行的多次请求只写一次。
- **spinner** 按 `motion.tick.spinner`（8 Hz）推进；**耗时**按 `motion.tick.elapsed`（1 Hz）至少推进一次。两者取并集后仍受 10 Hz 上限约束。
- **耗时呈现**：运行期在进度行内（`42s`）；结束期在收尾行内（`（42s）`）。`--json` 下耗时由既有的 `duration_ms` 字段承载（spec §11.5），**不新增字段**。
- **当前步骤呈现**：`--verify` 用 Gradle 日志尾行；`--refresh` 用「项目 + MC 版本」标签。两者都是**真实状态**，都单行截断。

### 2.6 第 1 处的降级矩阵

| 档位 | `--verify` | `versions --refresh` |
|---|---|---|
| TTY + UTF-8 + 有彩 | `⠹ 构建中… 42s · <尾行>`，`accent`/`dim` 着色 | `[████░░…] 23/66 · paper 1.21.4`，`bar.full` 用 `accent` |
| TTY + UTF-8 + **无色**（`NO_COLOR`） | **动画照常**，仅去掉 SGR；spinner 仍为盲文帧 | 同上，条内字形仍为 `█`/`░` |
| TTY + **`TERM=dumb` 或非 UTF-8** | **停止字形循环**，退化为纯文本行并每秒走动：`构建中… 42s · <尾行>`（ASCII 档 spinner 若保留则只用 `\|/-\`） | 退化为纯文本行：`刷新中… 23/66 · paper 1.21.4`（`bar.full/empty` → `#`/`-`，或整条省略只留 `n/N`） |
| **非 TTY**（pipe / CI / 重定向） | **完全不做原地重绘，不发任何 ANSI**。只出 **2 行离散文本**：开始行 `→ 正在运行 ./gradlew build --offline --no-daemon（超时 600s，日志 .vinoa/verify/…）` + 既有收尾行 | **每个项目完成时出 1 行**（共 3 行）：`paper 52 个版本已核对`；阶段 A 出 1 行开始行 |
| **`--json`** | **不做任何动效**（stdout 是唯一契约，stderr 保持安静）。耗时由 `duration_ms` 承载 | 同左 |

**非 TTY 的取舍理由**：CI 日志是事后读的，逐秒写行只会把日志刷爆且无人受益；2–4 行离散文本既证明「没卡死」，又保持可 grep。

---

## 3. 第 2 处：消除重绘抖动（修伪动效）

### 3.1 根因（实测，不是推测）

用仓库自带的 `scripts/termcap/ptycap.py` 驱动真 pty 复现第 ③ 页：

```bash
python3 scripts/termcap/ptycap.py --out .tmpcap/wiz3.bin --cols 100 --rows 40 --timeout 22 \
  --keys '0.8:\r,1.6:\r,2.4:\r,3.2:\r,4.0:\x1b[B\r,5.0:\r,6.0:\r,7.0:\r' \
  --cwd /root/vinoa/.tmpcap -- /root/vinoa/target/debug/vinoa init demo -p com.example.demo
```

> **方法论警告（先读这条）**：`grep -c` / `data.count(b'…')` 形式的**字节串计数不可靠**，它数的是字节流里同一串出现的次数，而这个次数**强依赖交互步数**（按了几次键、是否键入过滤词）。同一页在最短交互路径与键入过滤词之间可相差数倍。**因此「某字符串出现 N 次」不得作为断言对象**——只在下面用于定性说明「同一块 UI 被反复写出」这一事实。规格里出现的任何具体重绘数字都只是**当时的观测值**，不是实现要追的目标值。

按 #20 的 VT 逐格统计，第 ③ 页平台行的**物理重绘次数随交互步数变化**（不稳定，仅作定性）：

| 交互路径 | 平台行重绘次数 |
|---|---|
| 最短交互路径（纯 Enter，不按方向键、不键入过滤词） | **3**（下界） |
| 快速确认 | 3 |
| 键入过滤词 | 可达 **15** |

而**整屏清屏与交互步数无关**，是一条稳定量——这正是它被选为断言对象的原因（§3.6 A3）：

| 量 | 实测 | 稳定性 |
|---|---|---|
| `ED(2)`（`\x1b[2J`，整屏清） | **0 次** | **稳定**：与步数无关，全程为 0 |
| `EL(0)`（`\x1b[K`，擦到行尾） | 15 次 | 随交互步数变化，仅作定性 |
| `EL(2)`（`\x1b[2K`，擦整行） | 31 次 | 随交互步数变化，仅作定性 |

> **不要把这 15 与 31 读成同一个序列的浮动区间**——它们是**两个不同的转义序列**（`ClearType::UntilNewLine` 与 `ClearType::CurrentLine`），语义不同。早期版本曾写作「`EL` 15–31 次（随按键时序浮动）」，那是把两次计数混成一个范围，属表述错误；现按序列分列。要引用的量只有 `ED(2) = 0`。

**根因**：`src/wizard/mod.rs` 里页面标题与摘要走**裸 `eprintln!`**（第 **58 / 91 / 98–108 / 114 / 157–170 / 190** 行附近），这些字节**只写一次、从不重绘、也从不擦除**（`ED(2) = 0` 证实整屏从未被清）。而 `inquire` 的 prompt 在同一区域**反复用 `EL` 擦写**自己的行。两者**不共享网格、不共享绘制者**，于是 prompt 的擦写把静态标题块「扫」过一遍又一遍——视觉上就是同一块 UI 被反复叠印（次数随交互步数变化，最短路径下已 ≥ 3 遍）。

**这不是动效不够，是伪动效**：屏幕上出现了用户没有要求的运动。**结论：修法是让屏幕只有一个绘制者，而不是给这个抖动加缓动。**

### 3.2 绘制模型：一屏只在同一张网格上绘制一次

四条不变量（invariants）：

- **I1 单写者（single writer）**：向导运行期间，**vinoa 自己的渲染循环是终端的唯一写入者**。任何模块（包括 `inquire`、`report::info`、`report::warn`、裸 `print!`）都不得直接写终端。
- **I2 单网格（single buffer）**：一屏的所有可见元素——整屏边框、顶部导航条、主表单、行内错误、页脚提示、**右侧预览面板**——全部 compose 进**同一张 `Buffer`**（`ratatui` 的 immediate-mode buffer），尺寸为完整视口。
- **I3 每帧一次绘制**：每帧恰好一次 `Terminal::draw()`；由 `Buffer::diff()`（`ratatui-0.29.0/src/terminal/terminal.rs:201` `previous_buffer.diff(current_buffer)`）决定实际写哪些格子。**禁止先清屏再画**——独立清屏会让用户看到中间的空屏态。
- **I4 原子提交**：每帧包在 `BeginSynchronizedUpdate` / `EndSynchronizedUpdate` 里（DEC 2026，crossterm 的 `\x1b[?2026h` / `\x1b[?2026l`，已在 `crossterm-0.28.1` 与 `0.29.0` 的 `src/terminal.rs` 中确认存在）。终端整帧呈现，消除撕裂。这是**零额外帧、零 tick 循环**的抗闪手段。

**屏位选择**：向导默认进入**备用屏**（`EnterAlternateScreen`），退出时由 RAII 守卫 + panic hook 恢复（`EndSynchronizedUpdate` → `LeaveAlternateScreen` → `disable_raw_mode` → `Show` 光标）。理由：整屏边框本就意味着向导独占屏幕；备用屏让 I2 的「单网格」天然成立（没有残留 shell 内容混入）；退出后用户原有 scrollback 完好，而计划摘要随后由普通 shell 路径打印，信息不丢失。
**注意**：I1–I4 的保证**不依赖**备用屏。若 `EnterAlternateScreen` 失败（旧 conhost 等），退化为**主屏全视口绘制**，I1–I4 不变。

### 3.3 具体改动点（把打印变成 widget）

下列裸 `eprintln!` 必须**删除**，其内容改为 compose 进 `Buffer` 的行：

| 位置 | 现状 | 改为 |
|---|---|---|
| `mod.rs:58` | `① 工程` 标题 | 导航条 + 页标题 widget（每帧 compose，内容不变则不写格） |
| `mod.rs:91` | `位置: <name>` | 只读字段行 widget |
| `mod.rs:98–101` | `② 构建` 三行取值 | 只读摘要行 widget（版式属 #26） |
| `mod.rs:102–108` | `JDK: 由所选 MC 版本经矩阵推导…` | 同上，`faint` 行 widget |
| `mod.rs:114` | `③ 目标服务端` 标题 | 页标题 widget |
| `mod.rs:157–160` | `⚠ 至少选择一个平台` | **行内错误行** widget（表单区内 compose，不是打印） |
| `mod.rs:163–169` | `· paper 会自动带上 bukkit 兼容模块` | 预览面板/表单内的 compose 行（归属见 #27） |
| `mod.rs:190` | `④ 附加` 标题 | 页标题 widget |

同理，`src/init.rs:352` 那处 `inquire::Confirm`（ADR Decision 第 4 条已要求迁移）必须走同一套渲染，否则同一屏又出现第二个写者。

**可执行断言**：`src/wizard/` 下不得出现 `print!` / `println!` / `eprint!` / `eprintln!`（在渲染入口之外）。这条可以直接写成测试或 CI grep。

### 3.4 预览面板（第二个受害者）

[direction.md §2.2](direction.md) 已锁定保留常驻预览面板。它是一个**持续重绘区域**——若它被单独绘制（自己的 writer / 自己的 buffer），会**精确复现** §3.1 的同类叠印：主表单的 `EL` 擦写会扫过预览区，预览区的重绘也会扫过表单区。

**规则**：预览面板与主表单**必须共享同一张 `Buffer`**（I2），由 `Layout` 求解分栏（`col.divider` = 66，见 tokens.md §5.2），每帧随主表单一起 compose、一起 diff、一起在同一个 synchronized update 内提交。**预览面板不得有任何独立的刷新路径或独立的帧率。**

预览内容的更新（勾 `paper` → 树里多出 `platforms/bukkit`）**是 diff 的自然结果，不是动画**：格子内容变了就重写，没变就不写。**不做**逐项「飞入」、**不做**数字滚动、**不做**树展开动画（见 §6）。

### 3.5 第 2 处的降级

| 档位 | 行为 |
|---|---|
| 有彩 / 无色 | **同一张网格、同一帧、同一次 diff**；无色只是不发 SGR。结构完全一致 |
| ≥ 100 列 | 完整 REC：边框 + 导航条 + 表单 + 预览，全部在一张 Buffer |
| 80–99 列 | 丢弃预览面板（direction.md §4）→ 仍是**一帧一网格**，只是少一个 widget |
| < 80 列 | 丢弃整屏边框（P3 连续流）→ 仍是一帧一网格 |
| `TERM=dumb` / 非 UTF-8 | 框线与字形退 ASCII（tokens.md §4），**不进入备用屏**，主屏全视口重绘；模型不变 |
| 非 TTY | **向导根本不运行**（`should_run` 要求 stdin+stdout 都是 TTY，`src/wizard/mod.rs:20-23`），无降级路径需要定义 |

### 3.6 第 2 处的验收断言

**断言对象的选择原则**：只断言**不随交互步数变化**的量。物理重绘次数由 `Buffer::diff()` 决定（ADR-0001 已定 ratatui），把它写成等值断言会与 diff 的行为打架——**diff 有权多写或少写格子，那不是缺陷**。因此本文把断言分成两层：

- **逻辑层（主断言）**：每个逻辑行在**逻辑上**只被绘制一次——由 §3.2 的 I1 单写者不变量保证，可用源码级检查（A1）验证。
- **物理层**：只断言**稳定的物理量**（整屏清 == 0、synchronized update 成对），以及**下界**（不得少于该画的内容），绝不断言「恰好 N 次重绘」。

复用 §3.1 的 `ptycap.py` 命令，对捕获字节流断言：

| # | 层 | 断言 | 期望 |
|---|---|---|---|
| **A1** | 逻辑 | **③ 页每个逻辑行只被逻辑绘制一次**：`src/wizard/**` 内不得出现 `print!` / `println!` / `eprint!` / `eprintln!`（渲染入口之外），即页面标题与摘要不得走裸打印 | **0 处**（当前为 **11 处**裸 `eprintln!`，全部在 `src/wizard/mod.rs`，行号见 §3.3 表） |
| **A3** | 物理 | `data.count(b'\x1b[2J')`（ED(2)，整屏清） | **== 0** —— **主物理断言**，稳定、与步数无关，直接对应「一屏只画一次」的根因 |
| A1b | 物理 | 平台行**物理重绘次数**（逐格统计，非字节串计数） | **下界 ≥ 3**（最短交互路径），且**不得随「已渲染内容」增长**；**不作为等值断言** |
| A2 | 物理 | `data.count(b'paper implies')` | **不作为断言对象**（字节串计数依赖交互步数，见 §3.1 方法论警告）；仅用于人工核对「同一块 UI 未被反复写出」 |
| A4 | 物理 | `data.count(b'\x1b[?2026h')` | **≥ 1**（I4 生效） |
| A5 | 物理 | `b'\x1b[?2026h'` 与 `b'\x1b[?2026l'` 计数 | 相等（每帧成对，无悬挂） |
| A6 | 物理 | 退出后字节流含 `\x1b[?1049l`（离开备用屏） | 存在 |
| A7 | 逻辑 | `src/wizard/**` 内的 `eprintln!` 等裸打印 | **0 处**（与 A1 同源；A1 是断言表述，A7 是 grep 形式，二者取一即可） |

**A1 + A3 是 #27「③ 页一屏只画一次」与本文档一致性的共同锚点。**
理由：A1 断言的是**根因**（多写者 / 多网格），不依赖交互步数；A3 断言的是**稳定的物理证据**（整屏从未被清）。二者合起来足以证明「单网格、单写者」模型生效，且**都不会与 `Buffer::diff()` 的合法行为冲突**。**A1b 只作下界**，用于回归时发现「重绘随内容增长」这类退化，不作等值门禁。

---

## 4. 第 3 处：页面切换的轻微过渡

### 4.1 裁定：**硬切，不做逐帧过渡**

先回答任务里的两个问题：

**Q1：终端里的「过渡」是不是只是清屏后重绘？**
是，而且**连清屏都不需要**。页面切换时新旧内容差异极大，`Buffer::diff()` 本来就会重写几乎全部格子；再叠加一次独立清屏只会让用户看到「先全空、再全满」的一闪——**那是负价值**。

**Q2：上了 `ratatui`，逐帧动画值不值得（tick 循环与 CPU 占用）？**
**不值得。** 具体代价：

1. **必须新增一个 tick 循环**：为一段 120–200 ms 的过渡引入帧调度器、唤醒源、以及在用户按键时**中断动画**的模型。这是三个新的失败模式，而 ADR 的决策是「采用 ratatui 接管渲染」——不是「引入动画运行时」。
2. **CPU 从 0 变成非 0**：不带动画时，向导**空闲 0 帧/秒、CPU 0%**；带动画则每个页面切换都要跑满 60 fps 持续 200 ms，且必须在无人看时也能被正确取消。收益是「好看一点」，成本是一条常驻的调度路径。
3. **信息上无增益**：过渡动画有意义的前提是它传达空间或层级关系。终端的四页是**顺序**关系，且**顶部导航条已经逐帧显示「我在第几页」**（`✓1 工程 › ▸3 目标服务端 … 3/4`）。过渡动画只是把同一信息再表演一遍。
4. **违反已锁定的方向**：#19 采纳 P3 减法 = **框内不做动画**。页面切换恰好发生在框内。

### 4.2 规格：单帧硬切

- **触发条件**：`Nav::Next` / `Nav::Back` 导致的页索引变化（`src/wizard/mod.rs:31-50` 的循环）。
- **时长**：`motion.transition.duration = 0` ms —— **恰好一帧**。
- **帧数**：1。顺序为：`BeginSynchronizedUpdate` → （必要时 `terminal.clear()`，仅当新页比旧页矮、diff 无法覆盖残格）→ compose 新页到 Buffer → `draw()`（内部走 diff）→ `EndSynchronizedUpdate`。
- **可见的「过渡」**：导航条上的当前页标记从旧页位移动到新页位（`▸` 落在新的 `n/4` 上，`accent` + bold）。因为导航条每帧都 compose，标记**在同一帧内出现在新位置**——这就是全部过渡。
- **帧率**：不适用（无动画）。**若**未来复审要求加入过渡，上限为 `motion.fps.cap`（60 fps）、单次时长 ≤ 200 ms、**必须复用现有事件循环的 tick，不得新开线程**，且空闲 CPU 仍须为 0%。
- **CPU 预算**：页面切换期间 ≤ 1%；**空闲（无输入）时恒为 0 帧/秒、0%**。整个向导**没有常驻 tick 循环**——这是本裁定最重要的可测后果。

### 4.3 第 3 处的降级

| 档位 | 行为 |
|---|---|
| 有彩 / 无色 | 同样的单帧硬切；无色仅去掉 SGR |
| 80–99 / < 80 列 | 同样的单帧硬切（只是 widget 集合不同） |
| `TERM=dumb` / 非 UTF-8 | 同样的单帧硬切；不进备用屏，主屏全视口重绘 |
| 非 TTY | 向导不运行，无过渡 |
| `--json` | 向导不运行（`--yes` 路径），无过渡 |

**resize 特殊情形**：`SIGWINCH` 连发时按 `motion.resize.debounce`（50 ms）合并，之后出**一帧**——resize **不是**过渡，不得为它引入动画。

---

## 5. 明确不做（Do-Not list）

**用户已拍板，勿重开**：

| # | 不做 | 理由（可复述的代价） |
|---|---|---|
| D1 | **打字机效果** | 用户已明确拒绝 |
| D2 | **逐字 reveal** | 同上；且它与「一个网格一次绘制」直接冲突 |
| D3 | **彩虹渐变** | 同上；且违反「颜色只承载语义」 |
| D4 | **ASCII logo 庆典** | 同上；占屏且无信息 |
| D5 | **完成时的庆祝动画** | 同上；完成信息由完成横幅承担（#24） |

**本规格据此推导出的不做项**：

| # | 不做 | 理由 |
|---|---|---|
| D6 | **框内任何动画** | #19 采纳 P3 减法；三处动效全部在 shell 表面（§0） |
| D7 | **伪造进度百分比** | §2.1 已证 Gradle 给不出真实完成度；不确定就用 spinner 如实表达 |
| D8 | **`--console=rich` / 解析 Gradle 内部进度** | 违反 spec §11.4 冻结的 flag 集；且会污染 `tail` 日志（§2.1 证据 4） |
| D9 | **常驻 tick 循环 / 空闲动画** | 向导空闲必须 0 帧/秒、0% CPU（§4.1）。spinner 只存在于长任务运行期间 |
| D10 | **进度条缓动 / 插值 / 弹性** | 确定式进度条只在请求完成时跳一格（§2.4）；插值需要 tick 循环 |
| D11 | **页面切换的逐帧过渡（淡入/滑动/擦除）** | §4.1 的四条代价 |
| D12 | **光标闪烁动画** | 输入态用静态 `caret` 字形（tokens.md §4）表示 |
| D13 | **预览面板的逐项「飞入」/ 数字滚动 / 树展开动画** | 预览更新是 diff 的自然结果（§3.4） |
| D14 | **骨架屏 / shimmer / 加载占位动画** | 无此需求；且属框内动画（D6） |
| D15 | **OSC 9;4 任务栏进度**（Gradle 会发） | 终端特定、无降级路径、收益低；不复制该行为 |
| D16 | **响铃（BEL）/ 声音提示** | 非视觉，且会污染 pipe 场景 |
| D17 | **平滑滚动 / 缓动 / 页面间滑动** | 终端无滚动容器；四页是顺序关系，无空间隐喻 |
| D18 | **resize 动画** | resize 出单帧（§4.3） |

---

## 6. 接口与依赖

| 方向 | 内容 |
|---|---|
| ← **#19 direction.md** | §2.2 保留预览面板 ⇒ 需全屏接管；§4 采纳 P3 减法 ⇒ 框内不做动画；宽度档位（≥100 / 80–99 / <80 / <60）本文 §3.5、§4.3 直接引用 |
| ← **#21 ADR-0001** | **已定 `ratatui 0.29 + crossterm`**，本文与之对齐（I2/I3 依赖其 `Buffer::diff()`；I4 依赖 crossterm 的 `SynchronizedUpdate`）。ADR 同时明确「无色/非 TTY 降级由 vinoa 自己保证」——本文 §2.6、§3.5、§4.3 是这一条在**动效**上的落实 |
| ← **#22 tokens.md** | `bar.full` / `bar.empty` **与 `spin.frames.*`** 及其 ASCII 回退（§4）；`accent`/`dim`/`faint`/`warn`；列位 `col.content.left`/`right`、`col.divider`。**字形与宽度由 tokens.md §4 定义，本文只定义帧率与推进时机** |
| → **#25 degradation.md** | 本文给出动效轴的降级矩阵（§2.6、§3.5、§4.3）；**触发条件与优先级由 #25 定义**，本文不定义，只要求 `NO_COLOR` 与「非 TTY」是**两条独立的轴**（H3） |
| → **#27 wizard-3-4.md** | ③ 页绘制模型必须与本文 §3.2 的 I1–I4 一致；共同锚点是 **A1（逻辑：裸打印 0 处）+ A3（物理：`\x1b[2J` == 0）**。**不要引用任何具体重绘次数**——它随交互步数变化（§3.1 方法论警告） |
| → **#24 shell.md** | 长任务进度行的**排版**归 #24；本文只定其**动效**（触发、帧率、降级）。`--verify` 的收尾行结构与 `versions` 的元信息版式不改 |

---

## 7. 盲区与未决

- **Gradle 未能在本沙箱实机运行**（`libnative-platform.so` 加载失败）。§2.1 的结论来自官方文档 + Gradle 源码 + vinoa 自身冻结的调用方式。若要补一次实机确认，最小验证是：在一个生成物里跑 `./gradlew build --console=plain --no-daemon` 并把输出重定向到文件，断言文件里既无进度条也无逐任务 `> Task` 行。
- **备用屏在 Windows 旧 conhost 上的行为**未实测；本文已给出主屏退化路径（§3.2），但真机观感属 #20 的盲区清单。
- **`refresh()` 的进度回调缝隙**需要实现侧改动（§2.4 末）；若实现选择不改签名，则阶段 B 的确定式进度条无法达成，只能退化为阶段 A 的不确定式 spinner —— **这是唯一被允许的替代，且必须记录**。
- **`spin.*` 字形已收进 tokens.md §4**（`spin.frames.unicode` = `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`，10 帧；`spin.frames.ascii` = `\|/-\`，4 帧）。字形与宽度由 [tokens.md §4](tokens.md) 定义，本文只定义**帧率与推进时机**（§2）。**宽度保证**：两组逐帧宽度均为 1（盲文 10 帧 `EAW = N`，ASCII 4 帧 `EAW = Na`），故帧循环不造成列位抖动。ASCII 档下 `spin.frames.unicode` **整组**替换为 `spin.frames.ascii`（不逐帧混用，不得出现盲文字形）。
- **重绘次数不可作为门禁（已更正）**：本规格早期版本曾把「`Platforms` 字节串出现次数 == 1」写成断言，其依据的观测值（7）来自 `grep -c` 字节计数，**强依赖交互步数**（#20 的 VT 逐格统计：最短路径 3 次、键入过滤词可达 15 次），会误导实现者去追一个不存在的目标值。现已改为：**逻辑层断言 A1（每个逻辑行只被逻辑绘制一次，裸打印 0 处）+ 物理层断言 A3（整屏清 `ED(2)` 全程 == 0）**，重绘次数只作**下界**（A1b）。物理重绘由 `Buffer::diff()` 决定，**不是断言对象**。

## 8. 复现

```bash
# 叠印证据（§3.1）与断言 A1 / A1b / A3–A6（§3.6）
python3 scripts/termcap/ptycap.py --out .tmpcap/wiz3.bin --cols 100 --rows 40 --timeout 22 \
  --keys '0.8:\r,1.6:\r,2.4:\r,3.2:\r,4.0:\x1b[B\r,5.0:\r,6.0:\r,7.0:\r' \
  --cwd /root/vinoa/.tmpcap -- /root/vinoa/target/debug/vinoa init demo -p com.example.demo

# 矩阵规模（§2.4 的分母来源）
python3 - <<'PY'
import re
t=open('data/version-matrix.toml',encoding='utf-8').read()
for s in re.split(r'^\[', t, flags=re.M):
    if s.split(']')[0].strip() in ('platform.paper','platform.folia'):
        v=[x.strip().strip('"') for x in re.search(r'versions\s*=\s*\[(.*?)\]', s, re.S).group(1).split(',') if x.strip()]
        print(s.split(']')[0], len([x for x in v if '-' not in x]))
PY
```
