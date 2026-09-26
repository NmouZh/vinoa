# vinoa 界面降级契约（#25）

> **状态**：锁定。本文是**降级触发条件与优先级的唯一来源**——`tokens.md` §2 明写把这块让给本文，`motion.md` §7 只给出动效轴的降级矩阵并声明「触发条件与优先级由 #25 定义」。
> **上游**：[direction.md](direction.md)（#19 方向 + §4 宽度档位）、[tokens.md](tokens.md)（#22 四档层级 / §3.5 无色替代 / §4 ASCII 回退 / §5 间距 / §6 层级）、[motion.md](motion.md)（#23 动效降级矩阵）、`docs/research/terminal-render-capabilities.md`（#20 探测事实）、[0001-render-stack.md](../../adr/0001-render-stack.md)（#21 Decision）。
> **下游**：#24 共享外壳、#26/#27 向导逐页、#28 help/versions/schema。它们**只引用本文的断言编号**，不重复定义触发条件。
> **读者**：实现票（把 §7 的断言表机械翻译成测试）与验收方。

**一句话**：颜色、字形、动效、宽度是**四条正交的降级轴**；每条轴的触发条件、优先级与产出形态在本文被写成可执行断言；`NO_COLOR` 空串**不**触发降级，「16 色」档由 vinoa 自己定义。

---

## 0. 责任归属与裁定索引

### 0.1 谁负责降级 —— 不是 ratatui

ADR-0001 Decision 第 2 条已裁定，本文落实：

| 事实（来源 #20 §2.2 / §4） | 后果 |
|---|---|
| `ratatui 0.29` 源码内 `NO_COLOR` / `no_color` / `ansi_color_disabled` / `is_terminal` **零命中** | ratatui **不会**替我们关颜色 |
| `ratatui` 不检测 TTY | ratatui **不会**替我们退非交互 |
| `crossterm` 的 `ansi_color_disabled()` 只管「发不发色」，**不管 TTY** | TTY 判定也是调用方的责任 |
| `inquire` 一旦自定义 `RenderConfig` 就不再处理 `NO_COLOR` | 自定义样式 = 自己接管 |

**因此：降级 100% 由 vinoa 自己的渲染入口保证。** 具体是一个**单一解析点**（§1.9）产出一个不可变上下文 `UiEnv`，渲染路径只读它、绝不重复读环境变量。

### 0.2 裁定索引（下游最常引用的 8 条）

| # | 裁定 | 本文位置 |
|---|---|---|
| 1 | `NO_COLOR` **空串不触发**（照 no-color.org，照 `crossterm`；**否决** `inquire` 的空串即禁用的行为） | §1.2 |
| 2 | 优先级链唯一：`--color` > `VINOA_COLOR` > `NO_COLOR` > `CLICOLOR_FORCE` > `CLICOLOR=0` > `TERM=dumb` > 非 TTY > 默认；能力层（Windows VT / `TERM` 缺失）不可被推翻 | §1.8 |
| 3 | **「16 色」档由 vinoa 自己定义**为「有彩但未探到真彩/256」的**兜底档**；**不调用** `crossterm::style::available_color_count()`（它默认返回 8，会与四档表打架） | §2.2 |
| 4 | 无色 ≠ 零 ANSI：**无色**只去颜色 SGR，**保留 bold**；**零 ANSI** 是无色的加强档（`TERM=dumb` / 非 TTY / `--json` 的 stdout / Windows 无 VT） | §2.3 |
| 5 | 无色档语义靠**字形 + 缩进 + 列位**，逐位判别式见 §2.4 | §2.4 |
| 6 | ASCII 字形回退与无色是**两条独立轴**，可单独命中也可同时命中 | §3 |
| 7 | 宽度档位与 `direction.md` §4 严格一致，另补 `< 40` 的 `minimal` 档 | §5.1 |
| 8 | 行尾 `\x1b[0m` 例外**只属于原型渲染器**，vinoa 实际输出路径必须真正零 `\x1b` | §7.3 |

---

## 1. 触发条件（偏好层 + 能力层）

### 1.1 两层模型

降级判定分两层，**最终结果 = 偏好层 ∧ 能力层**。这个划分不是修辞，它决定了 `--color=always` 能不能救命：

| 层 | 回答的问题 | 由什么决定 | `--color=always` 能否推翻 |
|---|---|---|---|
| **偏好层** | 用户**想不想**要颜色 | `--color`、`NO_COLOR`、`CLICOLOR*`、`TERM=dumb`、是否 TTY | **能**（它就在偏好层里，优先级最高） |
| **能力层** | 终端**能不能**正确显示 | Windows 是否有 VT；非 Windows 的 `TERM` 是否存在 | **不能**（`--color=always` 变不出 VT 支持） |

### 1.2 `NO_COLOR` 的精确语义 —— 空串裁定

`no-color.org`（Last updated 2026-09-23）原文：

> Command-line software which adds ANSI color to its output by default should check for a `NO_COLOR` environment variable that, when present and **not an empty string** (regardless of its value), prevents the addition of ANSI color.

两个上游实现不一致（来源 #20 §1.2 / §1.3）：

| 实现 | `NO_COLOR=""` | `NO_COLOR="0"` | `NO_COLOR` 未设置 |
|---|---|---|---|
| 规范 | **不禁用** | 禁用 | 不禁用 |
| `crossterm 0.29.0`（`colored.rs:75-79` + 自带测试 `colored.rs:310-319`） | 不禁用 ✅ | 禁用 | 不禁用 |
| `inquire 0.9.4`（`render_config.rs:350-355`，`env::var` 的 `Ok(_)`） | **禁用** ❌ | 禁用 | 不禁用 |

**裁定：vinoa 采用规范与 `crossterm` 的语义。**

```
触发降级  ⇔  NO_COLOR 已设置 且 值非空
不触发    ⇔  NO_COLOR 未设置，或已设置但值为空串
```

三条理由（不是「少数服从多数」）：

1. **规范是权威**：`NO_COLOR` 是跨工具约定，vinoa 若偏离，用户在 `NO_COLOR= vinoa …`（例如从某个模板脚本继承了空值）时会得到与其它工具相反的行为，且无法预期。
2. **`inquire` 的行为在本仓库是可消除的**：ADR-0001 Decision 第 4 条已把 `inquire` 移出向导路径。保留 `inquire` 只会把一处已知的不一致永久留在依赖树里。**这条裁定是 Decision 第 4 条的一个额外收益。**
3. **与 clap 的 `--help` 对齐**：`--help` 的着色由 `anstyle-query 1.1.5` 决定，它的 `no_color()` 是 `non_empty(NO_COLOR)`（`src/lib.rs:50-52`）——同样「空串不禁用」。若 vinoa 自己取相反语义，同一个进程里 `--help` 与正文会给出不同的颜色行为。

**补充细则**（照规范 FAQ，来源 #20 §1.1）：

- `NO_COLOR` 是**提示软件**，不是「让终端禁止显示颜色」。
- 用户级配置与命令行参数**应当覆盖** `NO_COLOR` ⇒ 本文 §1.7 定义 `--color`。
- `NO_COLOR` **只关颜色，不关 bold / underline / italic** ⇒ 本文 §2.3 把「无色」与「零 ANSI」分成两档。

### 1.3 `TERM=dumb`

`TERM=dumb` ⇒ 偏好层禁用颜色，且**同时**触发能力层的 ASCII 字形回退（§3.1）。两条轴一起动，但**记两个独立标志**（`color=off` 与 `glyphs=ascii`），因为 `--color=always` 可以推翻前者、推不翻后者（`TERM=dumb` 的终端本来就不保证制图字符）。

### 1.4 stdout 不是 TTY（pipe / 重定向 / CI）

判定用 `std::io::IsTerminal::is_terminal()`，**只判 stdout**（颜色是写到 stdout 或 stderr 的；本文约定人类文本走 stderr，但「非 TTY」这个信号以 stdout 为准，与 `anstream` 的 `AutoStream` 一致）。

- **stdin 不是 TTY** 是**另一件事**：它决定「向导能不能跑」（`src/wizard/mod.rs:20-23` 的 `should_run`，要求 stdin+stdout 都是 TTY），与颜色无关。
- 非 TTY ⇒ 偏好层禁用颜色，**且**能力层强制零 ANSI（§2.3），**且**动效关闭（§4.2）。

### 1.5 `--json`

`--json` **不是**一个全局颜色开关，而是**一个 sink 的属性**：

| sink | `--json` 下 |
|---|---|
| **stdout** | 数据通道：**零 ANSI，恰好一个 JSON 文档**。不可被任何偏好层设置覆盖（含 `--color=always`） |
| **stderr** | 人类通道：仍按 §1.8 解析结果着色 |

理由：spec §11.5 的硬契约是 stdout 恰好一个 JSON 文档；`\x1b` 会污染机器解析。而 stderr 着色不影响该契约。

### 1.6 Windows 旧 conhost

