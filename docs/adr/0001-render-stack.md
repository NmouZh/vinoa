# ADR-0001 渲染方案选型

- **状态**：proposed
- **日期**：2026-09-26
- **相关**：issue #21 · supersedes: — · superseded by: —
- **依赖**：`docs/spec/ui/direction.md`（#19 方向锁定）、`docs/research/terminal-render-capabilities.md`（#20 事实报告）

## Context

向导（`vinoa init` 的交互路径）当前的渲染底座是 `inquire 0.9.4`，一行样式都没配。三条实测事实决定了这个决定必须现在做：

**1. `inquire` 在真 TTY 下只上 3 种颜色，全是它的默认值，且没有真彩。**
用仓库自带的 `scripts/termcap/ptycap.py` 驱动真 pty（`TERM=xterm-256color`、`COLORTERM=truecolor`）跑完整向导，捕获字节流里的 SGR 集合恰好是：

```
38;5;10   提示符 "?" / ">"        （inquire 默认 LightGreen）
38;5;14   帮助文本 "[...]"        （inquire 默认 LightCyan）
38;5;9    验证错误行 "#"          （inquire 默认 LightRed，仅错误路径）
38;2;*    真彩                    0 个
```

复现：`python3 scripts/termcap/ptycap.py --out <out> --cols 100 --rows 40 --timeout 25 --cwd <可写目录> -- /root/vinoa/target/debug/vinoa init demo-plugin`。
这与 spec §4 那张整屏 mockup 的差距是**结构性的**：spec 画的是整屏布局，而 `inquire` 只画一个个 prompt。

**2. 第 ③ 页把同一块 UI 叠印 7 遍（不是 3–4 遍）。**
同一份捕获里统计字符串出现次数：`Platforms` → **7 次**、`paper implies` → **7 次**。
根因在字节流里直接可见：整段捕获 `\x1b[2J`（整屏清）出现 **0 次**，而 `\x1b[K`（行内擦除）出现 **31 次**。页面标题走裸 `eprintln!`（`src/wizard/mod.rs` 第 58/91/98/114/190 行附近）一次性写出、**从不重绘也从不清除**；而 prompt 在同一区域反复擦写。两者不共享网格，于是标题块被 prompt 的擦写扫过一遍又一遍。
**这不是"加动效"能修的，是绘制模型的问题。**

**3. 向导当前零测试。**
`src/wizard/` 下没有任何 `#[test]`。`inquire` 需要真 pty 才能驱动，这正是零测试的原因——测试成本高到没人写。对比：`ratatui` 的 `TestBackend` 让 UI 渲染进内存 buffer 后逐格断言。

**4. 方向已锁定为 REC，且**保留**了右侧常驻预览面板。**
`direction.md` §2.2 裁定保留「即将生成」预览面板，理由是它把"选项"翻译成"产物"（勾 `paper` → 树里多出 `platforms/bukkit 6 files`）。**这是一个持续重绘的常驻区域**，而 `inquire` 是"一问一渲染"模型，做不出常驻区域。这一条把选型推向了全屏接管。

**5. 能力边界（来自 #20 的事实报告，非本节推断）。**
- `ratatui` 的 crossterm 后端**完全不检查 `NO_COLOR`**（`ratatui-0.29.0/src/` 内 grep `NO_COLOR` 无命中）。
- `ratatui` **不检测 TTY**。
- `inquire` 的 `RenderConfig::default()` 用 `env::var("NO_COLOR")` 判断，`Ok(_)` 即禁用——**空字符串也被判定为禁用**，与 no-color.org 规范（空串**不**禁用）冲突。
- 一旦显式设置 `RenderConfig`，`inquire` 不再自动处理 `NO_COLOR`（源码注释明示）。

即：**无论选哪条路，无色降级与非 TTY 降级都得 vinoa 自己做。**

## Decision

**采用 `ratatui 0.29 + crossterm` 全屏接管向导，`inquire` 移出向导路径。**

