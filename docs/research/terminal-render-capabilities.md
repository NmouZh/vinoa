# 终端色彩与渲染能力事实（inquire / ratatui / Windows / 无色降级）

**研究日期：** 2026-09-26
**范围：** vinoa 在 Linux / macOS / Windows 原生终端上呈现色彩与动效所需的**事实**依据，供「渲染方案选型」与「界面降级契约」消费。
**方法与取证规则：** 每条结论都追溯到**主源**——本机解包的 crate 源码（`~/.cargo/registry/src/`）、crates.io sparse index 的 manifest、Microsoft Learn 的 Windows Console 官方文档、以及 `no-color.org` 官方规范。crate 源码的引用给出**文件:行号**，可复核。不使用博客或二手总结。

> **本次增补（第二轮复核）。** 初版结论已逐条用**一手证据重跑**，不照抄。新增的证据类型有四类，正文中标为「实测」：
> 1. **最小 crate 探针**：临时 `cargo` 工程分别链接 `crossterm 0.29.0` + `inquire 0.9.4`，直接调用 `available_color_count()` / `Colored::ansi_color_disabled()` / `RenderConfig::default()`，在各种 `NO_COLOR`/`TERM`/`COLORTERM` 组合下打印真实返回值。
> 2. **`Cargo.lock` 解析实验**：把 `ratatui` 加进 **vinoa 自己的 manifest 副本**，跑 `cargo generate-lockfile`，看真实解析结果（而不是只读 crate 的 `Cargo.toml` 声明）。
> 3. **真 pty 捕获 + 自写 VT 模拟器**：用 `scripts/termcap/ptycap.py` 与 marker 驱动的 `pty.fork()` 抓原始字节，再用逐格 VT 模拟器统计**每个屏幕单元被写了多少次**——比 `grep -c` 可靠，因为字节计数随交互步数变化。
> 4. **`ratatui` 的 `TestBackend` 实跑**：编译运行最小 ratatui 程序，实测可断言粒度与 CJK 陷阱。

**本机工具链：** `rustc 1.98.1 (48a229cea 2026-09-01)`、`cargo 1.98.1 (797e8a9bc 2026-08-05)`；vinoa 声明 `rust-version = "1.85"`、`edition = "2024"`。

> **环境说明。** 本工作区的 `TERM=dumb` 且 `NO_COLOR=1`，`tput colors` 返回 `-1`（实测）。因此**本机无法直接肉眼验证颜色**；凡涉及「终端实际观感」的结论，均改用源码行为 + pty 抓取的原始字节作为证据，并明确标注哪些属未验证。

---

## 0. 决策相关结论（TL;DR）

| # | 事实 | 对选型的影响 | 证据等级 |
|---|---|---|---|
| 1 | `ratatui 0.29` 依赖 `crossterm ^0.28.1`，而 vinoa 现有的 `inquire 0.9.4` 已带来 `crossterm 0.29.0` | 上 ratatui 0.29 会**同时编译两个 crossterm**（实测：把 ratatui 加进 vinoa manifest 副本后 lock 解析出 `0.28.1` 与 `0.29.0`） | 实测 |
| 2 | `ratatui 0.30.2` 依赖 `crossterm ^0.29`（单一版本），但 **MSRV 1.88** | 想避免重复必须抬 MSRV 到 1.88，或接受双 crossterm | sparse index + 实测 |
| 3 | **`ratatui` 的 crossterm 后端完全不检查 `NO_COLOR`**（grep 零命中） | 上 ratatui 则无色降级**必须自己实现**，无自动路径 | 实测 grep |
| 4 | `inquire` 的 `RenderConfig::default()` 用 `env::var("NO_COLOR")` 判断，**空字符串也判定为「禁用颜色」** | 与 NO_COLOR 规范冲突（规范要求空串**不**禁用）；`crossterm` 是对的，`inquire` 不对 | 实测（probe 打印分支） |
| 5 | 一旦 vinoa 显式设置 `RenderConfig`，`inquire` **不再自动处理 `NO_COLOR`**（源码注释明示） | 自定义样式 = 自己接管无色降级 | 源码 |
| 6 | `crossterm` 的 `available_color_count()` **默认返回 8**，不是 16 | 真彩/256 探测失败时退到 8 色，需注意 | 实测 |
| 7 | `crossterm` 有 `force_color_output(bool)`，可**覆盖 `NO_COLOR`** | 提供了「配置文件/命令行覆盖环境变量」的正规出口 | 源码 |
| 8 | `ratatui` 自带 `TestBackend`，`Terminal::draw` 走 buffer diff | 可测试性与「消除重绘抖动」是 ratatui 的真实收益 | 实测（含 diff=0 幂等） |
| 9 | `ratatui 0.29` 声明 MSRV 1.74，但依赖 `instability ^0.3.1`，其 **0.3.11+ 要求 1.88**；在 vinoa 配置下 cargo 的 **MSRV-aware 解析**会锁到 `instability 0.3.10`（1.64），从而保住 1.85 | 「ratatui 0.29 不抬 MSRV」**结论成立，但理由不是 ratatui 的声明**——这层边界依赖调用方 manifest 与解析环境，是脆弱点 | 实测（三组对照，见 §6.2） |
| 10 | `ratatui 0.29` 还引入**第二个 `unicode-width`**（pin `=0.2.0` vs vinoa 现有 `0.2.2`） | 双版本不止 crossterm 一处；宽字符宽度计算可能出现两套口径 | 实测 lock |
| 11 | `inquire` 的 prompt 渲染走 **stderr**，但 vinoa 的 `should_run` 要求 **stdin 且 stdout** 都是 TTY | 存在一个可复现的「TTY 判定与渲染流不一致」边界（见 §4.4） | 实测（pty 重定向实验） |

---

## 1. `NO_COLOR` / `CLICOLOR_FORCE` / `TERM=dumb` 的精确语义

### 1.1 规范原文

`no-color.org`（**Last updated: 2026-09-23**，本次已抓取，HTTP 200）的定义：

> Command-line software which adds ANSI color to its output by default should check for a `NO_COLOR` environment variable that, when present and **not an empty string** (regardless of its value), prevents the addition of ANSI color.

三条关键细则（同页 FAQ）：

1. `NO_COLOR` 是给**软件**的提示，不是给终端「禁止显示颜色」的开关。
2. **用户级配置文件与命令行参数应当覆盖 `NO_COLOR`**——用户可以全局导出，再对单个程序开启颜色。
3. **`NO_COLOR` 只关颜色，不关 bold / underline / italic。**

### 1.2 `crossterm 0.29.0` 的实现——**符合规范**

`src/style/types/colored.rs:75-79`：

```rust
pub fn ansi_color_disabled() -> bool {
    !std::env::var("NO_COLOR")
        .unwrap_or("".to_string())
        .is_empty()
}
```

即：**已设置且非空**才算禁用，与规范一致。该 crate 自带的测试（`colored.rs:310-319`）明确断言了四种情形：

| `NO_COLOR` | `ansi_color_disabled()` |
|---|---|
| `"1"` | `true` |
| `"XXX"` | `true` |
| `""`（空串） | **`false`** |
| 未设置 | `false` |

**实测复核（最小 crate 探针，直接调用该函数）：**

| `NO_COLOR` | `ansi_color_disabled()` | `available_color_count()` |
|---|---|---|
| unset | `false` | 8 |
| `"1"` | `true` | 8 |
| `"0"` | **`true`** | 8 |
| `""`（空串） | **`false`** | 8 |
| `" "`（单个空格） | `true` | 8 |

注意 `NO_COLOR=0` **仍然禁用**——规范要求「regardless of its value」，`crossterm` 实现正确。`available_color_count()` 与 `NO_COLOR` 无关（它是能力探测，不是开关），这也被实测确认。

另有两点实现细节：

- **记忆化**：`ansi_color_disabled_memoized()`（`colored.rs:81-87`）用 `Once` 缓存首次结果。运行中改环境变量不会生效，除非显式调用 `set_ansi_color_disabled()`。
- **可覆盖**：`force_color_output(enabled: bool)`（`src/style.rs:191-193`）→ `set_ansi_color_disabled(!enabled)`，文档明确写「overriding NO_COLOR」。这正是规范 FAQ 第 2 条要求的出口。

### 1.3 `inquire 0.9.4` 的实现——**不符合规范**

`src/ui/api/render_config.rs:350-357`：

```rust
impl<'a> Default for RenderConfig<'a> {
    fn default() -> Self {
        match env::var("NO_COLOR") {
            Ok(_) => Self::empty(),
            Err(_) => Self::default_colored(),
        }
    }
}
```

`env::var` 对 `NO_COLOR=""` 返回 `Ok("")`，因此**空串也会让 inquire 退到无色配置**。规范要求空串**不**禁用。这是 `inquire` 与 `crossterm` 之间一处**可验证的行为不一致**（`crossterm` 对，`inquire` 错）。

**实测复核**（探针调用 `RenderConfig::default()`，检查 `prompt_prefix.style.fg`）：

| `NO_COLOR` | `prompt_prefix.style.fg` | 分支 |
|---|---|---|
| unset | `Some(LightGreen)` | `default_colored()` |
| `""`（空串） | **`None`** | **`empty()`** ← 与规范冲突 |
| `"1"` | `None` | `empty()` |

实际影响很小（极少有人导出空的 `NO_COLOR`），但记录在此，因为它会在「界面降级契约」里变成一条要写进测试的边界条件。

> **与 vinoa 现状的组合后果**：`vinoa init --help` 走的是 `clap`/`anstream`（§1.4），**不受** inquire 这个 bug 影响；但向导页走 inquire，**受影响**。两者对同一个空串 `NO_COLOR` 会给出不同答案——这正是降级契约必须显式钉死优先级的原因。

### 1.4 `anstyle-query 1.1.5`——clap 的 `--help` 走的就是这条