- crossterm 在 Windows 上**主动**调用 `SetConsoleMode` 打开 `ENABLE_VIRTUAL_TERMINAL_PROCESSING`（`crossterm-0.29.0/src/ansi_support.rs:5,17,21-30`）。
- `crossterm::ansi_support::supports_ansi()`（`ansi_support.rs:38-45`）的判定是：`enable_vt_processing()` 成功 **或** `TERM` 已设置且 `!= "dumb"`。
- **`ansi_support` 模块只在 `#[cfg(windows)]` 下导出**（`crossterm-0.29.0/src/lib.rs:257-259`）——因此能力层判定必须写 `#[cfg(windows)]` 分支，**不能在非 Windows 上引用它**（否则编译失败，且 `clippy --all-targets -D warnings` 会红）。

**裁定：Windows 上 `!supports_ansi()` ⇒ 能力层强制「零 ANSI + ASCII 字形」，且 `--color=always` 不能推翻。**

理由：旧 conhost 不解析 VT 序列，发 ANSI 只会把 `\x1b[38;5;74m` 当字面量打在屏幕上——这不是「颜色不好看」，是**输出损坏**。`--color=always` 是偏好，偏好不能创造能力。

### 1.7 用户强制：`--color=auto|always|never`

**裁定：需要，且作为 clap 的 global arg 加入。**

```
vinoa --color=<auto|always|never> <子命令> [args]
```

| 值 | 语义 |
|---|---|
| `auto`（默认） | 走 §1.8 的完整链 |
| `always` | 短路偏好层的禁用项（`NO_COLOR` / `CLICOLOR` / `TERM=dumb` / 非 TTY），**不**短路能力层（§1.1） |
| `never` | 终局无色，优先级最高 |

理由：

1. `no-color.org` FAQ 第 2 条**明确要求**「用户级配置文件与命令行参数应当覆盖 `NO_COLOR`」。没有 CLI 出口，这条规范无法满足——用户全局导出 `NO_COLOR=1` 后，没有任何办法对单次命令开颜色。
2. 环境变量出口不够用：CI 里单条命令要开颜色时，改环境变量比加一个 flag 麻烦且不可局部化。
3. 默认 `auto` ⇒ **不改变任何现有行为**，向后兼容。

**这是对 `docs/spec/vinoa-cli.md` §3.1 的一处增补**（该表当前没有 `--color`）。增补登记归 #28（它拥有 `--help` 面）；本文只定义语义。§11.4 冻结的「附加 flag」集合是传给 **Gradle** 的，不是 vinoa 自己的顶层 flag，因此本增补不构成对 §11.4 的违反。

**同时接受 `VINOA_COLOR` 环境变量**（取值同上），优先级**低于** `--color`、**高于** `NO_COLOR`。理由：`--help` 由 clap 在 vinoa 的代码运行**之前**打印（`src/cli.rs:369-377`，`DisplayHelp` 分支直接 `err.print()`），因此 `--color` 对 `--help` 不生效（见 §6.5）；环境变量是唯一能同时覆盖正文与 `--help` 之外的出口。

### 1.8 优先级链（唯一，第一个命中者胜）

**偏好层**（决定 `wanted: bool`）：

| 序 | 条件 | 结果 |
|---|---|---|
| 1 | `--color=never` | `wanted = false` |
| 2 | `--color=always` | `wanted = true`（跳过 3–8） |
| 3 | `VINOA_COLOR=never` / `=always` | 同 1 / 2 |
| 4 | `NO_COLOR` 已设置且**非空** | `wanted = false` |
| 5 | `CLICOLOR_FORCE` 已设置且**非空** | `wanted = true` |
| 6 | `CLICOLOR == "0"` | `wanted = false` |
| 7 | `TERM == "dumb"` | `wanted = false` |
| 8 | stdout 不是 TTY | `wanted = false` |
| 9 | 以上都不命中 | `wanted = true` |

序 4 > 5 > 6 的顺序**照抄 `anstream`**（`src/auto.rs:202-209`：`no_color()` → `clicolor_force()` → `clicolor_disabled`），目的是让 `--help` 与正文在环境变量层面尽量一致。

**能力层**（决定 `can: bool` 与档位，**不可被偏好层推翻**）：

| 序 | 条件 | 结果 |
|---|---|---|
| K1 | Windows 且 `!supports_ansi()` | `can = false`（零 ANSI + ASCII 字形） |
| K2 | 非 Windows 且 `TERM` **未设置** | `can = false`（零 ANSI） |
| K3 | `wanted == false` | 无色 |
| K4 | 否则 | 有彩，档位按 §2.1 |

K2 的理由是**与 `--help` 对齐**：`anstyle-query` 的 `term_supports_color()` 在非 Windows 下「`TERM` 未设置 → false」（`src/lib.rs:56-72`）。若 vinoa 自己取相反默认，同一个终端里 `--help` 无色而正文有色。

**零 ANSI 闸**（`sgr_allowed: bool`，决定 bold `1` 与 reset `0` 发不发；这是 §2.3「无色 ≠ 零 ANSI」的落点）：

| 序 | 条件 | `sgr_allowed` |
|---|---|---|
| S1 | K1 或 K2 命中（能力层已判 `can = false`） | `false` —— **硬**，任何偏好都推不翻 |
| S2 | `--color=never` | `false` —— 硬 |
| S3 | `TERM == "dumb"` 且**无显式 force**（`--color=always` 或非空 `CLICOLOR_FORCE`） | `false` |
| S4 | stdout 非 TTY 且**无显式 force** | `false` |
| S5 | 否则 | `true` |

**`sgr_allowed` 与 `wanted` 是两把不同的闸，必须分开算**——这正是 A5 与 A12 能同时成立的原因：

| 场景 | `wanted` | `sgr_allowed` | 实际输出 |
|---|---|---|---|
| `NO_COLOR=1` + pty | `false`（序 4） | `true`（S5） | 无颜色 SGR，**bold 保留** |
| `TERM=dumb` + pty | `false`（序 7） | `false`（S3） | 零 `\x1b`（无颜色也无 bold） |
| `CLICOLOR_FORCE=1 TERM=dumb` + pty | `true`（序 5 高于序 7） | `true`（S3 被 force 豁免） | 有颜色、有 bold |
| `--color=always` + 非 TTY | `true`（序 2） | `true`（S4 被 force 豁免） | 有颜色、有 bold |
| `--color=never` + pty | `false`（序 1） | `false`（S2） | 零 `\x1b` |
| `env -u TERM` + pty（非 Windows） | `true`（序 9） | `false`（S1，K2） | 零 `\x1b` |
| Windows `!supports_ansi` + `--color=always` | `true`（序 2） | `false`（S1，K1） | 零 `\x1b` + ASCII 字形 |

**S3/S4 可被 force 推翻、S1/S2 不可**——分界线是「启发式 vs 明确声明」：

- S3/S4 是**启发式**：`TERM=dumb` 与「stdout 是管道」都只**通常**意味着「别发转义」，而用户可以用 `--color=always` 明确说「我知道我在做什么」。CI 里把带颜色的输出重定向进日志查看器就是真实场景。
- S1 是**能力**：Windows 无 VT 时转义序列不被解析，发出去就是字面量垃圾。偏好不能创造能力。
- S2 是**用户的终局指令**：`never` 就是 never，没有更高优先级的出口（`--color` 只能出现一次，clap 的 last-wins 已在偏好层解析完毕）。

**这条分界也解释了 A5 与 A12 的差异**：`NO_COLOR` 只关颜色（S5 仍为 `true`，bold 保留，照规范 FAQ 第 3 条）；`TERM=dumb` 连 SGR 一起关（S3）。两者在偏好层的效果相同（都是 `wanted = false`），在零 ANSI 闸上不同。

### 1.9 实现约束：单点解析、可注入

```rust
// 唯一的探测点。进程生命周期内只构造一次。
pub struct UiEnv {
    pub wanted_color: bool,     // §1.8 偏好层
    pub sgr_allowed: bool,      // §1.8 零 ANSI 闸（bold/reset 发不发）
    pub can_ansi: bool,         // §1.8 能力层（Windows VT / TERM 存在）
    pub is_tty: bool,           // stdout
    pub is_utf8: bool,          // §3.1
    pub tier: ColorTier,        // §2.1 的 final_tier()
    pub glyphs: GlyphSet,       // §3.1
    pub width: u16,             // §5.4
    pub json_mode: bool,
    pub motion: bool,           // §4
}

impl UiEnv {
    /// 进程内唯一调用点：读真实环境变量。
    pub fn detect() -> Self { /* 读 env + crossterm + clap 解析结果 */ }

    /// 测试唯一入口：所有字段显式注入，不读环境。
    pub fn from_parts(p: UiEnvParts) -> Self { /* 纯函数 */ }

    pub fn color(&self, sink: Sink) -> ColorMode { /* sink 决定 §1.5 */ }
    pub fn ansi(&self, sink: Sink) -> bool { /* §1.8 的 S 表；sink=Stdout 且 json_mode ⇒ false */ }
}
```

**`wanted_color` 与 `sgr_allowed` 必须是两个字段**：它们不是同一个布尔的两面（见 §1.8 的场景表）。把二者合成一个 `color: bool` 会让 A5（`NO_COLOR` 保留 bold）与 A12（`TERM=dumb` 无 bold）中的一条必然失败——**这是实现票最容易踩的坑**。

**硬约束**：