责任边界，写清楚：

1. **渲染**：由 `ratatui` 的 immediate-mode 事件循环接管整屏；向导四页与右侧预览面板绘制到**同一张 `Buffer`**，每帧一次 `Terminal::draw`，由 `Buffer::diff()` 决定实际写哪些格子——这是第 ② 条叠印缺陷的结构性解法（`ratatui-0.29.0/src/terminal/terminal.rs:196-201` 的 `flush()` 走 `previous_buffer.diff(current_buffer)`）。
2. **无色 / 非 TTY / 窄终端降级**：**由 vinoa 自己保证，不依赖 `ratatui` 提供**（它不检查 `NO_COLOR`、不检测 TTY）。契约落在 `docs/spec/ui/degradation.md`（#25），实现落在 vinoa 自己的渲染入口。
3. **色彩档位探测**：由 vinoa 自己实现，不直接用 `crossterm::style::available_color_count()`——它的默认值是 **8**（`crossterm-0.29.0/src/style.rs:163-178` 的 `const DEFAULT: u16 = 8`），而 `direction.md`/`tokens.md` 的档位是 真彩 → 256 → 16 → 无色。#25 必须显式裁定"16"这一档从哪来。
4. **`inquire` 的去留**：向导路径移除。`src/init.rs:352` 还有一处 `inquire::Confirm` 用于交互确认，**同样迁到 vinoa 自己的渲染**，以便整个交互路径只有一套绘制模型、一套降级逻辑。若最终保留 `inquire` 作为降级实现，必须单独论证维护两套的成本——本 ADR 不建议。
5. **可保留的部分**：`src/wizard/form.rs` 里与 `inquire` 无关的纯函数**原样保留**——它们是向导的业务逻辑，不是渲染逻辑。

## Consequences

### 正面

- **叠印缺陷被结构性消除**：`Terminal::draw` 走 buffer diff，只写变化的格子。第 ③ 页 7 遍叠印的根因（裸 `eprintln!` 与 prompt 不共享网格）消失。
- **向导首次可测**：`TestBackend` 让 UI 渲染进内存 buffer 逐格断言。lead 已实测其粒度（编译运行最小 crate 验证）：
  - 文本：`buf[(x, y)].symbol()` 可读；
  - 颜色：`buf[(x, y)].style().fg` → `Some(Indexed(74))`；
  - 字重：`.style()` 的 `modifier` 含 `BOLD`；
  - 布局：`Layout` + `Constraint::Percentage` 多栏可解、可分栏断言；
  - 进度：`Gauge` 的 `gauge_style` 颜色可断言。
  - **CJK 陷阱已复现**：宽字符的续格 `symbol()` 返回 `" "`（空格）而非空串，且续格 style 是默认值——naive 拼接会让 `contains("目标服务端")` 失败。测试助手必须按显示宽度推进列号。
- **整屏布局才做得出来**：右侧预览面板靠 `Layout` + `Constraint` 求解。
- **许可证兼容**：`ratatui 0.29.0` 是 MIT（vinoa 是 Apache-2.0），兼容。
- **MSRV 兼容 1.85，但成立的原因不是 ratatui 的声明**：`ratatui` 自己声明 1.74，其传递依赖 `instability` 的 MSRV 在 **0.3.11 处从 1.6x 跳变到 1.88**。实测（vinoa 真实 manifest + `ratatui = "0.29.0"`，**在线**解析）cargo 输出 `Locking 182 packages to latest Rust 1.85 compatible versions` 并锁到 **`instability 0.3.10`**。
  **同一份 manifest 加 `--offline` 会锁到 `instability 0.3.14` 并给出 4 条 `requires Rust 1.88` 警告**（本机 registry 只缓存了 0.3.14）。即：1.85 这条线依赖「manifest 声明了 `rust-version`」+「解析时在线且候选集完整」两个前提，**不是 ratatui 声明里的保证**。详见 `docs/research/terminal-render-capabilities.md` §6.2 的三组对照。