`anstyle-query` 是 `anstream` 的依赖，而 `anstream` 是 `clap_builder` 的依赖，因此 **vinoa 的 `--help` 着色完全由它决定**（`cargo tree -i anstyle-query` 实测链路：`anstyle-query 1.1.5 → anstream 1.0.0 → clap_builder 4.6.7 → clap 4.6.7 → vinoa`）。

`src/lib.rs`：

| 函数 | 语义 | 行 |
|---|---|---|
| `clicolor()` | `Some(value != "0")`——**`CLICOLOR=0` 表示禁用** | :23-26 |
| `clicolor_force()` | `non_empty(CLICOLOR_FORCE)` | :34-36 |
| `no_color()` | `non_empty(NO_COLOR)`——与规范一致 | :49-51 |
| `term_supports_color()` | 非 Windows 下：`TERM` **未设置 → false**；`TERM=dumb` → false；否则 true | :55-71 |
| `is_ci()` | `CI` 环境变量**存在即为真**（不看取值） | :128-135 |

`anstream` 的判定顺序（`src/auto.rs:198-217`）：`no_color()` → `Never`；否则 `clicolor_force()` → `Always`；否则 `clicolor_disabled`（即 `CLICOLOR=0`）→ `Never`；否则 `raw.is_terminal() && (term_supports_color() || clicolor_enabled || is_ci())` → `Always`；否则 `Never`。

**优先级（实测验证）：`NO_COLOR` 高于 `CLICOLOR_FORCE`。**

**实测复核**（`./target/release/vinoa init --help`，stdout 接管道）：

| 环境 | 输出中的 ESC 字节数 |
|---|---|
| `NO_COLOR` unset | 0（非 TTY，`Never`） |
| `NO_COLOR=""` | 0 |
| `NO_COLOR=1` | 0 |
| `NO_COLOR` unset + `CLICOLOR_FORCE=1` | **32** ← 强制生效 |
| `NO_COLOR=1` + `CLICOLOR_FORCE=1` | **0** ← `NO_COLOR` 赢 |
| `NO_COLOR` unset + `CLICOLOR_FORCE=0` | **32** ← `CLICOLOR_FORCE` 只看「非空」 |

**真 pty 下再测**（`pty.fork()`，`TERM` 与 `NO_COLOR` 受控）：

| 环境 | ESC 字节数 |
|---|---|
| `TERM=dumb`（继承），`NO_COLOR` unset | 0 |
| `TERM=xterm-256color` | 77 |
| `TERM=xterm-256color` + `NO_COLOR=1` | 0 |
| `TERM=xterm-256color` + `NO_COLOR=""`（空串） | **77** ← 空串**不**禁用，与规范一致 |
| `TERM=dumb` 显式 | 0 |

结论：**`clap` 这一路完全符合规范，且已正确实现非 TTY 降级**；空串行为也是对的。与 `inquire` 的 §1.3 形成对照。

---

## 2. Windows 退化路径

### 2.1 `crossterm` 走 `winapi` + 显式启用 VT

`crossterm 0.29.0` 在 Windows 上通过 `winapi` 调用控制台 API，`src/ansi_support.rs:1-31` 全文：

```rust
use crossterm_winapi::{ConsoleMode, Handle};
use winapi::um::wincon::ENABLE_VIRTUAL_TERMINAL_PROCESSING;

fn enable_vt_processing() -> std::io::Result<()> {
    let mask = ENABLE_VIRTUAL_TERMINAL_PROCESSING;
    let console_mode = ConsoleMode::from(Handle::current_out_handle()?);
    let old_mode = console_mode.mode()?;
    if old_mode & mask == 0 {
        console_mode.set_mode(old_mode | mask)?;
    }
    Ok(())
}

pub fn supports_ansi() -> bool {
    INITIALIZER.call_once(|| {
        // Some terminals on Windows like GitBash can't use WinAPI calls directly
        // so when we try to enable the ANSI-flag for Windows this won't work.
        // Because of that we should check first if the TERM-variable is set
        // and see if the current terminal is a terminal who does support ANSI.
        let supported = enable_vt_processing().is_ok()
            || std::env::var("TERM").map_or(false, |term| term != "dumb");
        SUPPORTS_ANSI_ESCAPE_CODES.store(supported, Ordering::SeqCst);
    });
    SUPPORTS_ANSI_ESCAPE_CODES.load(Ordering::SeqCst)
}
```

逐条读出的事实：

1. **`crossterm` 不假设旧 `conhost` 默认支持 ANSI**——它主动 `GetConsoleMode` → 置位 `ENABLE_VIRTUAL_TERMINAL_PROCESSING` → `SetConsoleMode`。这与 Microsoft 官方推荐的启用步骤完全一致（见 §2.2）。
2. **失败时有第二条路**：若 WinAPI 调用失败，则退而检查 `TERM` 是否已设置且不是 `dumb`——这是给 GitBash / MSYS2 这类「拿不到真控制台句柄、但本身是 ANSI 终端」的环境留的口子。
3. **结果被 `Once` 记忆化**，进程内只探测一次。
4. **`supports_ansi()` 不只是探测，它还是「要不要走 ANSI 输出」的开关**：`src/command.rs:35-37` 的 `is_ansi_code_supported()` 直接返回它，而 `src/command.rs:118-127` 的 `queue()` 在 Windows 上据此分派：

```rust
#[cfg(windows)]
if !command.is_ansi_code_supported() {
    self.flush()?;
    command.execute_winapi()?;   // ← 改走 WinAPI，不写 ANSI 字节
    return Ok(self);
}
write_command_ansi(self, command)?;
```

即：**旧 Windows 上 `crossterm` 会把「设前景色」翻译成 `SetConsoleTextAttribute`，而不是往流里写 `\x1b[38;5;10m`。** 这是 `crossterm` 真正的 Windows 退化路径——不是「降级成无色」，而是「换成 API 调用」。

`crossterm_winapi 0.9.1` 已在 vinoa 的 `Cargo.lock` 中（`inquire` 的传递依赖）。

**WinAPI 路径的能力上限（关键限制）**：`src/style/sys/windows.rs:129-134` 与 `:161-166` 两处都写着同一句注释：

```rust
/* WinAPI will be used for systems that do not support ANSI, those are windows version less then 10.
   RGB and 255 (AnsiBValue) colors are not supported in that case.*/
Color::Rgb { .. } => 0,
Color::AnsiValue(_val) => 0,
```

即**旧 conhost 上真彩（`Rgb`）与 256 色（`AnsiValue`）都被静默映射成 `0`（黑）**，只有 16 色基本色可用。这直接否定了「在旧 conhost 上用 `Rgb` 做品牌色」的可行性。

`crossterm` 的颜色档位探测（`src/style.rs:163-181`）：

```rust
pub fn available_color_count() -> u16 {
    #[cfg(windows)]
    {
        // Check if we're running in a pseudo TTY, which supports true color.
        // Fall back to env vars otherwise for other terminals on Windows.
        if crate::ansi_support::supports_ansi() {
            return u16::MAX;   // ← 注意：Windows 上只要 VT 可用就直接报真彩
        }
    }

    const DEFAULT: u16 = 8;
    env::var("COLORTERM")
        .or_else(|_| env::var("TERM"))
        .map_or(DEFAULT, |x| match x {
            _ if x.contains("24bit") || x.contains("truecolor") => u16::MAX,
            _ if x.contains("256") => 256,
            _ => DEFAULT,
        })
}
```

三个要点：

- **默认档位是 8 色**（`DEFAULT = 8`），不是 16。函数自带注释「This does not always provide a good result.」实测确认（§1.2 表）。
- **Windows 上只要 `supports_ansi()` 为真就返回 `u16::MAX`（真彩）**，不再看 `COLORTERM`。这是一个**乐观**判断：VT 可用 ≠ 终端真的支持 24-bit（旧 conhost 在启用 VT 后仍会把 `38;2` 就近映射到 16 色表——见 §2.2 的 Microsoft 原文）。
- 该函数**不检查 `NO_COLOR`**——无色是**发射时**由 `ansi_color_disabled()` 把门的，两件事分开。

**实测复核（能力探测，`NO_COLOR` unset，Linux）**：

| 环境 | `available_color_count()` |
|---|---|
| `TERM=dumb` | 8 |
| `TERM=xterm` | 8 |
| `TERM=vt100` | 8 |
| `TERM=xterm-256color` | 256 |
| `TERM=xterm-truecolor` | 65535 |
| `COLORTERM=truecolor TERM=dumb` | 65535 |
| `COLORTERM=24bit TERM=xterm` | 65535 |
| `COLORTERM=truecolor TERM=xterm-256color` | 65535 |

注意 `COLORTERM` 优先于 `TERM`（源码用 `env::var("COLORTERM").or_else(|_| env::var("TERM"))`），且 `COLORTERM=truecolor` **能压过** `TERM=dumb`。这与 clap 那一路（`TERM=dumb` 直接否决）行为不同——同一个终端，`--help` 与向导可能得出不同结论。

### 2.2 Windows 官方文档怎么说（一手规范）