- `UiEnv::detect()` 在整个进程里**只允许有一个调用点**（`src/lib.rs::run()` 顶部）。
- 渲染路径（`src/ui/render/**`）内**不得出现** `std::env::var` / `crossterm::style::available_color_count` / `is_terminal()`。全部经 `UiEnv` 注入。
- 理由不只是可测性：`crossterm` 的 `ansi_color_disabled()` 是**记忆化**的（`colored.rs:81-87` 的 `Once`），运行中改环境变量不生效；`force_color_output()` 必须在任何输出之前调用一次。单点解析把这个时序风险消掉。

### 断言组 A —— 触发条件与优先级（17 条）

统一夹具：真实 pty 用 `python3 scripts/termcap/ptycap.py --out <out> --cols 100 --rows 40 -- <vinoa> …`（无 `--` 则非 TTY 路径用普通管道）。`<vinoa>` = `target/debug/vinoa`。SGR 集合由 §7.2 的解析器提取。

- **断言 A1**：`--color=never` + pty → 输出 SGR 集合为空（零 `\x1b`）。
- **断言 A2**：`--color=always NO_COLOR=1` + pty → SGR 集合**非空**且含颜色码（覆盖 `NO_COLOR`）。
- **断言 A3**：`--color=always vinoa versions | cat` → stdout 含颜色 SGR（覆盖非 TTY）。
- **断言 A4**：`--color=always vinoa versions --json | cat` → stdout 零 `\x1b` 且 `json.loads(stdout)` 成功（stdout 不受 `--color` 影响）。
- **断言 A5**：`NO_COLOR=1` + pty → 无颜色 SGR（`3x` / `4x` / `9x` / `38;*` / `48;*` / `39` / `49` 计数为 0），但 SGR `1`（bold）**允许存在**。
- **断言 A6**：`NO_COLOR= vinoa versions` + pty → 颜色 SGR 计数 **> 0**（空串不触发降级）。
- **断言 A7**：`NO_COLOR=0` + pty → 颜色 SGR 计数 = 0（任何非空值都触发）。
- **断言 A8**：`CLICOLOR_FORCE=1 TERM=dumb` + pty → 颜色 SGR 计数 > 0（序 5 高于序 7）。
- **断言 A9**：`CLICOLOR=0` + pty → 颜色 SGR 计数 = 0。
- **断言 A10**：`CLICOLOR=0 --color=always` + pty → 颜色 SGR 计数 > 0。
- **断言 A11**：`NO_COLOR=1 CLICOLOR_FORCE=1` + pty → 颜色 SGR 计数 = 0（序 4 高于序 5）。
- **断言 A12**：`TERM=dumb` + pty → 输出零 `\x1b`（含无 bold）。
- **断言 A13**：`vinoa versions | cat` → 零 `\x1b`。
- **断言 A14**：`env -u TERM vinoa versions` + pty（非 Windows）→ 零 `\x1b`。
- **断言 A15**：`vinoa --color=bogus versions` → exit 64，stderr 含 `auto`、`always`、`never` 三个取值。
- **断言 A16**（Windows-only）：`!supports_ansi()` 的 conhost + `--color=always` → 零 `\x1b` 且字形为 ASCII（能力层不可被偏好推翻）。
- **断言 A17**：`--color=always NO_COLOR=1 CLICOLOR=0 TERM=dumb vinoa versions | cat` → 颜色 SGR 计数 > 0（`--color=always` 是唯一能同时推翻 4/6/7/8 的入口）。

---

## 2. 色深档位：真彩 → 256 → 16 → 无色

### 2.1 vinoa 自己的档位探测

```rust
pub enum ColorTier { TrueColor, Ansi256, Ansi16, None }

/// 能力档位：只回答「这个终端最多能显示到什么色深」，不含偏好。
fn detect_capability_tier(env: &Env) -> ColorTier {
    let colorterm = env.get("COLORTERM").unwrap_or("");
    let term      = env.get("TERM").unwrap_or("");
    if contains_ci(colorterm, "truecolor") || contains_ci(colorterm, "24bit")
    || contains_ci(term,      "truecolor") || contains_ci(term,      "24bit") {
        ColorTier::TrueColor
    } else if contains_ci(term, "256") {
        ColorTier::Ansi256
    } else {
        ColorTier::Ansi16          // 兜底：含 TERM 未知 / 8 色 / 16color
    }
}

/// 最终档位 = 能力档位 ∧ 偏好 ∧ 能力门（§1.8 的 K 表）。只有这里会产出 None。
fn final_tier(env: &Env, wanted: bool) -> ColorTier {
    if !wanted || !env.can_ansi { ColorTier::None } else { detect_capability_tier(env) }
}
```

**探测与裁定分成两个函数是刻意的**：`detect_capability_tier` 是纯能力查询（可单测：`TERM=xterm-16color` → `Ansi16`，与 `NO_COLOR` 无关）；`final_tier` 是唯一的裁定点。任何调用方若直接拿能力档位去着色，就绕过了 §1.8 的偏好层——**这是实现票最容易犯的错**。

**vinoa 不调用 `crossterm::style::available_color_count()`。** 理由见 §2.2。

### 2.2 「16 色」这一档从哪来 —— 显式裁定

**事实（#20 §2.1 / ADR-0001 Decision 第 3 条，已核实）**：`crossterm-0.29.0/src/style.rs:163-178`：

```rust
const DEFAULT: u16 = 8;              // ← 默认是 8，不是 16
env::var("COLORTERM").or_else(|_| env::var("TERM"))
    .map_or(DEFAULT, |x| match x {
        _ if x.contains("24bit") || x.contains("truecolor") => u16::MAX,
        _ if x.contains("256") => 256,
        _ => DEFAULT,
    })
```

即 `available_color_count()` 的**值域是 `{8, 256, u16::MAX}`——它永远不返回 16**。所以 `tokens.md` §2 的四档里，「16」这一档**没有任何上游依据**，必须由本文裁定。

**裁定：把「16 色」定义为「有彩但未探到真彩 / 256」的兜底档，而不是新增一档 8 色。**

三条理由：

1. **16 档的取值本来就是「裸 SGR 色相码」**（`tokens.md` §3.1 的 `92` `91` `93` `96` `97` `37` `90`），它们在任何 ≥ 8 色的 ANSI 终端上**都是合法序列**。不存在「8 色终端收到 16 色码会坏掉」的情况——最坏是终端把亮色码按基色渲染。
2. **最坏情况的语义仍然成立**：若终端忽略亮度、把 `91` 当 `31`、把 `96` 当 `36`，`tokens.md` §3.4 的规则正是「**保角色，不保色相距离**」——ok→绿、err→红、warn→黄、accent→青的**色相族**全部保留。因此 8 色能力落在 16 档内，不丢语义。
3. **新增第 5 档会破坏 tokens.md 的完备性论证**：`tokens.md` §1 论证「8 位是完备的」，§2 论证「四档是完备的」。为了一个 crossterm 的 `const DEFAULT` 去加第 5 档，等于让上游一个自带注释「This does not always provide a good result.」的启发式值决定我们的规格面。

**被否掉的两种做法**：

| 否掉 | 理由 |
|---|---|
| 直接用 `available_color_count()` 的返回值做档位 | 它返回 `8`，而 `tokens.md` 没有 8 色档 ⇒ 要么把 8 悄悄当 16（不可见的隐式映射），要么新增一档（见上）。而且它**不检查 `NO_COLOR`**，语义上只管能力不管偏好，容易被误用成总开关。 |
| 新增 8 色档（用 `30–37` 无亮色） | 需要新色值、新对比度表、新层级论证；而收益为零——`30–37` 与 `90–97` 在同一终端上的色相族相同，只是亮度不同，而亮度恰恰是 16 档最弱的那根轴（`tokens.md` §3.4 已声明 16 档靠字形与列位兜底）。 |

**顺带裁定**：`TERM` 含 `16color`（如 `xterm-16color`）**也**落在 `Ansi16`——它的取值与兜底档完全一致，不需要单独分支。

### 2.3 逐级放弃什么（把 tokens.md §2 变成可判定规则）

| 档 | 触发（§2.1） | 发出的 SGR | 放弃什么 | 是否发 bold |
|---|---|---|---|---|
| `TrueColor` | `COLORTERM` / `TERM` 含 `truecolor`/`24bit` | `38;2;r;g;b` | 无 | 是 |
| `Ansi256` | `TERM` 含 `256` | `38;5;n` | 精确色相；保留 8 位可辨性 | 是 |
| `Ansi16` | 兜底（未知 / 8 色 / `16color`） | 裸 `3x` / `9x` | 第 4 级无彩阶梯（`faint` 与 `border` 合并到 `90`）；底色块 | 是 |
| `None` | §1.8 的 K3（`wanted == false`）或 K1/K2（`can = false`） | **无颜色 SGR** | 全部颜色；语义交给字形 + 缩进 + 列位 | 见 §1.8 的 `sgr_allowed`（`NO_COLOR` 下为**是**） |

**「无色」与「零 ANSI」是两档，不是一档**（照 `no-color.org` FAQ 第 3 条：`NO_COLOR` 只关颜色，不关 bold）：

