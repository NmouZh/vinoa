# vinoa `init` 执行流程（草稿 v1）

> 供 ticket [#17 init 的执行流程与阶段划分](https://github.com/NmouZh/vinoa/issues/17) 讨论用。
> 主体是**用户可见的完整流程**；内部阶段划分见附录。本文件是待你确认的草稿，不是最终规格。

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

── 目标平台与版本 ────────────────────────
? 目标平台（空格多选，默认 paper）:
  [x] paper      [ ] bukkit     [ ] velocity   [ ] bungeecord
  [ ] folia      [ ] sponge     [ ] minestom   [ ] nukkit
? 目标 Minecraft 版本: 1.21.11        （矩阵内可选：1.8.9 … 26.2）
   → 已解析: paper-api 1.21.11-R0.1-SNAPSHOT · Java 21 · Gradle 9.8.0
? 元数据格式: ( ) plugin.yml（推荐）   ( ) paper-plugin.yml（实验性）

── 可选项 ────────────────────────────────
? 生成物语言: 中文 / English / 双语
? 可选模块: [ ] 持久化(SQLite)  [ ] 运行时多语言  [ ] GUI 骨架  [ ] 生态集成
? 初始化 git 仓库: 是

── 确认 ──────────────────────────────────
即将创建（共 27 个文件）:
  my-plugin/
  ├─ settings.gradle.kts · build.gradle.kts · gradle/libs.versions.toml
  ├─ common/            共享抽象（命令 / 配置 / 消息 / 日志）
  ├─ platform/paper/    Paper 模块 + plugin.yml + 示例命令 + 示例监听器
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
  注意: 当前目标是 1.21.11（Java 21）。若切换到 1.8.9 及更早版本，
        需 Java 8 toolchain，部分质量工具会降级为 best-effort 配置。
  文档: docs/usage.md · 模板变量说明见 README
```

## 2. 非交互等价命令（AI / CI 路径）

```bash
vinoa init --name my-plugin --package com.example.myplugin \
  --platform paper --mc 1.21.11 --lang zh --license MIT \
  --git --yes
```

- `--dry-run`：只打印上面"即将创建"那一段（同一份计划，保证"预览即实际"）。
- `--json`：把计划/结果输出成结构化 JSON，供 agent 解析。
- `--yes`：跳过确认；非 TTY 环境下自动等价于 `--yes`。
- 缺必填参数且无法询问 → 报错退出并列出缺失项（不挂起）。

## 3. 失败时用户看到什么

| 情形 | 用户看到 |
|---|---|
| 目标目录非空 | `目录已存在且非空`，列出冲突文件，给出 `--output <other>` 或明确拒绝（默认不覆盖） |
| 平台 × 版本组合不存在 | 明确报错 + 可用组合列表（交互路径下根本选不到） |
| 版本矩阵联网刷新失败 | 回退内置矩阵并警告；离线不阻塞生成 |
| 中途中断 / 渲染失败 | 不留半个工程（原子落盘），说明已回滚 + 可复现命令 |
| `--verify` 构建失败 | 工程**保留**，报告失败原因与日志路径，退出码标记"生成成功、验证失败" |

## 4. 已确认（2026-09-25，与用户逐条确认）

1. **入口形态**：`vinoa init` 作用于当前目录；`vinoa init <name>` 新建子目录。
2. **提问节奏**：分三组（身份 / 平台与版本 / 可选项），组间可回退。
3. **确认步骤**：默认打印摘要 + `Y/n`；`--yes` 与非 TTY 跳过。
4. **完成提示**：路径 + `cd` + `./gradlew build` + `./gradlew runServer` + 版本注意事项 + 文档链接。
5. **失败语义**：照 §3 —— 非空目录不覆盖、原子落盘无残留、联网刷新失败回退内置矩阵并警告、`--verify` 失败保留工程并用单独退出码。

**内部实现选择（用户不必关心，可随时推翻）**：采用"先算完整计划 → 落盘只执行计划"的单一代码路径，使 `--dry-run` 的输出与实际写入必然一致；阶段划分见附录 0–8。

## 附录：内部阶段划分（用户不可见）

| # | 阶段 | 职责 | 失败产物 |
|---|---|---|---|
| 0 | 上下文与 TTY 检测 | 目录占用、是否在 git 仓库内、是否离线、TTY 与否 | 无 |
| 1 | 输入收集 | flags + 交互补齐 | 无 |
| 2 | 校验 | 名称/包名合法性、目录占用、平台×版本组合 | 无 |
| 3 | 版本矩阵解析 | 锁定 paper-api / Java / Gradle / 工具链 / 元数据格式 | 无 |
| 4 | 计划生成 | 完整文件清单 + 内容 + 额外动作（这就是 `--dry-run` 的产物） | 无 |
| 5 | 渲染 | 模板树 → 内存文件集（变量替换、条件文件、重命名） | 无 |
| 6 | 落盘 | 临时目录 → 整体 rename（原子） | 回滚，无残留 |
| 7 | 收尾 | `git init` + 首次提交、可选 `--verify` | 见 §3 |
| 8 | 结果输出 | 人类可读摘要 / `--json` | — |