来源：[Console Virtual Terminal Sequences](https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences)、[SetConsoleMode](https://learn.microsoft.com/en-us/windows/console/setconsolemode)（本次均抓取成功，HTTP 200）。

**（a）ANSI 默认不可用，必须显式开启。** Microsoft 原文：

> The following terminal sequences are intercepted by the console host when written into the output stream, **if the ENABLE_VIRTUAL_TERMINAL_PROCESSING flag is set** on the screen buffer handle using the SetConsoleMode function.

并且 `SetConsoleMode` 文档在 screen buffer 模式表里列出：

> **ENABLE_VIRTUAL_TERMINAL_PROCESSING** 0x0004 … **Ensure ENABLE_PROCESSED_OUTPUT is set when using this flag.**

→ **需要哪些启用步骤**：`GetStdHandle(STD_OUTPUT_HANDLE)` → `GetConsoleMode` → `dwMode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING`（并确保 `ENABLE_PROCESSED_OUTPUT`）→ `SetConsoleMode`。这正是 `crossterm` 在做的。

**（b）旧系统会直接失败，官方要求优雅降级。** 同一页：

> Checking whether SetConsoleMode returns `0` and GetLastError returns ERROR_INVALID_PARAMETER is the current mechanism to determine when running on a down-level system. An application receiving ERROR_INVALID_PARAMETER with one of the newer console mode flags in the bit field should **gracefully degrade behavior and try again**.

→ 这解释了 `crossterm` 为什么把 `enable_vt_processing()` 的返回值当布尔用：失败即旧系统。

**（c）真彩在旧 conhost 上被降级为 16 色表里最近的颜色。** 官方「Extended Colors」小节原文：

> Some virtual terminal emulators support a palette of colors greater than the 16 colors provided by the Windows Console. For these extended colors, **the Windows Console will choose the nearest appropriate color from the existing 16 color table for display.**

→ **重要**：这意味着即使在**已启用 VT 的旧 conhost** 上，`\x1b[38;2;r;g;b` 也不会得到真彩，而是被就近吸附到 16 色。而 `crossterm` 的 `available_color_count()` 在 Windows 上会因为 VT 可用就报 `u16::MAX`（§2.1）——**探测结果与真实观感不一致**。

**（d）框线字符有原生支持，且不依赖 Unicode 字形。** 官方「Designate Character Set」小节：

> | ESC ( 0 | Designate Character Set – DEC Line Drawing | Enables DEC Line Drawing Mode |
> Notably, the DEC Line Drawing mode is used for drawing borders in console applications. The following table shows what ASCII character maps to which line drawing character. … 0x6c `l` → `┌`，0x71 `q` → `─`，0x78 `x` → `│`

→ 旧 conhost 有**内建**的 DEC 线绘字符集。但这是**终端侧字形**，与「直接写 UTF-8 的 `┌─┐│` 字节」是两条不同路径。`crossterm`/`inquire` 走的是后者（直接写 UTF-8），因此不享受这条保证。**旧 conhost 的默认代码页（如 437/936）能否正确显示 UTF-8 框线字节，取决于代码页与字体**——本机无法验证（见 §7）。

**（e）`DISABLE_NEWLINE_AUTO_RETURN` 对全屏 TUI 有实际意义。** 官方：

> The typical usage of this flag is intended in conjunction with setting ENABLE_VIRTUAL_TERMINAL_PROCESSING to better emulate a terminal emulator where writing the final character on the screen (in the bottom right corner) without triggering an immediate scroll is the desired behavior.

→ 若最终上 ratatui 全屏接管，这个 flag 是「右下角写字不触发滚动」的关键；`crossterm` 未自动设置它，需调用方处理。

**（f）CJK 宽字符：官方文档无对应承诺。** 我通读了上述两页，**没有**任何关于「双宽字符占两格」的保证；相反，`SetConsoleMode` 文档提到 DBCS 相关的历史包袱：

> Historically, these flags only functioned in DBCS code pages for Chinese, Japanese, and Korean languages. … It is off by default to maintain compatibility with known applications that have historically taken advantage of the console ignoring these flags on non-CJK machines…

→ **结论：CJK 宽字符在旧 conhost 上的列对齐属于未验证项**（见 §7）。注意 `ENABLE_LVB_GRID_WORLDWIDE` 的存在恰恰说明 conhost 的宽字符/属性处理有语言相关分支。

### 2.3 `ratatui` 没有 `NO_COLOR` 检查，也没有 Windows 特判

对 `ratatui-0.29.0/src/` 全量 grep（三种模式，均实测）：

```
grep -rn 'NO_COLOR'  ratatui-0.29.0/src/   → rc=1（零命中）
grep -rn 'no_color'  ratatui-0.29.0/src/   → rc=1（零命中）
grep -rn 'is_terminal\|IsTerminal\|isatty\|atty' ratatui-0.29.0/src/ → rc=1（零命中）
grep -rn 'available_color_count' ratatui-0.29.0/src/ → rc=1（零命中）
```

平台相关 `cfg` 只有一处，且与颜色无关：`src/lib.rs:340` 的 `#[cfg(all(not(windows), feature = "termion"))]`（termion 后端在 Windows 上不编译）。

结论：**`ratatui` 不提供任何自动的无色或非 TTY 降级，也不做 Windows 能力判断。** 它把颜色当作已决定的事实，直接写进 buffer，再由 `CrosstermBackend` 转成 crossterm 命令。Windows 上的 ANSI 启用/退化**完全继承 crossterm 的行为**（§2.1），ratatui 不额外处理。

> 顺带：`ratatui` 的 `CrosstermBackend::size()`（`src/backend/crossterm.rs:253-256`）直接转发 `crossterm::terminal::size()`，没有任何兜底。因此 §4.3 的尺寸问题在 ratatui 上同样存在，且是**硬失败**（`Terminal::new` 直接返回 `Err`）。

### 2.4 Unicode 边框与 CJK 宽字符

- **未验证**：旧 `conhost` 上圆角框线（`╭╮╰╯`）与 CJK 宽字符的实际渲染质量，本机是 WSL，**无法验证**。需真机确认。
- **已确认（本机 Linux pty）**：`inquire` 与 vinoa 自己输出的框线/箭头/复选框字符都是**直接写 UTF-8 字节**，不依赖 DEC 线绘模式。见 §3.5 的原始字节。
- 已知相关的第三方事实：NO_COLOR 官方列表中，`crossterm` 自 **0.27**（2023-08-06）起支持 `NO_COLOR`——与 §1.2 的源码一致；`inquire` 自 **0.0.10**（2021-08-29）起在列。

---

## 3. `inquire 0.9.4` 的可定制边界

### 3.1 `RenderConfig` 能改什么

`src/ui/api/render_config.rs:23-160` 定义 `RenderConfig<'a>`，公开字段（均可用 `with_*` builder 覆盖）。实测该文件共有 **22** 个 `pub fn with_`，其中 **18** 个属于 `RenderConfig`、**4** 个属于 `ErrorMessageRenderConfig`：

| 字段 | 类型 | 默认值（`default_colored()`，:200-231） |
|---|---|---|
| `new_line_prefix` | `Option<Styled<&str>>` | `None` |
| `prompt_prefix` | `Styled<&str>` | `"?"` + **LightGreen**（ANSI 10） |
| `answered_prompt_prefix` | `Styled<&str>` | `">"` + **LightGreen** |
| `prompt` | `StyleSheet` | `empty()`（**无色**） |
| `default_value` | `StyleSheet` | `empty()` |
| `placeholder` | `StyleSheet` | **DarkGrey**（ANSI 8） |
| `help_message` | `StyleSheet` | **LightCyan**（ANSI 14） |
| `text_input` | `StyleSheet` | `empty()` |
| `password_mask` | `char` | `'*'` |
| `answer` | `StyleSheet` | **LightCyan** |
| `answer_from_new_line` | `bool` | `false` |
| `canceled_prompt_indicator` | `Styled<&str>` | `"<canceled>"` + **DarkRed**（ANSI 1） |
| `error_message` | `ErrorMessageRenderConfig` | 见下 |
| `highlighted_option_prefix` | `Styled<&str>` | `">"` + **LightCyan** |
| `unhighlighted_option_prefix` | `Styled<&str>` | `" "` + LightCyan |
| `scroll_up_prefix` / `scroll_down_prefix` | `Styled<&str>` | `"^"` / `"v"`（无色） |
| `selected_checkbox` | `Styled<&str>` | `"[x]"` + **LightGreen** |
| `unselected_checkbox` | `Styled<&str>` | `"[ ]"`（无色） |
| `option_index_prefix` | `IndexPrefix` | `None` |
| `option` | `StyleSheet` | `empty()` |
| `selected_option` | `Option<StyleSheet>` | `Some(LightCyan)`；`None` 时回退到 `option` |
| `calendar` | `calendar::CalendarRenderConfig` | 仅 `date` feature |
| `editor_prompt` | `StyleSheet` | 仅 `editor` feature |

`ErrorMessageRenderConfig`（`:426-433`）独立一层，4 个字段：`prefix`（`"#"` + **LightRed**）、`separator`、`message`（LightRed）、`default_message`（`"Invalid input."`）。

**颜色表达能力（决定能不能上真彩）**：`StyleSheet`（`src/ui/api/style.rs:52-59`）是

```rust
pub struct StyleSheet {
    pub fg: Option<Color>,   // inquire 自己的 Color 枚举
    pub bg: Option<Color>,
    pub att: Attributes,     // bold/italic/underline/… 位标志
}
```

而 `inquire` 的 `Color`（`src/ui/api/color.rs`）文档自称「**Currently a clone of [crossterm::style::Color]**」，且**包含 `Rgb { r, g, b }`（:137）与 `AnsiValue(u8)`（:154）**。

→ **结论：`inquire` 完全支持 24-bit 真彩与 256 色，也能设 bold/italic/underline。** 「inquire 只能 16 色」是错的。

**这解释了实测到的「只有 2–3 种颜色」**：在默认配置下，向导实际用到的只有 `prompt_prefix` 的 LightGreen（`\x1b[38;5;10m`）、`help_message`/`answer` 的 LightCyan（`\x1b[38;5;14m`）、以及错误路径的 LightRed（`\x1b[38;5;9m`），因为 `text_input` / `default_value` / `prompt` / `option` 都是 `StyleSheet::empty()`。**颜色并非不可改，而是从未被配置。**

### 3.2 `RenderConfig` 改不到什么

- **布局**：inquire 按 prompt 逐行渲染，不接管全屏。整屏边框、左右分栏、常驻预览面板**做不了**。
- **重绘模型**：每页由独立 prompt 顺序渲染，页面标题（vinoa 自己的 `eprintln!`）与 prompt 不在同一网格上——这正是 ③ 页叠印的根因，**样式层改不掉**。量化证据见 §3.5。
- **持续动画**：没有 tick 循环，无法做常驻进度区。
- **两个 prompt 之间的共享上下文**：每个 prompt 自己开/关 raw mode、自己管理帧。

### 3.3 关键陷阱：自定义即放弃 `NO_COLOR`

`inquire` 各 prompt 的 `with_render_config` 文档反复写着同一句警告（实测命中多个文件，如 `src/prompts/multiselect/mod.rs:118-123`、`src/prompts/editor/mod.rs:83-88`）：

> Note: The default render config considers if the `NO_COLOR` environment variable is set to decide whether to colorize the output. When overriding the config in a prompt, `NO_COLOR` is no longer considered and your config will be treated as the default one. If you want to have a config and still support `NO_COLOR`, you will have to do this on your end.

全局入口 `src/config.rs:8-20`：

```rust
static GLOBAL_RENDER_CONFIGURATION: LazyLock<Mutex<RenderConfig<'static>>> =
    LazyLock::new(|| Mutex::new(RenderConfig::default()));
pub fn set_global_render_config(config: RenderConfig<'static>) { ... }
```

注意 `LazyLock::new(|| Mutex::new(RenderConfig::default()))`——**全局默认值在首次访问时求值一次**，即 `NO_COLOR` 只在那一刻被读一次。

**推论：vinoa 一旦引入 token 表并 `set_global_render_config(...)`，就必须自己实现 `NO_COLOR` 判定。** 这不是可选项，是设置样式的必然代价。

### 3.4 默认两色 `38;5;10` / `38;5;14` 是否可覆盖——**可以，且这是唯一的正解**

- **可以覆盖**：这两个颜色分别来自 `prompt_prefix`/`answered_prompt_prefix`/`selected_checkbox`（LightGreen）与 `help_message`/`answer`/`selected_option`/`highlighted_option_prefix`（LightCyan），全部是**公开字段 + `with_*` builder**。要换成真彩只需 `StyleSheet::new().with_fg(Color::Rgb{r,g,b})`。
- **编码路径**：`38;5;10` 这个具体写法来自 `crossterm` 的 `Colored` 显示实现（`src/style/types/colored.rs:131-151`），它把 16 色**一律编码成 8-bit 形式**：

  ```rust
  Color::Green  => f.write_str("5;10"),
  Color::Cyan   => f.write_str("5;14"),
  Color::Red    => f.write_str("5;9"),
  Color::Rgb{r,g,b} => write!(f, "2;{r};{g};{b}"),
  Color::AnsiValue(val) => write!(f, "5;{val}"),
  ```

  → **`inquire` 的 `Color::LightGreen` 之所以渲染成 `38;5;10` 而不是 `\x1b[92m`，是 `crossterm` 的选择，不是 inquire 的限制。** 换 `Rgb` 即可拿到 `38;2;…`。
- **代价**：一旦覆盖，就落入 §3.3 的陷阱——`NO_COLOR` 需要自己处理。

### 3.5 现状与「叠印」的量化证据

vinoa 目前**从未**调用 `set_global_render_config`（`grep -n 'RenderConfig\|set_global_render_config' src/` 零命中；`src/wizard/mod.rs` 只 `use inquire::{Confirm, CustomUserError, InquireError, MultiSelect, Select, Text}`）。全部样式都是 `inquire` 默认值。

**实测方法**：用 `pty.fork()` 驱动 `/root/vinoa/target/debug/vinoa init` 走完 ①②③ 页（marker 驱动，确保包名合法以越过校验），捕获原始字节流，再用自写 VT 模拟器**逐格统计每个屏幕单元被写了多少次**（`grep -c` 会随交互步数变化，不可比）。

**（a）真 TTY 下的颜色集合——实测**（`TERM=xterm-256color` + `COLORTERM=truecolor`，`NO_COLOR` 已 unset）：

```
38;5;14  × 62     ← inquire 默认 LightCyan（help_message / answer）
38;5;10  × 32     ← inquire 默认 LightGreen（prompt_prefix）
38;5;9   ×  5     ← inquire 默认 LightRed（error_message，仅校验失败路径）
38;2;*   ×  0     ← 真彩：一个都没有
```

→ **spec 里「inquire 在真 TTY 下只上 2 种颜色，全是它的默认值」属实**（第 3 种 `38;5;9` 只在错误路径出现）。这条现已用**独立探针 + 真 pty 字节流**双重确认。

**（b）重绘/叠印——两种互补的量化方式**

**方式一：逐格重写次数**（不依赖按键步数）。同一屏内，每个单元格被重复写入的最大次数；「最短路径」= 只做 Enter 逐项确认，不按方向键、不键入过滤词、除包名外不改任何字段：

| 屏幕行（③ 页） | **最短路径** | 快速确认（含少量方向键） | 键入过滤词 |
|---|---|---|---|
| `> Minecraft version 26.2` | **3** | 3 | 3 |
| `> Platforms (paper implies bukkit) paper` | **3** | 3 | **15** |
| `  · paper implies the bukkit compatibility module` | **4** | 4 | 4 |
| `> Metadata format plugin.yml` | **4** | 4 | 10 |
| `? Continue?` | **4** | 4 | 8 |

**方式二：字节流里的串出现次数**（`#23` 的 `motion.md` §3.1 用的是这个）。用 `motion.md` 记录的原命令（注意必须带 `-p com.example.demo`）复跑，**数字逐项复现**：

| 串 | 复现结果 | `motion.md` §3.1 记录 |
|---|---|---|
| `Platforms` | **7** | 7 ✓ |
| `paper implies` | **7** | 7 ✓ |
| `Minecraft version` | **2** | 2 ✓ |
| `Target server` | **1** | 1 ✓ |
| `ED(2)`（`\x1b[2J`） | **0** | 0 ✓ |
| `EL`（`\x1b[K`） | **15** | 15 ✓ |

> ⚠️ **复跑时的两个陷阱**（我踩过，记录以免下游重踩）：
> 1. **必须带 `-p com.example.demo`**。若只写 `vinoa init demo-plugin`，默认包名 `com.example.demo-plugin` 含 `-`、不合法，向导会**卡在 ① 页包名校验**，字节流里 `Platforms` 出现 **0** 次。这不是数字不稳定，是命令不完整。
> 2. **数字随按键时序变化**（15 与 31 的差别即来自此），因此断言必须连同 `--keys` 一起固定。方式二与方式一测的是**不同的量**：方式二数「某串被输出了几次」（含重画），方式一数「同一格被覆盖了几次」。两者都有效，用途不同——方式二适合做回归断言（可精确复现），方式一适合说明「叠印几遍」的严重程度（下界 3，与步数无关）。

而**同一捕获**（`wiz3.bin`）里：

```
\x1b[2J（整屏清）      =  0 次   ← 全程从不整屏清
\x1b[K （擦到行尾）    = 15 次
\x1b[2K（擦整行）      = 31 次
\x1b[1A（光标上移）    =  4 次
```

> 顺带澄清 `motion.md` §3.1 里「`EL`（`\x1b[K`）15，另一次按键时序下为 31」：在本捕获中 `\x1b[K` = 15、**`\x1b[2K` = 31**。两者是**不同的转义序列**（`\x1b[K` 擦到行尾，`\x1b[2K` 擦整行），31 大概率是另一次捕获里 `\x1b[2K` 的计数而非同一序列的波动。这不影响该表的结论（`ED(2) = 0` 才是关键），但若下游要引用「31」，建议明确标注它对应哪个序列。

**源码侧对应**：`inquire` 的帧结束逻辑（`src/ui/frame_renderer.rs:271-320`）只做**逐行 hash 比较**——行内容变了就重写该行并 `clear_until_new_line()`；行消失则 `clear_line()`；然后 `\r` + `\n` 走到下一行。`src/terminal/crossterm.rs` 里全 crate 只有两处 `ClearType`，都是 `CurrentLine` / `UntilNewLine`（实测 `grep -rn 'ClearType::'` 只有 2 处），**没有任何 `ClearType::All`**。

→ **这解释了实测的 `ED(2) = 0`**：`inquire` 的绘制模型是「在一个自己维护的矩形里逐行擦写」，**不整屏清**，也**不知道**同一区域上还压着 vinoa 用 `eprintln!` 写的页面标题。

**根因定位（代码级）**：`src/wizard/mod.rs:58` / `:98` / `:114` / `:190` 用 `eprintln!` 写页面标题与只读字段，而 prompt 由 inquire 独立渲染到 **stderr**（`src/terminal/crossterm.rs:97` 的 `IO::Std(stderr())`）。两者虽同流，但**没有任何共享的网格/光标账本**——`eprintln!` 写完后光标停在某处，inquire 从**它自己以为的**位置开始画帧。页间来回切换（`Nav::Back`）会把标题重复写进同一区域。

> **关于「`Platforms` = 7」的复核结论：属实，可精确复现。** 我最初复跑失败得 0，原因是当时拿到的命令**漏了 `-p com.example.demo`**（见上方陷阱 1），不是数字本身不稳。带上 `-p` 后用 `motion.md` 的原命令复跑，6 项统计**逐项吻合**。因此 `motion.md` §3.1 的这张表**可以保留原样**，只需把 `--keys` 与 `-p` 一并固定在复现命令里。本报告额外提供「逐格重写次数」（方式一）作为与按键步数无关的补充量度，两者互补、不互相取代。

**（c）交互协议细节（顺带记录，供降级契约用）**：捕获流里可见 `\x1b[?2004h` / `\x1b[?2004l`（bracketed paste 开关，由 `src/terminal/crossterm.rs:91,237` 发出）与 `\x1b[?25l` / `\x1b[?25h`（光标隐藏/显示）。`\x1b[2J` 为 0 次意味着 **inquire 不会清理它没有画过的区域**——切页时旧页残留是必然的。

### 3.6 对 NO_COLOR 的态度（一句话）

**默认尊重（但把空串也当禁用，见 §1.3）；一旦你设置 `RenderConfig`，它就把这件事完全交还给你，不再介入。**

---

## 4. 非 TTY 行为

### 4.1 总表

| 组件 | 非 TTY 行为 | 来源 |
|---|---|---|
| `clap` / `anstream` | 自动禁用颜色（`AutoStream` 检测非终端）；`CLICOLOR_FORCE` 可强制覆盖 | `anstream-1.0.0/src/auto.rs:198-217`；本机实测：pipe 下 0 转义，`CLICOLOR_FORCE=1` 下 32 转义 |
| `crossterm`（颜色） | 颜色由 `ansi_color_disabled()` 把门；**它本身不检测 TTY**，检测 TTY 是调用方的责任 | `colored.rs:75-79`；实测 `Colored::ansi_color_disabled()` 与 `is_terminal()` 无关 |
| `crossterm`（尺寸） | 非 TTY 时 `terminal::size()` 走 `tput` 兜底；`tput` 失败则报错 | `terminal/sys/unix.rs:99-105, 273-297` |
| `inquire` | 非 TTY 时 prompt 返回 `InquireError::NotTTY`（vinoa 已映射为 `usage.invalid`，exit 64） | `inquire-0.9.4/src/error.rs:15,69`；vinoa `src/wizard/mod.rs:339`；本机实测返回 `ERR NotTTY` |
| `ratatui` | **无官方非 TTY 降级路径**；`TestBackend` 是测试后端，不是降级后端 | §2.3 grep 零命中；§4.3 实测 |

### 4.2 `inquire` 的 `NotTTY` 判定机制（源码级）

`inquire` **不做** `isatty()` 检查，它靠 **raw mode 失败**来判定。`src/error.rs:66-72`：

```rust
impl From<io::Error> for InquireError {
    fn from(err: io::Error) -> Self {
        match err.raw_os_error() {
            Some(25 | 6) => InquireError::NotTTY,   // ENOTTY(25) / ENXIO(6)
            _ => InquireError::IO(err),
        }
    }
}
```

而 `src/terminal/crossterm.rs:88-99` 的 `new()` 第一件事就是 `terminal::enable_raw_mode()?`。→ **判定链是：`enable_raw_mode` → 失败 → `errno 25/6` → `NotTTY`**。

**实测**：`inquire::Text::new("name?").prompt()` 在 stdin/stdout 都接管道时返回 `ERR NotTTY`（`< /dev/null`）。vinoa 把它映射成 exit 64（`src/wizard/mod.rs:339`）。

**⚠️ 一个必须记录的边界（实测发现）**：`inquire` 的 prompt 渲染**写 stderr**（`src/terminal/crossterm.rs:97` 的 `IO::Std(stderr())`），但 vinoa 的门禁 `should_run` 要求 **stdin 与 stdout 都**是 TTY（`src/wizard/mod.rs:22`）。两者不是同一个流：

```
pty: 什么都不重定向                        → 向导渲染  ✓
pty: stdout -> 文件（stderr 仍是 tty）     → 向导不渲染（vinoa 门禁拒绝，走 --yes 等价路径）
pty: stderr -> 文件（stdout 仍是 tty）     → 向导不渲染（inquire 写 stderr 失败/门禁拒绝）
```

第三种情形最值得注意：`inquire` **本该**能工作（它的输入是 stdin、输出是 stderr，两者都还是 tty），但 vinoa 的门禁看的是 stdout，于是整个向导被跳过。这不是 bug（对 vinoa 而言「stdout 不是终端 ⇒ 走非交互」是合理策略），但它意味着 **vinoa 的降级判定与渲染库的判定口径不一致**——降级契约必须明确「以哪个流为准」。

### 4.3 `ratatui` 在非 TTY 下的实测行为——**会照常写出 ANSI**

用一个最小 ratatui 程序（`CrosstermBackend::new(io::stdout())` + `Terminal::new` + `draw`），stdout 接管道：

**（a）`TERM` 未设置时，`Terminal::new` 直接失败：**

```
size = Err kind=WouldBlock msg=Resource temporarily unavailable (os error 11)
Terminal::new = Err kind=WouldBlock msg=Resource temporarily unavailable (os error 11)
```

原因：`crossterm` 的 `terminal::size()` 在 `ioctl(TIOCGWINSZ)` 失败后走 `tput` 兜底，而 `tput` 在 `TERM` 未设置时**报错且无输出**，`tput_value` 解析出 0 → 返回 `None` → 报 `last_os_error()`（实测拿到 `EAGAIN`/11，属于「兜底路径掩盖了真实原因」）。**这是 ratatui 在 CI 环境（常不设 `TERM`）下最可能的失败模式。**

**（b）`TERM` 有值（哪怕 `dumb`/`vt100`）时，尺寸兜底到 80×24，且 ANSI 照写：**

| `TERM` | `CrosstermBackend::size()` | `Terminal::new` | 实际写出的字节 |
|---|---|---|---|
| 未设置 | `Err(WouldBlock)` | `Err` | — |
| `dumb` | `Ok(80×24)` | `Ok` | `\x1b[1;1Hhi\x1b[39m\x1b[49m\x1b[59m\x1b[0m\x1b[?25l` … `\x1b[?25h` |
| `xterm-256color` | `Ok(80×24)` | `Ok` | 同上 |
| `vt100` | `Ok(80×24)` | `Ok` | 同上 |

（80×24 来自 `tput cols`/`tput lines` 在无终端时的返回，实测 `TERM=dumb tput cols` = 80、`tput lines` = 24。）

**（c）`NO_COLOR` 只影响颜色字节，不影响控制字节：**

| 环境 | 写出的字节 |
|---|---|
| `NO_COLOR` unset | `\x1b[1;1Hhi\x1b[39m\x1b[49m\x1b[59m\x1b[0m\x1b[?25l` |
| `NO_COLOR=1` | `\x1b[1;1Hhi\x1b[m\x1b[m\x1b[m\x1b[0m\x1b[?25l` |

注意 `NO_COLOR=1` 下**仍然有 `\x1b[1;1H`（光标定位）、`\x1b[?25l`（隐藏光标）、`\x1b[0m`（重置）**——因为 crossterm 的 `NO_COLOR` 只管 `Colored` 的显示（§1.2），**不管光标/清屏等控制序列**。`\x1b[m` 是 `Colored` 被禁用后 `fmt` 提前返回 `Ok(())`、由外层 `csi!("{}m")` 补上前后缀产生的空 SGR。

→ **对降级契约的直接含义**：「`NO_COLOR=1` 时输出不含 ANSI 转义」这个断言**在 ratatui/crossterm 路径上不成立**。要满足「非 TTY 不得出现 ANSI 转义」，必须在**是否启用 ratatui 之前**就短路（例如非 TTY 直接走纯文本路径），不能指望库层过滤。

### 4.4 `ratatui` 是否有官方的非 TTY 降级路径

**没有。** 依据：

- §2.3 的全量 grep：`is_terminal` / `NO_COLOR` / `available_color_count` 在 `ratatui-0.29.0/src/` 全部零命中。
- `Backend` trait 的实现里没有任何「非终端」分支；`TestBackend` 是**测试**后端（`src/backend/test.rs`），它把内容写进内存 `Buffer` 供断言，**不是**「终端不可用时自动切换」的后端。
- `Terminal::new` 只做 `backend.size()?`，失败即 `Err`（实测 §4.3a）。

→ **`TestBackend` 之外，ratatui 不提供任何非 TTY 降级路径。** vinoa 现有的「非 TTY 自动降级为非交互」是它**自己**在 `src/wizard/mod.rs:22` 与 `src/init.rs:346-349` 实现的（`is_terminal()`），并非来自任何库。上 ratatui 后这条逻辑仍归 vinoa。

### 4.5 小结（仅陈述事实）

- 三条颜色路径（clap / inquire / crossterm）对 `NO_COLOR` 的处理**互不一致**：clap 正确且含 `CLICOLOR_FORCE` 覆盖；crossterm 正确但需调用方主动查；inquire 空串判错。
- 非 TTY 降级**没有任何一个库提供**，是 vinoa 自己的责任。
- ratatui 在 `TERM` 未设置时会**硬失败**（`Terminal::new` 返回 `Err`），这对 CI 是可预期的风险点。
- `NO_COLOR` 不会消除 ratatui/crossterm 的控制序列（只有颜色 SGR 受影响）。

---

## 5. `ratatui 0.29` 的 `TestBackend` 能断言到什么粒度

### 5.1 存在性与机制

- `src/backend/test.rs` 存在，实现 `Backend` trait（`draw`、`append_lines`、`hide_cursor`、`clear_region` 等）。其 `draw` 就是把 `(x, y, &Cell)` 直接写进内部 `Buffer`（`src/backend/test.rs:237-246`）。
- `TestBackend` 提供现成断言辅助：`assert_buffer`（:145）、`assert_buffer_lines`（:189）、`assert_scrollback`（:159）、`assert_cursor_position`（:223）。
- `Terminal::draw` 走 **buffer diff**：`src/terminal/terminal.rs:198-201` 取 `previous_buffer.diff(current_buffer)`；diff 实现在 `src/buffer/buffer.rs:486-516`。

**实测（幂等性，最能说明 diff 的价值）**：同一 UI 连画两次，`buf1.diff(&buf2).len()` = **0**；换成不同 UI 后 = 123。即**未变化的帧不产生任何终端写入**。

这解释了为什么 ratatui 能**结构性**消除重绘垃圾：它只向终端写变化的格子，而 inquire 的逐 prompt 重画做不到（§3.5）。

### 5.2 断言粒度——**实测结论表**

用最小 ratatui crate（30×8 `TestBackend`，`Layout` 切上下三行 + 左右 50/50 分栏，渲染 `Paragraph` / `List` / `Gauge`）实跑：

| 粒度 | 可否 | 实测方式与结果 |
|---|---|---|
| 文本内容 | ✅ | `buf[(x,y)].symbol()` 逐格读，拼出整行 |
| 逐格样式 | ✅ | `buf[(x,y)].style()` 返回完整 `Style` |
| 前景色（16 色） | ✅ | `buf[(x,y)].style().fg == Some(Color::Green)` — 实测 Green 命中 2 格 |
| 前景色（真彩） | ✅ | `buf[(0,0)].style().fg` 实测 = `Some(Rgb(1, 2, 3))`，精确匹配 |
| `Gauge` 的 gauge_style 颜色 | ✅ | 实测 = `Some(Color::Blue)` |
| 属性 modifier | ✅ | `buf[(x,y)].modifier.contains(Modifier::BOLD)` — 实测 BOLD 命中 3 格，且与 `fg=Yellow` 组合可断言 |
| 布局分栏 | ✅ | 实测同一行同时含「左栏」与「右栏」→ `true` |
| 光标位置 | ✅ | `assert_cursor_position` / `backend().get_cursor_position()` |
| 滚动回滚区 | ✅ | `assert_scrollback`（仅 `Viewport::Inline` 相关） |
| 是否需要分离 I/O | **是** | 渲染函数须接受 `Frame`/`Buffer`，不能直接写 stdout |

**全部通过。** 结论：`TestBackend` 的粒度足以对「token 表的色值」「无色降级后的字符」「分栏布局」「进度条颜色」写**逐格断言**——这正是 vinoa 目前（向导零测试）最缺的能力。

### 5.3 ⚠️ CJK 宽字符陷阱（实测复现，重要）

宽字符占两格，**第二格的 `symbol()` 是一个普通空格 `" "`**，且**续格的 style 是默认值**（不是宽字符本身的样式）。实测 30×8 buffer 中 `日本語` 逐格转储：

```
x=5  symbol="日" w=2 | 续格 symbol=" "  续格.fg=Some(Reset) 续格.modifier=NONE | 本格.fg=Some(Yellow)
x=7  symbol="本" w=2 | 续格 symbol=" "  续格.fg=Some(Reset) 续格.modifier=NONE | 本格.fg=Some(Yellow)
x=9  symbol="語" w=2 | 续格 symbol=" "  续格.fg=Some(Reset) 续格.modifier=NONE | 本格.fg=Some(Yellow)
```

朴素拼接的结果：

```
row = "│宽 : 日 本 語    ││             │"
row.contains("日本語") = false      ← 屏幕上完全正确，测试却是红的
```

跳过续格后：

```
joined = "│宽: 日本語   ││             │"
joined.contains("日本語") = true    ← 正确
```

**两种正确的断言写法**：

1. **跳过续格**：遍历时若前一格 `symbol().width() == 2`，则跳过当前格。
2. **不依赖拼接**：直接对单格断言，或对「去掉所有空格」后的串断言（脆弱，不推荐——见下）。

**为什么不能简单去掉空格**：vinoa 的 UI 大量使用空格对齐（`  · paper implies…`、`[ ] paper`），去空格会破坏列语义，也会让 `"New Project"` 与 `"NewProject"` 混淆。

**源码侧依据**：`Buffer::set_stringn`（`src/buffer/buffer.rs:354-363`）在写入宽度为 2 的 grapheme 后，会把后续格 `reset()`：

```rust
for (symbol, width) in graphemes {
    self[(x, y)].set_symbol(symbol).set_style(style);
    let next_symbol = x + width;
    x += 1;
    // Reset following cells if multi-width (they would be hidden by the grapheme),
    while x < next_symbol { self[(x, y)].reset(); x += 1; }
}
```

而 `Cell::reset()`（`src/buffer/cell.rs:141-151`）把 `symbol` 设为 `" "`、颜色设为 `Color::Reset`——**这正是实测看到的续格形态**。`Buffer::diff`（`:486-516`）内部也用 `symbol().width()` 与 `to_skip` 正确处理双宽（源码注释给出 `aaa` → `aコ` 的例子），所以**渲染是正确的，只有测试断言容易踩坑**。

这一点对 vinoa 尤其要紧：它中英双语，而这将是它**第一次**有 UI 测试。若不预先记录，这个坑会在别人写第一条 UI 测试时以「测试莫名其妙挂了」的形式出现。

### 5.4 是否要求 UI 逻辑与终端 I/O 分离

**是，而且是强制性的。** `Terminal::draw` 的签名是 `draw<F>(&mut self, render_callback: F) where F: FnOnce(&mut Frame)`。渲染闭包只能拿到 `Frame`（即 `Buffer` 的视图），**拿不到 stdout**。要写测试就必须把「画什么」写成接受 `&mut Frame` 的纯函数，由外层决定用 `TestBackend` 还是 `CrosstermBackend`。

→ 这既是可测试性的来源，也是**改造成本**：vinoa 现在的 `src/wizard/mod.rs`（412 行）与 `src/wizard/form.rs`（474 行）是「边问边写 eprintln」的直线流程，没有「渲染函数」这一层。上 ratatui 意味着**必须**引入这一层。

---

## 6. 依赖与体积事实

> **本节方法说明**：下面所有解析结果都来自**把 ratatui 加进 vinoa 自己的 `Cargo.toml` + `Cargo.lock` 副本后跑 `cargo generate-lockfile`**，而不是只读 crate 的声明。这能捕捉到「crate 声明 MSRV 低、但传递依赖把 MSRV 顶上去」这类只看 `Cargo.toml` 会漏掉的事实。

> ⚠️ **`--offline` 会改变解析结果**（§6.2 实测）。本节的 §6.1 与 §6.3 结论在在线/离线两种模式下**一致**（涉及的两个版本都已在本机 registry 缓存中）；但 **§6.2 的 MSRV 结论只在线上成立**，因为它依赖 cargo 能看到缓存里没有的 `instability 0.3.10`。凡引用本节数字时，请注明解析模式。

### 6.1 crossterm 版本重复（决策关键）

| 组合 | 解析结果（实测） |
|---|---|
| `inquire 0.9.4` 单独（= vinoa 现状） | crossterm **0.29.0** 单版本 |
| `inquire 0.9.4` + `ratatui 0.29.0` | crossterm **0.28.1** 与 **0.29.0** 并存 |
| `inquire 0.9.4` + `ratatui 0.30.2` | crossterm **0.29.0** 单版本 |

来源：`ratatui-0.29.0/Cargo.toml:366-368` 声明 `crossterm = "0.28.1"`（optional），`:485-489` 的 `default = ["crossterm", "underline-color"]` 默认开启它。sparse index 亦确认 `ratatui 0.29.0 → crossterm req=^0.28.1`、`ratatui 0.30.x → crossterm req=^0.29`。

`cargo tree -i crossterm@0.28.1` 实测：

```
crossterm v0.28.1
└── ratatui v0.29.0
    └── vinoa v0.1.0
```

**MSRV 对照（实测 + sparse index）**：

| 版本 | 声明的 `rust-version` |
|---|---|
| `ratatui 0.29.0` | 1.74.0 |
| `ratatui 0.30.0` | 1.86.0 |
| `ratatui 0.30.1` | 1.88.0 |
| `ratatui 0.30.2` | 1.88.0 |

vinoa 声明 **1.85**（`Cargo.toml:5`），本机工具链 **1.98.1**。

### 6.2 `ratatui 0.29` 的 MSRV：**在 vinoa 配置下兼容 1.85，但原因是解析器、不是 ratatui 的声明**

初版这里写的「ratatui 0.29 MSRV = 1.74，低于 vinoa 的 1.85」是**误导**的——它只读了 ratatui 自己的声明。但我在复核过程中一度得出的相反结论（「ratatui 0.29 实际会把 MSRV 顶到 1.88」）**同样是错的**，而且错得很有价值。完整记录如下。

**根因链（源码实测）**：

```
ratatui 0.29.0
└── instability ^0.3.1        (ratatui-0.29.0/Cargo.toml:377-378，非 optional)
    └── darling 0.24.1        (proc-macro)
```

`instability` 的 MSRV 在 **0.3.11 处跳变**（sparse index 实测，共 15 个未 yank 版本）：

| instability 版本 | `rust_version` |
|---|---|
| 0.3.0 – 0.3.10 | 1.60 – 1.64 |
| **0.3.11 – 0.3.14** | **1.88** |

**实测（三组对照，同一 manifest，唯一变量是「在线/离线」）**：

| 变体 | 命令 | 解析出的 `instability` | cargo 警告 |
|---|---|---|---|
| A | vinoa 真实 manifest + 真实 `Cargo.lock`，**`--offline`** | **0.3.14** | 4 条 `requires Rust 1.88` |
| B | vinoa 真实 manifest，删 lock，**`--offline`** | **0.3.14** | 4 条 `requires Rust 1.88` |
| C | vinoa 真实 manifest，删 lock，**在线** | **0.3.10** | 无（只有 `Adding instability v0.3.10 (available: v0.3.14, requires Rust 1.88)`） |

变体 C 的实际输出：

```
Locking 182 packages to latest Rust 1.85 compatible versions
  Adding instability v0.3.10 (available: v0.3.14, requires Rust 1.88)
  Adding ratatui v0.29.0 (available: v0.30.2, requires Rust 1.88.0)
```

**结论：差异来自 `--offline`，不是 manifest。** 本机 registry 缓存里只下载了 `instability 0.3.14`（实测 `~/.cargo/registry/src/*/instability-0.3.14` 与 `cache/*/instability-0.3.14.crate` 各一份，无 0.3.10）。离线模式看不到 0.3.10，只能退而用本地缓存里最新的 0.3.14，于是触发警告；在线模式则正常执行 MSRV-aware 解析，**主动选中 0.3.10**。

**最小 crate 对照（进一步确认因果）**：

| 工程配置 | 解析结果 |
|---|---|
| `rust-version = "1.85"` + `edition = "2024"` + **在线** | `instability 0.3.10`，输出 `Locking 13 packages to latest Rust 1.85 compatible versions` |
| **无 `rust-version`** + `edition = "2021"` + 在线 | `instability 0.3.14`，输出 `Locking 12 packages to latest compatible versions`（注意：措辞里没有 "Rust 1.85 compatible"） |

→ 两个条件缺一不可：**manifest 里要有 `rust-version`**（cargo 才有 MSRV 约束可用），**且不能 `--offline`**（否则候选集不完整）。

**所以正确的表述是**：

> **`ratatui 0.29` 在 vinoa 的配置下（`edition = "2024"` ⇒ resolver 3、`rust-version = "1.85"`、在线解析）与 MSRV 1.85 兼容。但成立的原因不是 ratatui 声明的 1.74，而是 cargo 的 MSRV-aware 解析主动锁到了 `instability 0.3.10`。**

**这是一条脆弱点，必须记录**：这层安全边界**不在 ratatui 的声明里**，而在「调用方 manifest 声明了 `rust-version`」+「解析时在线且候选集完整」这两个前提上。具体风险：

1. **`cargo update` 可能破坏它**——若在无 `rust-version` 约束的环境（或离线环境）解析，会拿到 0.3.14。
2. **`--offline` / vendor 目录 / 内网镜像**若只镜像了 0.3.14，同样会拿到 0.3.14。
3. **ratatui 未来放宽 `instability` 约束**不影响；但若它把 `instability` 换成别的、或 `instability` 回退 MSRV，边界会移动。
4. 若要**不依赖解析器行为**地钉死这条边界，唯一确定的做法是**显式 pin `instability = "=0.3.10"`**（或更早）。这属于选型/规格决定，不在本报告范围。

> **方法论教训（值得下游注意）**：`cargo generate-lockfile --offline` 在依赖缓存不完整时会**静默地**给出与在线解析不同的结果，并附带 MSRV 警告。本报告的所有 lockfile 结论都应注明是否用了 `--offline`。§6.1 与 §6.3 的双版本结论不受影响（那两个包的两个版本都在本地缓存里，在线/离线结果一致），但**§6.2 的 MSRV 结论必须在线上复现**。

### 6.3 第二个重复依赖：`unicode-width`

实测：加 ratatui 0.29 后，lock 里出现**两个** `unicode-width`：

| 版本 | 谁用 |
|---|---|
| `0.2.2` | vinoa 现状（经 `inquire 0.9.4`，其 req 为 `"0.2"`） |
| `0.1.14` | `ratatui 0.29` → `unicode-truncate 1.1.0` |
| `0.2.0` | `ratatui 0.29` 直接依赖，**pin 为 `=0.2.0`**（`Cargo.toml:417-418`） |

`cargo tree -i unicode-width@0.1.14` 实测：

```
unicode-width v0.1.14
└── unicode-truncate v1.1.0
    └── ratatui v0.29.0
```

而 `ratatui` 的 `=0.2.0` pin 会把 `inquire` 原本用的 `0.2.2` **降级**到 `0.2.0`（实测 lock 中 0.2.2 → 0.2.0）。

→ **影响**：同一进程内存在两套 `unicode-width`（0.1.14 与 0.2.0），宽字符宽度计算可能有两套口径；且 `=0.2.0` 的 pin 会**阻塞** vinoa 未来升级 `unicode-width` 的补丁版本。这属于「上 ratatui 0.29 的隐藏成本」，初版未记录。

**对比：`ratatui 0.30.2` 无此问题**——实测它锁出 `unicode-width = ['0.2.2']` 单版本（与 vinoa 现状一致），`crossterm = ['0.29.0']` 单版本，且无 `instability`/`unicode-truncate`。

### 6.4 `ratatui 0.29.0` 的依赖面

`Cargo.toml`：`bitflags`、`cassowary`（布局求解器，实测锁定 0.3.0）、`compact_str`、`crossterm`（可选）、`document-features`（可选）、`indoc`、`instability`、`itertools`、`lru`、`palette`（可选）、`paste`、`serde`（可选）、`strum`、`unicode-truncate`、`unicode-width`。

- **license：MIT**（与 vinoa 的 Apache-2.0 兼容）。
- **声明的 MSRV 1.74**（但见 §6.2）。
- 默认 features：`["crossterm", "underline-color"]`。
- `ratatui` 通过 `pub use crossterm;`（`src/lib.rs:330`）**重导出 crossterm**，因此可以只用 ratatui 的 crossterm，不必在 `Cargo.toml` 里另列（这也是双版本问题的一个缓解点：直接事件循环可以用 ratatui 的 0.28，但 inquire 仍带 0.29，两者类型不互通）。

### 6.5 二进制体积

**未取得可信对照**。debug 产物（vinoa 101 MB）含调试信息，与带 ratatui 的 debug 产物（7.3 MB，因 crate 数量不同）不可比；release 侧只量到 vinoa 本身 **8.8 MB**，对照组未完成。**此项属未验证**，若选型需要该数据，应单独测量。

---

## 7. 本机无法验证的盲区

明确列出，避免下游票把它们当已知。**这一节是刻意保守的**——凡未亲手跑出来的，一律标为盲区，不做推断式断言。

### 7.1 平台类（本机是 WSL2 + `TERM=dumb`，无法验证）

| # | 盲区 | 为什么本机补不了 | 建议验证方式 |
|---|---|---|---|
| B1 | **旧 `conhost` 上 ANSI 的实际可用性** | 本机无 Windows；`crossterm` 的 `supports_ansi()` 依赖 `SetConsoleMode` 返回值，WSL 下走的是完全不同的 `cfg(not(windows))` 分支，**一行都没执行到** | GitHub Actions `windows-latest` 跑一个最小 `crossterm` 探针；或在真 Windows 10/11 的「传统控制台」模式下跑 |
| B2 | **旧 `conhost` 上圆角框线（`╭╮╰╯`）与树形字符（`├─└`）的渲染质量** | 同上。且 Microsoft 文档只保证 **DEC 线绘模式**（`ESC ( 0` + ASCII）有原生字形，**不保证**直接写 UTF-8 框线字节 | 真机截图；重点看默认代码页（437 / 936）与字体 |
| B3 | **旧 `conhost` 上 CJK 宽字符的列对齐** | 同上。Microsoft 文档**没有任何**双宽字符占两格的承诺；`ENABLE_LVB_GRID_WORLDWIDE` 的存在反而说明有语言相关分支 | 真机上跑中文界面，检查框线是否错位 |
| B4 | **`ratatui` 在 Windows 上的实测渲染** | 同 B1 | `windows-latest` + `TestBackend` 只能测逻辑，**测不到观感**；观感需真机 |
| B5 | **Windows 上 `available_color_count()` 返回 `u16::MAX` 是否与真实观感一致** | 源码显示 Windows 上只要 `supports_ansi()` 为真就报真彩，但 Microsoft 文档说旧 conhost 会把 `38;2` 就近吸附到 16 色表（§2.2c）——**两者矛盾，本机无法裁决** | 真机跑 `38;2` 与 `38;5` 对比 |
| B6 | **真彩（24-bit）在目标用户终端的普及度** | 未做跨终端抽样；`crossterm` 的探测只依赖 `COLORTERM`/`TERM` 字符串，而实测显示 `COLORTERM=truecolor` 能压过 `TERM=dumb`（§2.1），可能过于乐观 | 需要用户侧统计或问卷 |
| B7 | **macOS 上的行为** | 本机是 Linux/WSL，未在 macOS 上跑过任何一项 | 真机 |

### 7.2 时序 / 性能类（本机跑得出但未做定量）

| # | 盲区 | 说明 |
|---|---|---|
| B8 | **`ratatui` 逐帧动画的 CPU 占用** | 若最终决定做页面切换动画，需实测 tick 循环的 CPU 与帧率上限。本报告只证明了 `TestBackend` 的 diff 幂等性（同 UI 重画 diff=0），**没有**测真实终端的写入吞吐 |
| B9 | **二进制体积增量** | §6.5，对照组未完成 |
| B10 | **`inquire` 帧渲染的实际重绘频率与延迟** | 本报告统计的是**字节流里的重写次数**（交互驱动），不是「每秒多少次」。慢速终端的体感未测 |

### 7.3 依赖解析类（结论有时效性与环境依赖）

| # | 盲区 | 说明 |
|---|---|---|
| B11 | **MSRV 兼容性依赖解析环境，非 ratatui 声明** | §6.2：在线 + 有 `rust-version` 时锁 `instability 0.3.10`（保住 1.85）；**离线**（本机 registry 只有 0.3.14）或**无 `rust-version`** 时锁 0.3.14（需 1.88）。即「MSRV 兼容」是解析器行为，不是库的保证。未验证：CI/内网镜像/vendor 场景下的实际解析结果 |
| B12 | **离线 registry 缓存对解析结果的影响** | 本机 registry 只有 `ratatui-0.29.0` 源码（无 0.30.x），`ratatui = "0.30.2"` 的完整解析在离线模式下遇到 `js-sys`/`uuid` feature 冲突而失败。§6.1/6.3 中 0.30.2 的版本对比来自 **sparse index 元数据**，**不是**完整 lockfile 实测——**这一条证据等级低于其他条目** |
| B13 | **`NO_COLOR` 空串在真实用户环境中的出现频率** | 决定 §1.3 那个不一致值不值得专门测。理论上罕见（多数 shell 不会导出空值） |

### 7.4 明确「只是文档推断、未实测」的条目

以下条目**仅来自 Microsoft 官方文档**，本机**没有**任何可执行验证：

- §2.2a：ANSI 需要显式 `SetConsoleMode` 启用 —— **纯文档**。
- §2.2b：旧系统返回 `ERROR_INVALID_PARAMETER` —— **纯文档**。
- §2.2c：真彩被就近吸附到 16 色表 —— **纯文档**，且与 `crossterm` 的探测逻辑（§2.1）**存在矛盾**。
- §2.2d：DEC 线绘字符集 —— **纯文档**。
- §2.2e：`DISABLE_NEWLINE_AUTO_RETURN` 的行为 —— **纯文档**。
- §2.2f：CJK 宽字符无官方承诺 —— **纯文档的「未提及」**，不等于「不支持」。

### 7.5 已在本机实测、可复现的条目（非盲区）

为免下游误判，明确列出**本报告确已实测**的部分：

- `NO_COLOR` 在 crossterm / inquire / anstyle-query 三处的**真实分支**（§1.2、§1.3、§1.4）。
- `available_color_count()` 在 8 种 `TERM`/`COLORTERM` 组合下的**真实返回值**（§2.1）。
- `vinoa init --help` 在 pipe / pty 下、6 种环境组合的**ESC 字节计数**（§1.4）。
- `inquire` prompt 在非 TTY 下返回 `NotTTY`（§4.2）。
- `ratatui` 在非 TTY 下**照写控制序列**、`TERM` 未设置时 `Terminal::new` **硬失败**（§4.3）。
- `TestBackend` 的断言粒度、`diff` 幂等性、**CJK 续格 `symbol()=" "`**（§5.2、§5.3）。
- `Cargo.lock` 解析（在线/离线对照）：双 `crossterm`、双 `unicode-width`、`instability` 0.3.10 vs 0.3.14 的 MSRV 差异（§6.1–6.3）。
- 真 pty 下 ③ 页的**逐格重写次数**与 `\x1b[2J` = 0（§3.5b）。

---

## 8. 对选型的净影响（仅陈述事实含义，不做决定）

**倾向 ratatui 的事实：**
- `TestBackend` 提供逐格（含真彩、modifier）断言能力，`inquire` 需要真 pty，而 vinoa 的 wizard 当前**零测试**。
- `Terminal::draw` 的 buffer diff 是重绘垃圾的**结构性**解法（实测同 UI 重画 diff=0）。
- 整屏布局（分栏、常驻预览）只有它能做。
- 许可证兼容（MIT）；crossterm 可重导出。

**倾向留在 `inquire` 的事实：**
- 引 ratatui 0.29 会产生**两个 `crossterm`** **和两个 `unicode-width`**（§6.1、§6.3）。MSRV 本身**不构成障碍**（在线解析下锁 `instability 0.3.10`，保住 1.85），但那条边界**不在 ratatui 的声明里**，依赖调用方 manifest 与解析环境，属脆弱点（§6.2）。
- `ratatui` **不检查 `NO_COLOR`、不检测 TTY**，无色与非 TTY 降级全部自己做；且在 `TERM` 未设置时会硬失败。
- immediate mode 意味着事件循环、焦点、校验、页间回退全部自建，且**必须**先把渲染抽成 `&mut Frame` 纯函数（§5.4），这对现有 412 + 474 行的直线流程是结构性改造。
- 而 `inquire` 的样式层**其实够用**——`RenderConfig` 有 **18** 个可覆盖字段且**支持真彩**，当前只用到 3 个颜色（其中 1 个只在错误路径），说明「难看」主要是**没配置**，不是**配不了**（但布局与重绘确实配不了）。

**两边共同的前提（无论选哪条）：**
- 只要自定义样式，`NO_COLOR` 就得自己实现（`inquire` §3.3 明示放弃；`ratatui` 从未处理）。且三个库对 `NO_COLOR` 的口径**互不一致**，必须由 vinoa 定义优先级。
- 非 TTY 降级是 vinoa 自己的逻辑，两个库都不管。
- 「非 TTY 下不得出现 ANSI 转义」这条断言，**在 ratatui/crossterm 路径上无法靠库保证**，必须在启用渲染栈之前短路（§4.3c）。

---

## 附录 A：本报告的复现命令

```bash
# 版本事实（vinoa 现状）
grep -A1 'name = "crossterm"'  Cargo.lock    # → 0.29.0
grep -A1 'name = "inquire"'    Cargo.lock    # → 0.9.4
grep -A1 'name = "unicode-width"' Cargo.lock # → 0.2.2

# 依赖重复（把 ratatui 加进 manifest 副本后解析，不要动仓库）
cp Cargo.toml Cargo.lock /tmp/probe/ && cd /tmp/probe
sed -i 's/inquire = "0.9.4"/inquire = "0.9.4"\nratatui = "0.29.0"/' Cargo.toml
cargo generate-lockfile          # ⚠️ 必须在线：--offline 会因缓存不全锁到 instability 0.3.14
grep -A1 'name = "instability"' Cargo.lock   # 在线 → 0.3.10（保住 1.85）
cargo tree -i crossterm@0.28.1   # → ratatui 0.29 带来的第二个 crossterm
cargo tree -i unicode-width@0.1.14

# 颜色探测（最小 crate 探针）
env -u NO_COLOR probe info                       # ansi_color_disabled=false, count=8
env NO_COLOR=        probe info                  # 空串 → crossterm false，inquire colorless
env NO_COLOR=1       probe inquire-default       # → COLORLESS
env -u NO_COLOR COLORTERM=truecolor TERM=dumb probe info  # count=65535

# clap 路径（真二进制）
env -u NO_COLOR CLICOLOR_FORCE=1 ./target/release/vinoa init --help | grep -c $'\033'  # → 32
env NO_COLOR=1  CLICOLOR_FORCE=1 ./target/release/vinoa init --help | grep -c $'\033'  # → 0

# ③ 页重绘（真 pty）。必须带 -p 给出合法包名，否则卡在 ① 页包名校验、到不了 ③ 页
python3 scripts/termcap/ptycap.py --out .tmpcap/wiz3.bin --cols 100 --rows 40 --timeout 22 \
  --keys '0.8:\r,1.6:\r,2.4:\r,3.2:\r,4.0:\x1b[B\r,5.0:\r,6.0:\r,7.0:\r' \
  --cwd "$PWD/.tmpcap" -- "$PWD/target/debug/vinoa" init demo -p com.example.demo
grep -ao  'Platforms'      .tmpcap/wiz3.bin | wc -l   # → 7
grep -ao  'paper implies'  .tmpcap/wiz3.bin | wc -l   # → 7
grep -aoP '\x1b\[2J'      .tmpcap/wiz3.bin | wc -l   # → 0（inquire 从不整屏清）
grep -aoP '\x1b\[K'       .tmpcap/wiz3.bin | wc -l   # → 15（另 \x1b[2K → 31）
```

> **注意**：跑 ③ 页**必须给合法包名**（`-p com.example.demo` 或交互中改掉默认值）。默认包名 `com.example.demo-plugin` 含 `-`，不合法，向导会停在 ① 页——这正是「`Platforms` 出现 0 次」的唯一原因。§3.5 的「方式一」数据来自 marker 驱动的 `pty.fork()` 脚本（可精确控制交互步数），「方式二」数据来自上面这条命令。

## 附录 B：一句话速查

| 问题 | 答案 |
|---|---|
| 默认色数（探测失败时） | **8**（`DEFAULT = 8`） |
| `NO_COLOR` 空串 | crossterm：不禁用；inquire：**禁用**；clap：不禁用 |
| `NO_COLOR` 能否被覆盖 | 能——crossterm `force_color_output(true)`；clap `CLICOLOR_FORCE` |
| `CLICOLOR_FORCE` vs `NO_COLOR` | `NO_COLOR` 赢 |
| inquire 能否上真彩 | **能**（`Color::Rgb`） |
| inquire 默认用几色 | 3（`38;5;10`、`38;5;14`、错误时 `38;5;9`） |
| inquire 整屏清屏 | **从不**（`ClearType::All` 零命中） |
| ratatui 处理 `NO_COLOR` | **不处理** |
| ratatui 非 TTY 降级 | **无**（`TestBackend` 不是降级后端） |
| ratatui + `TERM` 未设置 | `Terminal::new` **返回 Err** |
| `TestBackend` 能断言颜色吗 | 能，含真彩与 modifier |
| CJK 续格 `symbol()` | `" "`（空格），style 为默认值 |
| ratatui 0.29 在 vinoa 下的 MSRV | **1.85 兼容**（cargo MSRV-aware 解析锁 `instability 0.3.10`；**离线或无 `rust-version` 时会变 1.88**） |
| ratatui 0.29 的重复依赖 | `crossterm`（0.28.1 + 0.29.0）、`unicode-width`（0.1.14 + 0.2.0） |

---

## 附录 C：与既有 spec 假设的对照（哪些属实、哪些需要修正）

| spec / 既有文档的表述 | 复核结论 | 证据 |
|---|---|---|
| 「inquire 在真 TTY 下只上 2 种颜色，全是它的默认值」 | **属实**（实测恰为 `38;5;10` + `38;5;14`；另有第 3 种 `38;5;9` 仅在**错误路径**出现） | §3.5a，真 pty 字节流 + 独立探针 |
| 「配色是捡来的 / 从未配置」 | **属实**——`grep -n 'RenderConfig\|set_global_render_config' src/` 零命中 | §3.5 |
| 「inquire 的样式层配不了更多颜色」 | **不属实**——`RenderConfig` 有 **18** 个 builder（另有 4 个在 `ErrorMessageRenderConfig`），`Color` 枚举含 `Rgb` 与 `AnsiValue`，**支持真彩** | §3.1、§3.4 |
| 「③ 页把同一块 UI 叠印 3–4 遍」 | **属实**。两种量度一致：字节流里 `Platforms` = 7、`paper implies` = 7（可精确复现）；逐格看 `Platforms` 行被重写 3 次（最短路径，键入过滤词时 15 次）、`· paper implies…` 行 4 次；全程 `\x1b[2J` = 0 | §3.5b |
| 「页面标题走裸 eprintln，prompt 走 inquire，两者不在同一网格」 | **属实**——`src/wizard/mod.rs:58,98,114,190` 是 `eprintln!`，prompt 渲染到 stderr（`inquire/src/terminal/crossterm.rs:97`），无共享光标账本 | §3.5b |
| 「ratatui 0.29 的 MSRV 低于 vinoa 的 1.85」 | **结论对，理由错**——ratatui 声明 1.74，但真正保住 1.85 的是 cargo 的 MSRV-aware 解析锁到 `instability 0.3.10`；这层边界**不在 ratatui 的声明里** | §6.2 |
| 「非 TTY 自动降级是 vinoa 自己实现的」 | **属实**（`src/wizard/mod.rs:22`、`src/init.rs:346-349`） | §4.4 |
| 「`NO_COLOR` 时输出不含 ANSI」 | **在 ratatui/crossterm 路径上不成立**——控制序列（`\x1b[1;1H`、`\x1b[?25l`、`\x1b[0m`）照写，只有颜色 SGR 被抑制 | §4.3c |

---