| 档 | 定义 | 触发（§1.8 的 S 表） |
|---|---|---|
| **无色**（color-off） | 不发任何**颜色** SGR：`30–37` `40–47` `90–97` `100–107` `38;*` `48;*` `39` `49` | `NO_COLOR` 非空 / `CLICOLOR=0` / `TERM=dumb` / 非 TTY / `--color=never` / 能力层 `can = false` |
| **零 ANSI**（ansi-off） | 不发任何 `\x1b`（含 bold `1`、reset `0`） | `--color=never` / Windows 无 VT / `TERM` 未设置 / `--json` 的 stdout / `TERM=dumb`（无 force）/ stdout 非 TTY（无 force） |

关系：**ansi-off ⊃ color-off**（零 ANSI 一定无色；无色不一定零 ANSI）。

**注意 `TERM=dumb` 与非 TTY 在两张表里都出现，但语义不同**：它们在偏好层关掉颜色（必然），在零 ANSI 闸里关掉 SGR **除非**用户显式 force（§1.8 的 S3/S4）。所以「`TERM=dumb` + `--color=always`」的结果是**有颜色、有 bold**——而「Windows 无 VT + `--color=always`」的结果是**零 ANSI**。差别在于前者是启发式、后者是能力。

**为什么保留 bold**：`tokens.md` §6 的层级表把 **L1↔L2 的非色轴定为「字重（bold vs regular）+ 位置」**。若无色档把 bold 也去掉，L1 与 L2 只剩位置轴；而在 `< 80` 宽度档（§5.1）连边框都被丢弃时，位置轴会被削弱。保留 bold 让「去掉颜色后层级依然成立」这条 `tokens.md` §6 的硬约束在最弱的环境里仍然有两条轴。

**为什么 `TERM=dumb` 与「非 TTY」默认是 ansi-off**：这两者对应的接收端（哑终端 / 日志文件）默认不解析任何 SGR，发 bold 可能变成字面 `^[[1m`。用户显式 force 时才认为他知道接收端能处理。

**无色档不得「发一个看起来一样的默认前景色」**：`tokens.md` §3.5 已裁定 `fg` 在无色档**完全不发色码**，理由是「前者会让 `NO_COLOR` 的输出里出现 `38;5;252`」。本文把它变成断言 B7。

### 2.4 无色档的语义判别式（本票核心）

`tokens.md` §3.5 的替代表在这里变成**可判定规则**。设 `frame` 为同一帧的文本（已去 SGR），`col(x)` 为显示列号（CJK 宽字符占 2 列）：

| # | 规则（无色档下必须成立） | 判定方式 |
|---|---|---|
| **R1** | 每个 `err` 承载行**以 `✗ ` 开头**（ASCII 字形档为 `x `） | `line.startswith("✗ ")`；且该行内 `✗` 出现 **1** 次 |
| **R2** | 每个 `ok` 承载行**以 `✓ ` 开头**（ASCII 档 `+ `） | 同上 |
| **R3** | 每个 `warn` 承载行**以 `! ` 开头** | `! ` 已是 ASCII，ASCII 档不变 |
| **R4** | **当前项独占标记列**：整帧在 `col.cursor`（= 3）列上有且仅有一个非空格字形，且该字形 ∈ {`❯`, `>`} | 逐行取 `col 3` 的字符，计数 |
| **R5** | `dim` 承载文本的起始列 = 同块 `fg` 文本起始列 − `indent.unit`（= 2）；行内形态则前置 `· ` | 比较同块两行的首个非空格列 |
| **R6** | `faint` 承载文本满足 R5 **且**（行首为 `· ` **或** 右端对齐到 `col.content.right`） | 同上 + `line.rstrip()` 长度判定 |
| **R7** | **只换轴，不减信息**：无色帧与真彩帧的**行数相等**，且 `✗`/`✓`/`!`/`❯` 四种标记的计数**逐项相等** | 见断言 B9 |
| **R8** | 无色档**不得**通过删除内容来「变干净」：无色帧的**非空格字符总数 ≥ 真彩帧的 90%** | 见断言 B9 |

R7/R8 是 `tokens.md` §3.5「降级只允许换轴，不允许减信息」的可执行形式。R8 的 90% 下界是为了容纳「`dim` 改缩进带来的空格位移」与「`faint` 新增 `· ` 前缀」——它允许**字形替换**（`✓`→`+` 等宽），只禁止**成片删除**。

### 断言组 B —— 色深与无色语义（10 条）

- **断言 B1**：`COLORTERM=truecolor TERM=xterm-256color` + pty → 输出含 `38;2;`。
- **断言 B2**：`env -u COLORTERM TERM=xterm-256color` + pty → 含 `38;5;` 且**不含** `38;2;`。
- **断言 B3**：`env -u COLORTERM TERM=xterm-16color` + pty → 含裸 16 色码（正则 `\x1b\[(3[0-7]|9[0-7])m`）且**不含** `38;5;` / `38;2;`。
- **断言 B4**：`env -u COLORTERM TERM=xterm` + pty → 同 B3（未知 TERM 落 16 档兜底，不是 8 档、不是无色）。
- **断言 B5**：`grep -rn "available_color_count" src/` → 0 命中（vinoa 不采用 crossterm 的 8 色默认）。
- **断言 B6**：`TERM=xterm-16color` + pty → 每一行 `err` 承载文本同时含 `✗`（16 档下 `err` 的对比度仅 4.10，不得只靠颜色，`tokens.md` §3.4）。
- **断言 B7**：`NO_COLOR=1` + pty → 输出**不含** `38;5;252` 且**不含** `39`（无色 ≠ 发默认前景色）。
- **断言 B8**：`NO_COLOR=1` + pty，对每一帧运行 §2.4 的 R1–R6 判别器 → 0 violations。
- **断言 B9**：同一输入分别在 `COLORTERM=truecolor` 与 `NO_COLOR=1` 下渲染同一帧 → 行数相等；`✗`/`✓`/`!`/`❯` 计数逐项相等；非空格字符数之比 ≥ 0.9。
- **断言 B10**：`NO_COLOR=1` + pty 的某一帧 → 至少 1 行满足 R5 且至少 1 行满足 R6（缩进/`·` 确实在承担 `dim`/`faint`）。

---

## 3. 字形轴（ASCII 回退）

### 3.1 触发条件

**字形档与颜色档完全独立**（`tokens.md` §4 明写「ASCII 回退档与无色档是两个独立的轴」）。判定：

```rust
pub enum GlyphSet { UnicodeRound, UnicodeSquare, Ascii }

fn detect_glyphs(env: &Env, win_ansi: bool) -> GlyphSet {
    if env.term_is_dumb() || !env.is_utf8() || !win_ansi { GlyphSet::Ascii }
    else if TERM_SQUARE_BOX.contains(env.term())     { GlyphSet::UnicodeSquare }
    else                                              { GlyphSet::UnicodeRound }
}
```

`is_utf8()` 的判定（取第一个**非空**值，转小写）：

| 序 | 变量 | 命中 `utf-8` 或 `utf8` → UTF-8 |
|---|---|---|
| 1 | `LC_ALL` | |
| 2 | `LC_CTYPE` | |
| 3 | `LANG` | |
| 4 | 全未设置 | **非 UTF-8**（保守 → ASCII） |

Windows 例外：`is_utf8()` 在 Windows 上恒为 `true`（code page 探测不可靠），由 `!win_ansi` 兜底成 ASCII。

`TERM_SQUARE_BOX`（**DEC 制图扩展不可用、但 locale 是 UTF-8** 这一格；`tokens.md` §4 定义了「这个格子存在」，格子里的终端清单由本文定义——它属于降级触发条件，归本文）：

```
{"linux", "vt100", "vt102", "vt220", "ansi", "cons25", "cygwin", "sun", "d200", "interix"}
```

这是一个**单一常量表**——新增终端只改这一处。表内元素的共同点是：它们是「ANSI 色可用、Unicode 制图不可用」的历史终端。

### 3.2 映射（引用 `tokens.md` §4，本文不重复定义字形）

| 用途 | Unicode | ASCII 回退 |
|---|---|---|
| 当前项光标 `cursor` | `❯` | `>` |
| 导航当前位置 `nav.current` | `▸` | `>` |
| 导航已完成 `nav.done` | `✓` | `+` |
| 导航未来页 `nav.future` | `·` | `-` |
| 导航分隔 `nav.sep` | `›` | `>` |
| 成功勾 `ok` | `✓` | `+` |
| 失败叉 `err` | `✗` | `x` |
| 警告 `warn` | `!` | `!`（已 ASCII） |
| 分隔符 `sep` | `·` | `-` |
| 树·枝 / 末 / 竖线 | `├─` / `└─` / `│` | `\|-` / `` `- `` / `\|` |
| 框·圆角 | `╭╮╰╯` | `+` |
| 框·直角 | `┌┐└┘` | `+` |
| 框·横竖 | `─` `│` | `-` `\|` |
| 进度条满 / 空 | `█` / `░` | `#` / `-` |
| 输入光标 `caret` | `▏` | `\|` |
| 方向键提示 | `↑↓` `← →` | `^v` `<- ->` |
| spinner（帧序列） | `spin.frames.unicode` | `spin.frames.ascii` |

**已是 ASCII、无需回退的三个**（`tokens.md` §4 明写）：`check.on` = `[x]`、`check.off` = `[ ]`、`warn` = `!`。它们在 Unicode 档与 ASCII 档**逐字节相同**，因此断言只需覆盖「它们存在」，不需要覆盖「它们被替换」。