### 负面（具体代价）

- **要重写两个文件**：`src/wizard/mod.rs`（412 行）与 `src/wizard/form.rs`（474 行）中的渲染部分。`form.rs` 有 **12 个 `pub fn`**（`validate_project_name` / `validate_package_name` / `platform_options` / `compare_mc` / `system_ui_language` 等）与 `inquire` 无关，**可原样保留**。
- **13 处 prompt 调用点要重建**：`src/wizard/mod.rs` 里有 13 个 `(Text|Select|MultiSelect|Confirm)::new` 构造点。immediate mode 意味着事件循环、输入、焦点、校验、页间回退全部自建。
- **依赖树重复，实测为三处（不止 crossterm）**：把 `ratatui = "0.29.0"` 加进 vinoa 的真实 manifest 后跑 `cargo generate-lockfile`，lock 里出现三个重复 crate：

  | crate | 现状（仅 `inquire`） | 加 ratatui 后 | 原因 |
  |---|---|---|---|
  | `crossterm` | `0.29.0` | **`0.28.1` + `0.29.0`** | ratatui 要 `^0.28.1`（`ratatui-0.29.0/Cargo.toml:366-368`），inquire 要 0.29 |
  | `unicode-width` | `0.2.2` | **`0.1.14` + `0.2.0`** | ratatui **精确 pin** `=0.2.0`（`ratatui-0.29.0/Cargo.toml:417-418`），把原本统一的 0.2.2 挤成两份；`unicode-truncate` 又拉 0.1.14 |
  | `instability` | 无 | `0.3.10` | ratatui 的传递依赖（`^0.3.1`） |

  即：**ratatui 不是"不引新东西"，它引 3 个新 crate 并制造 2 组重复编译。** 这与 issue #21 原文里「后端不引新东西」的说法不符，本 ADR 以实测为准。
- **`unicode-width` 的 pin 值得单独警惕**：ratatui 用 `=0.2.0`（精确等号）而非 `^0.2.0`，这会在 vinoa 其它依赖想升 `unicode-width` 时**硬顶住**。vinoa 目前只有 `inquire` 用它（0.2.2），所以表现为"多编译一份"而不是"降级"。
- **MSRV 安全边界在解析环境，不在库声明里**：`instability 0.3.11+` 起 `rust-version = 1.88`，高于 vinoa 的 1.85。能锁到 0.3.10 靠的是 cargo 的 MSRV-aware 解析（edition 2024 ⇒ resolver 3）**且在线**。**`--offline` / vendor 目录 / 内网镜像只镜像了 0.3.14 时，解析会落到 1.88 并给出 `requires Rust 1.88` 警告**（本机实测）。CI 若用离线 vendor 构建，需要显式确认 `instability` 锁在 ≤0.3.10，或把它写进 `Cargo.lock` 并走 `--locked`。
- **既有测试契约需重建并重测**：`Esc → exit 130`（`interrupt.cancelled`）等既有约定要靠新的绘制模型重新实现并重新覆盖。
- **无色 / 非 TTY / 窄终端降级全部自建**：这是净增的维护面，没有上游库兜底（见 Context 第 5 条）。
- **「非 TTY 不得出现 ANSI」不能靠渲染栈保证**：实测 `ratatui` 在 `TERM` 有值时即使 stdout 不是 TTY 也**照写控制序列**（`\x1b[1;1H` / `\x1b[?25l`），`NO_COLOR=1` 只抑制颜色 SGR。这条硬契约只能在**选择渲染栈之前短路**（见 `docs/spec/ui/degradation.md`）。
- **页面切换若做逐帧动画，会引入 tick 循环与 CPU 占用**：动效边界见 #23。

## Alternatives considered

### A1. 留在 `inquire`，只配 `RenderConfig` 样式层（否掉）