**宽度不变式**：所有 Unicode 字形与它的 ASCII 回退**显示宽度相同**（框线全部宽 1；`├─`/`|-` 都是 2；`⠋` 与 `|` 都是 1）。因此字形回退**不改变任何列位**——这是「可以放心降级」的依据（**字形取值与逐帧宽度实测见 [tokens.md](tokens.md) §4**，本文只引用 token 名、不复制字形）。**唯一例外**：`key.leftright` 的 `← →`（3 列）→ `<- ->`（5 列），它只出现在页脚提示行且是行尾内容，不影响列位对齐。

**明确不做**：ASCII 档下**不得**出现任何盲文字形（`U+2800–U+28FF`）。`spin.frames.ascii` 的定义在 [tokens.md](tokens.md) §4（帧率与推进时机在 [motion.md](motion.md) §2），本文只把它变成断言 C8。

### 3.3 边框与树形在窄终端下的行为

**窄终端不触发 ASCII 回退。** 宽度与字形是两条独立的轴：

| 条件 | 边框 | 树形 |
|---|---|---|
| `GlyphSet::UnicodeRound` | 圆角 `╭╮╰╯` | `├─ └─ │` |
| `GlyphSet::UnicodeSquare` | 直角 `┌┐└┘` | 同上 |
| `GlyphSet::Ascii` | `+ - \|` | `\|- ` `` `- `` `\|` |
| 宽度 `< 80`（任意字形档） | **丢弃整个边框** | 保留（预览面板已丢，树形随预览消失） |
| 宽度 `< 60` | 丢弃 | 保留（若有树形，改为**扁平列表 + 缩进 2**，`tokens.md` §5.3 的 `indent.tree.unit` 在极窄档失效） |

### 断言组 C —— 字形（9 条）

- **断言 C1**：`TERM=dumb` + pty → 输出不含任何 `U+2500–U+257F`（制图块）与 `U+2580–U+259F`（块元素）字符。
- **断言 C2**：`LC_ALL=C TERM=xterm-256color` + pty → 不含制图块字符；含 `+`、`\|`、`-` 组成的框线。
- **断言 C3**：`LC_ALL=C.UTF-8 TERM=xterm-256color` + pty → 含 `╭` 与 `╮`。
- **断言 C4**：`LC_ALL=C.UTF-8 TERM=linux` + pty → 含 `┌` 与 `┐`，**不含** `╭`。
- **断言 C5**：`NO_COLOR=1 LC_ALL=C.UTF-8 TERM=xterm-256color` + pty → **仍含** `╭`（无色不触发 ASCII 回退）。
- **断言 C6**：`LC_ALL=C TERM=xterm-256color` + pty → 颜色 SGR 计数 **> 0**（ASCII 回退不触发无色）。
- **断言 C7**：`LC_ALL=C` 的预览树 → 前缀逐字符匹配 `\|- ` / `` `- `` / `\|   `，且**不含** `├` `└` `│`。
- **断言 C8**：`LC_ALL=C` 下运行长任务 → 输出不含 `U+2800–U+28FF`（无盲文）。
- **断言 C9**：`LC_ALL=C` + pty 与 `LC_ALL=C.UTF-8` + pty 的同一帧 → `[x]`、`[ ]`、`!` 三种字形逐字节相同（它们本就是 ASCII）。

---

## 4. 动效轴

### 4.1 三条轴的关系（与 `motion.md` H3 一致）

`motion.md` §1 的 H3：**「无色 ≠ 不动。颜色与运动是两条正交的降级轴：`NO_COLOR` 下动画照常，只去掉 SGR；非 TTY 下动画才关闭。」** 本文落实这条，并把它并入 §1 的两层模型：

| 轴 | 关掉动效的条件 |
|---|---|
| 颜色（§2.3） | `NO_COLOR` / `CLICOLOR=0` / `TERM=dumb` / 非 TTY / `--color=never` / 能力层 `can = false` |
| **动效** | **仅**：非 TTY（stdout）**或** `TERM=dumb` |
| 字形（§3） | `TERM=dumb` / 非 UTF-8 / Windows 无 VT / `TERM ∈ TERM_SQUARE_BOX` |

即：**`NO_COLOR` 单独出现时动画照常**；`--color=never` 也**不**关动效（用户说的是「别上色」，不是「别动」）。

**`--color=always` 与 `CLICOLOR_FORCE` 不关动效、也不开动效**：动效轴只看「有没有 TTY / 终端是否 dumb」，与 §1.8 的 force 语义无关。`--color=always vinoa … --verify | cat` 会得到**有颜色但无动效**的输出（颜色被 force 打开，原地重绘仍被非 TTY 禁止）——这正是 D1 在加了 `--color=always` 后仍要求零 `\r` 的原因。

### 4.2 非 TTY / `TERM=dumb` 下的替代

直接引用 `motion.md` §2.6 的降级矩阵，本文只补三条**判定规则**：

1. **零原地重绘**：非 TTY 档下输出中不得出现 `\r`（回车）与 `\x1b[K` / `\x1b[2J`——后者已被 ansi-off 覆盖，但 `\r` 是**裸控制字符**，不在 SGR 管辖内，必须单独禁止）。
2. **离散行**：长任务只出**开始行 + 每个事件 1 行**，`--verify` 总计 ≤ 3 行，`versions --refresh` 总计 ≤ 4 行（`motion.md` §2.6）。
3. **进度条字形整条省略**：`TERM=dumb`（仍是 TTY）档下不出现 `bar.full`（`█` / `#`）与 `bar.empty`（`░` / `-`）的**重复序列**，进度改由 `n/N` 数字承担（`motion.md` §2.6 的 `刷新中… 23/66 · paper 1.21.4`）。**非 TTY 档下进度改由离散行承担**，`n/N` 不再是必需项——`motion.md` §2.6 明写非 TTY 只出「开始行 + 每个事件 1 行」。

### 断言组 D —— 动效（5 条）

- **断言 D1**：`vinoa init … --verify | cat` → 输出零 `\x1b` 且**零 `\r`**。
- **断言 D2**：`vinoa versions --refresh | cat` → stdout 行数 ≤ 1（JSON 或单文档），stderr 行数 ≤ 4。
- **断言 D3**：`NO_COLOR=1` + pty 跑长任务 → 颜色 SGR 计数 = 0，但**同一行在两个不同时刻的内容不同**（spinner 仍在转）——按 `motion.md` §2.6 的 TTY+无色行。
- **断言 D4**：`TERM=dumb` + pty 跑长任务 → 无字形帧循环：同一行在 `t` 与 `t+1` 的差异**只允许出现在秒数字段**。
- **断言 D5**：`TERM=dumb` + pty 跑长任务 → 不出现 `█` / `░` / `#` / `-` 组成的进度条序列，且含 `n/N` 形式的数字（覆盖 `motion.md` §2.6 的 `TERM=dumb` 行）。

---

## 5. 宽度退化

### 5.1 宽度档位表（与 `direction.md` §4 严格一致，补 `< 40`）

| 档 | 宽度 `W`（列） | 形态 | 丢弃 | token |
|---|---|---|---|---|
| `full` | `W ≥ 100` | 完整 REC：整屏圆角边框 + 顶部导航条 + 右侧预览面板 | — | `screen.w.full = 100` |
| `no-preview` | `80 ≤ W ≤ 99` | 表单占满整宽；边框与导航条保留 | 预览面板、`col.divider` 分栏线 | `screen.w.preview.min = 100` |
| `no-border` | `60 ≤ W ≤ 79` | P3 式连续流；导航条压成一行页位（`✓1 工程 › ▸3 目标服务端 · 3/4`） | 整屏边框（`box.*` 全部字形） | `screen.w.noborder.min = 60` |
| `single-column` | `40 ≤ W ≤ 59` | 单栏：标签与取值**分行**，取值缩进 `indent.unit` | 两列布局（`label.w = 13` 失效）、树形前缀（改扁平 + 缩进 2） | `screen.w.single.min = 40` |
| `minimal` | `W < 40` | 见 §5.3 | 全部多栏、进度条、预览、导航条页名（只留 `n/4`） | `screen.w.hard.min = 40` |

**`< 40` 是本文对 `direction.md` §4 的增补**（direction 只定义到 `< 60`）。理由是可算的，不是拍的：单栏形态的最小可用宽度 = `col.content.left`(3) + 最短标签宽度(≈15) + 2 + 最短可读取值宽度(≈20) = **40**。低于 40 列，标签与取值分行后单行仍放不下一个完整取值。

**W < 40 时的契约**：vinoa **不保证**可读性，但**保证**三件事——(a) 不 panic；(b) 不横向滚动（每行显示宽度 ≤ `W`）；(c) 非 TTY 下零 ANSI。

**与 `direction.md` §4 的逐条对照**（验收用）：

| direction.md §4 | 本文 | 一致？ |
|---|---|---|
| `≥ 100` 完整 REC：边框 + 导航条 + 预览 | `full`，同 | ✅ |
| `80–99` 丢弃预览面板，表单占满；导航条保留 | `no-preview`，同 | ✅ |
| `< 80` 丢弃整屏边框，退化为 P3 式连续流；导航条压成一行页位 | `no-border`（60–79），同 | ✅ |
| 极窄（`< 60`）单栏 + 截断策略，详见 #25 | `single-column`（40–59）+ `minimal`（<40），截断规则见 §5.3 | ✅（本文补完） |

### 5.2 各档的形态规则

**`no-preview`（80–99）**：表单区左边界仍是 `col.content.left = 3`，右边界从 `col.form.right = 63` **扩展到 `W − 3`**。右对齐注释的目标列随之变化。预览面板与 `col.divider` 完全不出现（不画空框）。

**`no-border`（60–79）**：丢弃框线后，正文起始行从 `row.body.top = 4` 上移到 `row.nav.sep + 1 = 3`。导航条仍在 `row.nav = 1`，但**只保留页位与页名**，丢弃已答摘要。

**`single-column`（40–59）**：每个「标签 + 取值」对变成两行：

```
工程名称
  my-plugin
```

标签行用 `dim` 语义（无色档靠 R5 的列位），取值行缩进 `indent.unit = 2`。只读推导值（如 JDK）在取值行尾追加 `·` + 说明，允许换行。

### 5.3 极窄档（< 60）的截断 / 换行 / 切单栏规则

**三类内容，三种处置**（这是「截断策略」的完整定义）：

| 类别 | 例子 | 处置 |
|---|---|---|
| **hard：标识性内容（不可截断）** | 路径、Maven 坐标、包名、版本号、命令 | **换行**（续行缩进 +1 = `indent.unit`），**绝不截断** |
| **soft：说明性内容（可截断）** | 说明行、`when:` 表达式、注释、警告文案 | **截断**到 `W − 起始列 − 1`，末字符替换为 `…`（U+2026；ASCII 档为 `...`） |
| **omit：重复性内容（可省略）** | 进度条字形序列、分隔线 | 整条省略（进度条 → 只留 `n/N`） |

**硬规则**：

1. **不横向滚动**：任何行的显示宽度（CJK 按 2 计）≤ `W`。
2. **不折到 0 列**：续行必须缩进 ≥ `indent.unit`。
3. **截断必留标记**：soft 截断后必须以 `…` 结尾（用户必须能看出被截断了）。
4. **hard 换行的续行不重复标签**，只重复缩进。
5. **极窄档不改变语义标记**：`✗` / `✓` / `!` / `❯` 与 `n/N` 在任何宽度档下都存在（它们是降级契约的锚点）。

### 5.4 宽度从哪来

```rust
fn detect_width(env: &Env) -> u16 {
    if let Ok((cols, _)) = crossterm::terminal::size() { if cols > 0 { return cols } }
    if let Some(n) = env.get("COLUMNS").and_then(|s| s.parse::<u16>().ok()) { if n > 0 { return n } }
    100   // screen.w.full
}
```

- 第 1 优先：`crossterm::terminal::size()`（`crossterm-0.29.0/src/terminal.rs:136`）。
- 第 2 优先：`COLUMNS`（已导出时）。这让 `COLUMNS=70 vinoa versions | cat` 成为一条**不依赖 pty** 的可测断言。
- 兜底：`screen.w.full = 100`（`tokens.md` §5.1），即**最完整**形态而不是最保守形态。理由：非交互场景（`--json` / pipe）的消费者是程序，宽度不构成可读性问题，而 `100` 是原型画布宽度、是**唯一经过人眼验收**的版式。

### 断言组 E —— 宽度（10 条）

统一夹具：**版式断言**（E1–E5、E7–E9）走 `ratatui::backend::TestBackend`，直接把 `W` 注入 `UiEnv::from_parts`——比 pty 的 `TIOCSWINSZ` 稳定，且能逐格读列位；**进程级断言**（E6、E10）走 `COLUMNS=<n> vinoa … | cat`。E1–E4 若要用真 pty 复现，用 `ptycap.py --cols <n>`。

- **断言 E1**：`W = 100` 的 `TestBackend` 帧 → `col.divider`(=66) 处有分栏竖线，且预览区标题存在。
- **断言 E2**：`W = 90` 的 `TestBackend` 帧 → 无分栏竖线、无预览标题；`row.nav`(=1) 有导航条；框线存在。
- **断言 E3**：`W = 70` 的 `TestBackend` 帧 → 无任何 `box.*` 字形（`╭╮╰╯┌┐└┘─`）；导航条仍是 1 行。
- **断言 E4**：`W = 50` 的 `TestBackend` 帧 → 标签与取值分行（不存在「标签在 col 3 且取值在 col 16」的同行）。
- **断言 E5**：`W = 30` 的 `TestBackend` 帧 → 无 `█` / `░`；进度行含 `n/N` 数字；导航条只含 `n/4`（无页名）。
- **断言 E6**：`COLUMNS=`（空）或 `COLUMNS=abc` 且无 pty → 按 `W = 100` 渲染（不 panic）。
- **断言 E7**：对 `W ∈ {30, 40, 50, 60, 70, 80, 99, 100, 200}` 的每一档 → 每行显示宽度 ≤ `W`。
- **断言 E8**：`W = 50` → soft 类被截断的行以 `…` 结尾；hard 类（含 `/` 的路径、含 `:` 的坐标）**不出现** `…` 且发生换行。
- **断言 E9**：`W ∈ {50, 70, 100}` 三档 → `✗`/`✓`/`!`/`❯` 的计数逐项相等（宽度只改版式，不改语义）。
- **断言 E10**：`COLUMNS=100` 与 `COLUMNS=200` → 输出**逐字节相同**（`≥ 100` 归并到同一档）。

---

## 6. 硬契约（不得违反）

### 6.1 `--json`：stdout 恰好一个 JSON 文档

spec §11.5 冻结。**任何降级档、任何宽度档、任何 `--color` 取值下都成立。**

- stdout = 数据通道：恰好一个 JSON 文档，零 ANSI，零人类文本。
- stderr = 人类通道：所有 `report::info` / `plan_human` / `error` 的人类行。
- 现有实现（`src/report/mod.rs:41-52`）用 `JSON_EMITTED` 保证「只有第一份文档走 stdout」，本文不改这个机制，只把它写进断言。

### 6.2 CI（非 TTY）不得出现 ANSI 转义

- 覆盖**所有**子命令与**所有**路径，含 `--help`（clap/anstream 已经做到，实测 `vinoa init --help | cat` 的 `\x1b` 计数为 0）。
- **包括透传内容**：`--verify` 会把 Gradle 日志尾部写到 stdout（spec §11.4）。日志文件里的字节可能含 ANSI（若某次 Gradle 未被 `--console=plain` 约束，或被用户自定义脚本污染）。因此非 TTY 档下，**所有透传字节必须过 ANSI 消毒**（剥离 `\x1b` 序列，或替换为可见转义文本）。

### 6.3 `cargo clippy --all-targets -- -D warnings` 通过

CI 已在 `.github/workflows/acceptance.yml` 的 `unit` job 里跑（`ubuntu-latest` + `windows-latest`）。本契约新增的代码必须维持 exit 0。**本机基线已实测：exit 0。**

特别注意（§1.6）：`crossterm::ansi_support` 只在 Windows 下存在，引用必须包 `#[cfg(windows)]`，否则非 Windows 编译失败 ⇒ clippy 红。

### 6.4 Windows 与 Linux 行为都成立

同一组断言必须在 `ubuntu-latest` 与 `windows-latest` 上都通过。仅 Windows 的条目（A16）用 `#[cfg(windows)]` 标注，在 Linux 上**跳过而不是失败**；A14 与 L2 是**非** Windows 条目，在 Windows 上跳过。

### 6.5 `--help` 的着色不由 vinoa 控制（诚实边界）

`src/cli.rs:369-377`：`DisplayHelp` 分支直接 `err.print()`，**在 vinoa 自己的 `UiEnv::detect()` 之前**。`--help` 的颜色由 `clap_builder → anstream → anstyle-query` 决定（#20 §1.4）。因此：

- **保证**：`--help` 在 `NO_COLOR`（非空）/ `TERM=dumb` / 非 TTY 下零 ANSI —— 这些 `anstream` 已实现，实测为 0。
- **不保证**：`--color=never` 对 `--help` 生效。若 #28 要求完全接管，需自定义 `help_template` 渲染或显式用 `anstream::AutoStream` 包装输出——**那是 #28 的决定，本文不替它做**。
- **一致性**：本文 §1.8 的序 4–6 照抄 `anstream` 的判定顺序，因此环境变量层面 `--help` 与正文行为一致；唯一可能分叉的是 `--color` 与 `VINOA_COLOR`。

### 断言组 F —— 硬契约（13 条）