`RenderConfig` 确实有约 35 个可覆盖字段（`inquire-0.9.4/src/ui/api/render_config.rs`），当前只用到 2 个颜色——**"难看"确实有一部分是"没配置"**。

但否掉，因为两条**配不了**的能力正是方向锁定的核心：

1. **布局配不了**：做不出右侧常驻预览面板（`direction.md` §2.2 已裁定保留）。
2. **重绘配不了**：`inquire` 逐 prompt 渲染，`eprintln!` 与 prompt 不在同一网格——第 ③ 页 7 遍叠印改不掉。

即：样式层能改善**颜色与字形**，改不了**布局与绘制模型**。而方向锁定要求的恰恰是后者。

### A2. 留在 `inquire`，放弃预览面板以保住它（否掉）

这是一条自洽的路线：丢掉预览面板 ⇒ 不需要全屏接管 ⇒ `inquire` + 样式层够用，改动量最小（13 处调用点保持不动）。

否掉，因为代价落在**用户看得见的能力**上：预览面板是唯一把"勾 `paper`"与"多出 `platforms/bukkit 6 files`"这个因果显式呈现给用户的地方（`direction.md` §2.2）。为了省下重写成本而丢掉它，等于用"实现省事"换"用户少知道一件事"——与 issue #18 的"人面优先"定位冲突。

### A3. `ratatui 0.30.x`（不选，非因缺陷）

`ratatui 0.30.2` 的依赖形态明显更好（crates.io sparse index 实测）：`crossterm ^0.29`（与 `inquire` 同版，**消除 crossterm 重复**）、`instability ^0.3`，且**不再有 `unicode-width = "=0.2.0"` 那个精确 pin**（0.29.0 有，0.30.2 没有）——即 A3 能同时解决本 ADR 负面栏里的两处重复。

不选它只因为一条：**`rust-version = "1.88.0"`**（index 实测），高于 vinoa 声明的 `rust-version = "1.85"`。抬 MSRV 是独立决定（影响 `edition 2024` 之外的兼容承诺与 CI 镜像），不在本 ADR 范围。

**这条是本次选型里最值得回看的一项**：0.29 的三处依赖重复（`crossterm` / `unicode-width` / 新增 `instability`）里有**两处**会被 0.30 直接消掉。若将来 vinoa 抬 MSRV 到 1.88，应把「迁到 ratatui 0.30」作为一条独立 ADR 重新评估——届时的净收益是**净减**依赖，而非现在这样的净增。

### A4. 自己写终端渲染（否掉）

维护面最大：光标移动、SGR、擦除、宽字符、Windows 控制台，全部自建。而 `ratatui` 已提供 buffer diff、`Layout` 求解与 `TestBackend`；后端 `crossterm` 也**已经**在依赖树里（`inquire` 带来 0.29.0，无需新增该 crate 本身）。重复造轮子的收益为零。

（注意：说"crossterm 已在依赖树"只对 **crossterm 本身**成立。ratatui 0.29 仍会新增 `instability`、并把 `crossterm`/`unicode-width` 变成两份——精确代价见 Consequences 负面栏。本 ADR 初稿曾据此写成"后端不引新东西"，实测后已更正。）

## 未决 / 交接

- **#25 必须显式裁定"16 色"档从哪来**：`crossterm` 探测默认值是 8，不是 16（Context 第 5 条 / Decision 第 3 条）。
- **真机 Windows 观感未验证**：本机是 WSL + `TERM=dumb`，旧 `conhost` 上的 ANSI、圆角框线与 CJK 宽字符只能靠 CI（`windows-latest` 已在 `acceptance.yml`）或用户确认。属已知盲区（详见 `docs/research/terminal-render-capabilities.md` §7）。
- **`inquire` 是否彻底移出依赖树**：Decision 第 4 条要求向导路径移除；`src/init.rs:352` 那处 `Confirm` 的迁移是本 ADR 的直接后续工作。