- **断言 F1**：`vinoa versions --json | cat` → stdout 恰好 1 行、`json.loads` 成功、`\x1b` 计数 = 0。
- **断言 F2**：`vinoa init my-plugin -m 1.21.11 --platform paper --json | cat` → 同 F1。
- **断言 F3**：`vinoa schema --json | cat` → 同 F1。
- **断言 F4**：`vinoa versions --platform nope --json | cat` → stdout 恰好 1 行 JSON，`ok == false`，`exit_code == 64`，`\x1b` 计数 = 0。
- **断言 F5**：`vinoa versions --json 2>/dev/null | wc -l` → `1`（人类文本确实走 stderr）。
- **断言 F6**：对 `{versions, versions --json, schema, schema --json, init --help, versions --help, schema --help, init -m 1.21.11 --platform nope}` 每一条执行 `… | cat` → `\x1b` 计数 = 0。
- **断言 F7**：`cargo clippy --all-targets -- -D warnings` → exit 0。
- **断言 F8**：`cargo test --locked --all-targets` 在 `.github/workflows/acceptance.yml` 的 `unit` job 的 `ubuntu-latest` 与 `windows-latest` 两个矩阵项上 → 上述断言集**都**通过。
- **断言 F9**：`vinoa init --help | cat` 与 `NO_COLOR=1 vinoa init --help`（pty）→ `\x1b` 计数 = 0（§6.5 的保证部分）。
- **断言 F10**：`versions --platform nope` 在 `W ∈ {30, 100}` × `--color ∈ {never, always}` → exit code 恒为 64（降级不改变退出码）。
- **断言 F11**：`vinoa versions --json 2>/dev/null | python3 -c "import json,sys; json.load(sys.stdin)"` → exit 0（stdout 是**完整**文档，未与 stderr 交错）。
- **断言 F12**：构造一个含 `\x1b[31m` 的伪造 Gradle 日志文件，让 `--verify` 读取它 → 非 TTY 下 stdout 的 `\x1b` 计数 = 0（透传消毒生效）。
- **断言 F13**：`--json` 的 `verify.tail[]` → 每一项都不含 `\u001b`（JSON 里的 ESC）。

---

## 7. 断言的机械翻译索引

### 7.1 测试夹具约定

| 夹具 | 用途 | 位置 |
|---|---|---|
| `assert_cmd::Command::cargo_bin("vinoa")` | 非 TTY 断言（A3/A13/F 组） | `tests/degradation_pipe.rs` |
| `scripts/termcap/ptycap.py` | 真 TTY 断言（A/B/C 组、D3/D4；**Windows 无 pty，该文件整体 `#[cfg(unix)]`，F8 的 Windows 侧只跑 pipe 与 TestBackend 断言**） | `tests/degradation_pty.rs`（`#[cfg(unix)]`） |
| `ratatui::backend::TestBackend` | 逐格断言（R1–R8、B8–B10、E1–E5） | `src/ui/render/tests.rs`（单元） |
| `UiEnv::from_parts(..)` | 探测纯函数断言（B4、E6、§2.1/§3.1/§5.4） | `src/ui/env/tests.rs`（单元） |
| CI 矩阵 | Windows 专有（A16） | `acceptance.yml` |

**Windows 上的覆盖缺口（明写，不掩盖）**：Windows 没有 `pty`，因此 A/B/C 组与 D3/D4 的**真终端**断言在 `windows-latest` 上无法执行。F8 在 Windows 侧的实际含义是：**pipe 断言（A3/A4/A13、E6/E10、F 组）与 `TestBackend` 断言（B8/B9/E1–E5、E7–E9）必须通过**，TTY 断言在该平台跳过。这条缺口由 A16 与 §6.4 的能力层断言部分补偿。

**前置实现要求**（否则断言无法机械翻译）：渲染函数必须接受 `Frame`/`Buffer` 而不直接写 stdout（ADR-0001 Consequences 已列为要求）；`UiEnv` 必须可注入（§1.9）。

### 7.2 SGR 解析器（断言共用的判定工具）

```python
SGR = re.compile(r"\x1b\[([0-9;]*)m")

# 16 色前景 30–37 / 背景 40–47 / 亮前景 90–97 / 亮背景 100–107 / 默认前景 39 / 默认背景 49
COLOR_CODES = {*range(30, 38), *range(40, 48), *range(90, 98), *range(100, 108), 39, 49}

def is_color_param(params: str) -> bool:
    """`1;96` 这类复合参数只要含一个颜色码就算颜色（fail closed）。"""
    for part in params.split(";"):
        if not part.isdigit():
            return True                 # 畸形参数按「含颜色」处理：宁可误报，不可漏报
        code = int(part)
        if code in COLOR_CODES or code in (38, 48):
            return True                 # 38/48 是扩展色引导码（38;5;n / 38;2;r;g;b）
    return False

def sgr_set(raw: bytes) -> list[str]:
    return [m.group(1) or "0" for m in SGR.finditer(raw.decode("utf-8", "replace"))]

def color_sgr(raw: bytes) -> list[str]:
    return [p for p in sgr_set(raw) if is_color_param(p)]

def has_esc(raw: bytes) -> bool:
    return b"\x1b" in raw
```

- 「颜色 SGR 计数 = 0」用 `color_sgr()`；「零 ANSI」用 `has_esc()`。
- `bold`(`1`)、`reset`(`0`)、`22`（normal intensity）**不计入**颜色 SGR —— 这正是 §2.3 无色/零 ANSI 两档区分的实现形式。
- **复合参数必须支持**：`tokens.md` §3.1 规定 SGR 片段按 `bg → bold → fg` 拼接，因此 `\x1b[1;96m`（bold + 亮青）是**预期形态**；朴素的前缀正则会把 `1;96` 判成非颜色而漏掉整条断言。上面这版逐段扫描，无此漏洞。

### 7.3 行尾重置：原型渲染器 vs vinoa 实际输出路径（精确措辞）

`tokens.md` §3.5 已记录：`docs/wayfinder/prototypes/render.py` 的 `Canvas.to_ansi()` 给**每行结尾**追加 `\x1b[0m`，9 张既有原型全部依赖它，**不可移除**。因此：

| 对象 | 措辞 | 断言形式 |
|---|---|---|
| **vinoa 实际输出路径**（`--json` / 非 TTY / `NO_COLOR` / `--color=never`） | **真正零 `\x1b`** | `assert not has_esc(stdout)`；`assert not has_esc(stderr)` |
| **原型渲染器 `render.py`** | 允许**每行行尾恰好一个** `\x1b[0m` | `断言：对 .ans 的指定区域，除行尾那一个重置外不含任何 SGR`（`tokens.md` §7 的可执行版本） |

**两者不得混用**：`render.py` 的 `Canvas` 是栅格化工具（`scripts/termcap/ansishot.py`）的输入，**不是** vinoa 的输出路径。任何把 `render.py` 的输出当作 vinoa 输出的断言都是错的；反之，任何要求 vinoa 输出「行尾保留一个重置」的断言也是错的（vinoa 的无色档一个都不发）。

若未来为降级形态新增原型图（例如 `degradation.png`），它沿用原型渲染器的例外，并在该原型的 `.ans` 断言里写明「除行尾重置外」。

### 7.4 断言总表

| ID | 命令 / 输入 | 期望 | 落点 |
|---|---|---|---|
| A1 | `--color=never` + pty | 零 `\x1b` | pty |
| A2 | `--color=always NO_COLOR=1` + pty | 颜色 SGR > 0 | pty |
| A3 | `--color=always versions \| cat` | 颜色 SGR > 0 | pipe |
| A4 | `--color=always versions --json \| cat` | 零 `\x1b` + 一个 JSON | pipe |
| A5 | `NO_COLOR=1` + pty | 颜色 SGR = 0，bold 允许 | pty |
| A6 | `NO_COLOR=` + pty | 颜色 SGR > 0 | pty |
| A7 | `NO_COLOR=0` + pty | 颜色 SGR = 0 | pty |
| A8 | `CLICOLOR_FORCE=1 TERM=dumb` + pty | 颜色 SGR > 0 | pty |
| A9 | `CLICOLOR=0` + pty | 颜色 SGR = 0 | pty |
| A10 | `CLICOLOR=0 --color=always` + pty | 颜色 SGR > 0 | pty |
| A11 | `NO_COLOR=1 CLICOLOR_FORCE=1` + pty | 颜色 SGR = 0 | pty |
| A12 | `TERM=dumb` + pty | 零 `\x1b` | pty |
| A13 | `versions \| cat` | 零 `\x1b` | pipe |
| A14 | `env -u TERM` + pty | 零 `\x1b` | pty |
| A15 | `--color=bogus versions` | exit 64 + 三取值提示 | pipe |
| A16 | Windows `!supports_ansi` + `--color=always` | 零 `\x1b` + ASCII 字形 | CI(win) |
| A17 | 全条件 + `--color=always` + 非 TTY | 颜色 SGR > 0 | pipe |
| B1 | `COLORTERM=truecolor` + pty | 含 `38;2;` | pty |
| B2 | `TERM=xterm-256color` + pty | 含 `38;5;`，无 `38;2;` | pty |
| B3 | `TERM=xterm-16color` + pty | 裸 16 色码，无 `38;5;` | pty |
| B4 | `TERM=xterm` + pty | 同 B3（兜底 16） | pty / unit |
| B5 | `grep available_color_count src/` | 0 命中 | static |
| B6 | `TERM=xterm-16color` + pty | err 行含 `✗` | pty |
| B7 | `NO_COLOR=1` + pty | 无 `38;5;252`、无 `39` | pty |
| B8 | `NO_COLOR=1` + pty，R1–R6 | 0 violations | unit |
| B9 | 真彩 vs 无色同帧 | 行数/标记计数相等，字符数比 ≥ 0.9 | unit |
| B10 | `NO_COLOR=1` + pty | 至少 1 行满足 R5、1 行满足 R6 | unit |
| C1 | `TERM=dumb` + pty | 无制图/块元素字符 | pty |
| C2 | `LC_ALL=C TERM=xterm-256color` + pty | 无制图字符，有 `+ \| -` | pty |
| C3 | `LC_ALL=C.UTF-8` + pty | 含 `╭` `╮` | pty |
| C4 | `TERM=linux LC_ALL=C.UTF-8` + pty | 含 `┌`，无 `╭` | pty |
| C5 | `NO_COLOR=1 LC_ALL=C.UTF-8` + pty | 仍含 `╭` | pty |
| C6 | `LC_ALL=C` + pty | 颜色 SGR > 0 | pty |
| C7 | `LC_ALL=C` 预览树 | `\|-` / `` `- `` / `\|` | unit |
| C8 | `LC_ALL=C` 长任务 | 无 `U+2800–U+28FF` | pty |
| C9 | `LC_ALL=C` vs `C.UTF-8` | `[x]`/`[ ]`/`!` 逐字节相同 | pty |
| D1 | `--verify \| cat` | 零 `\x1b`、零 `\r` | pipe |
| D2 | `versions --refresh \| cat` | stderr 行数 ≤ 4 | pipe |
| D3 | `NO_COLOR=1` + pty 长任务 | 颜色 SGR = 0 且 spinner 仍在转 | pty |
| D4 | `TERM=dumb` + pty 长任务 | 行差异只在秒数 | pty |
| D5 | `TERM=dumb` + pty 长任务 | 无进度条序列，含 `n/N` | pty |
| E1 | `W=100` TestBackend | col 66 有分栏线 | unit |
| E2 | `W=90` TestBackend | 无分栏线/预览，有框 | unit |
| E3 | `W=70` TestBackend | 无 `box.*` 字形 | unit |
| E4 | `W=50` TestBackend | 标签与取值分行 | unit |
| E5 | `W=30` TestBackend | 无 `█`/`░`，含 `n/N` 与 `n/4` | unit |
| E6 | `COLUMNS=abc` 无 pty | 按 100 渲染，不 panic | pipe |
| E7 | `W ∈ {30,40,50,60,70,80,99,100,200}` | 每行宽度 ≤ `W` | unit |
| E8 | `W = 50` | soft 以 `…` 结尾；hard 换行不截断 | unit |
| E9 | `W ∈ {50,70,100}` | 标记计数逐项相等 | unit |
| E10 | `COLUMNS=100` vs `200` | 逐字节相同 | pipe |
| F1 | `versions --json \| cat` | 1 行 JSON，零 `\x1b` | pipe |
| F2 | `init … --json \| cat` | 1 行 JSON，零 `\x1b` | pipe |
| F3 | `schema --json \| cat` | 1 行 JSON，零 `\x1b` | pipe |
| F4 | `versions --platform nope --json \| cat` | 1 行 JSON，`ok:false`，exit 64 | pipe |
| F5 | `versions --json 2>/dev/null \| wc -l` | `1` | pipe |
| F6 | 8 条命令逐一 `\| cat` | 零 `\x1b` | pipe |
| F7 | `cargo clippy --all-targets -- -D warnings` | exit 0 | CI |
| F8 | 断言集 | ubuntu + windows 都通过 | CI |
| F9 | `init --help \| cat` 与 `NO_COLOR=1 … --help` | 零 `\x1b` | pipe/pty |
| F10 | `versions --platform nope` 跨宽度/颜色档 | exit 恒 64 | pipe |
| F11 | `versions --json 2>/dev/null \| json.load` | exit 0 | pipe |
| F12 | 含 `\x1b[31m` 的伪造 Gradle 日志 | 非 TTY stdout 零 `\x1b` | pipe |
| F13 | `--json` 的 `verify.tail[]` | 无 `\u001b` | pipe |

**断言总数：A 17 + B 10 + C 9 + D 5 + E 10 + F 13 = 64 条。**

---

## 8. 与下游票的接口

| 下游 | 需要引用本文的什么 |
|---|---|
| **#24 共享外壳** | §2.4 的 R1–R8（错误/成功/警告行的无色形态）、§3.2 的 ASCII 字形映射、§5.1 的宽度档位表、§6.2 的透传消毒（`--verify` 日志尾部是 #24 的「长任务进度行」的邻居）。**行版式归 #24，触发条件归本文。** |
| **#26 / #27 向导逐页** | §2.4 R4（`col.cursor` 独占标记列在无色档仍然成立）、§3.3（窄终端不触发 ASCII 回退）、§5.2/§5.3 的单栏与截断规则。 |
| **#28 help / versions / schema** | §6.5（`--help` 的着色不由 vinoa 控制，`--color` 对 `--help` 不生效）、§1.7（`--color` 需登记进 §3.1 的 flag 表）、§6.1（`--json` 形状不变）、E5/E7（窄终端下 versions 的坐标换行规则）。 |
| **#23 动效** | 本文 §4.1 采纳其 H3，并补三条判定规则（零 `\r` / 离散行数上限 / 进度条整条省略）。 |
| **#21 渲染选型** | 本文落实 ADR-0001 Decision 第 2 条（降级由 vinoa 自己保证）与「未决/交接」的第 1 条（16 色档来源）。 |

**下游必须遵守的两条元规则**：

1. **不得重复定义触发条件**。任何下游文档若需要「什么时候降级」，引用本文的断言编号，不重写条件表。
2. **不得新增色位/档位**。新增即意味回到 #19（方向）与 #22（token）重新裁定——`tokens.md` §1 的完备性论证会因此失效。

---

## 9. 未决 / 盲区

1. **旧 conhost 的真机观感未验证**（#20 §7 已列为盲区）。A16 可以在 CI 的 `windows-latest` 上跑，但该 runner 用的是 **Windows Terminal / 现代 conhost**，不一定能复现「旧 conhost 无 VT」。**A16 的 `!supports_ansi()` 分支可能在 CI 上永远不触发**——这是已知的覆盖空洞，需要真机或强制 mock（把 `supports_ansi()` 包一层可注入的 trait）。**同一条缺口也适用于 Windows 侧的全部 TTY 断言**（Windows 无 pty，见 §7.1）。
2. **`TERM_SQUARE_BOX` 白名单是启发式的**。它列的是「ANSI 色可用、Unicode 制图不可用」的历史终端；这个集合的边界没有权威来源，只能靠实测。它是**单一常量表**，调整成本低。
3. **`COLUMNS` 在真实 shell 里通常不导出**。E6/E10 依赖它，因此这两条在测试里必须显式设 `COLUMNS`，不能依赖继承；版式断言（E1–E5、E7–E9）走 `TestBackend` 注入 `W`，不受此影响。
4. **`--verify` 的透传消毒**（§6.2 / F12）目前**没有实现**。它是本契约新增的要求，属于实现票的工作量。
5. **`--color` 是新增 flag**，需要同步 `docs/spec/vinoa-cli.md` §3.1 的 flag 表与 `--help` 输出。这两处不在本文的写作范围内，已交接给 #28 与 lead。
6. **`UiEnv` 的单点解析要求**会与现有 `src/wizard/mod.rs:20-23` 的 `should_run`、`src/init.rs:346` 的 `is_tty` 产生两处重复的 TTY 判定。实现票应把它们合并到 `UiEnv`——**这是本文对实现的一处直接要求**，不是建议。
7. **F1/F3/F5/F6/F9/F11/E6/E10 在本文写作时已实测通过**（`target/debug/vinoa`，本机 `TERM=dumb NO_COLOR=1`、stdout 非 TTY；`clippy` 基线 exit 0）。其余断言依赖尚未存在的 `UiEnv` / `--color` / 透传消毒，属**待实现后验证**。已通过的部分不构成「无需回归」——它们正是实现期最容易被破坏的既有契约。

## 10. 本机已实测的基线（写作时快照）

| 断言 | 结果 |
|---|---|
| F1 / F3 / F6（`versions --json`、`schema --json`、8 条命令 `\| cat`） | `ESC_COUNT=0` ✅ |
| F4（`versions --platform nope --json`） | 1 行 JSON、`ok:false`、exit 64、零 `\x1b` ✅ |
| F5（`versions --json 2>/dev/null \| wc -l`） | `1` ✅ |
| F9（`init --help \| cat`） | `ESC_COUNT=0` ✅ |
| F11（`versions --json \| json.load`） | exit 0 ✅ |
| E6（`COLUMNS=abc vinoa versions`） | exit 0、零 `\x1b` ✅ |
| E10（`COLUMNS=100` vs `200`） | md5 相同 ✅ |
| F7（`cargo clippy --all-targets -- -D warnings`） | exit 0 ✅ |
| A15（`--color=bogus`） | 当前 exit 64（flag 尚不存在，clap 报未知参数）——**实现 `--color` 后须复验为「exit 64 且提示三取值」** |
| A1–A14、A16–A17、B1–B10、C1–C9、D1–D5、E1–E5、E7–E9、F2、F8、F10、F12、F13 | 未实测（依赖 `UiEnv` / `--color` / 透传消毒 / 真 pty 夹具） |
