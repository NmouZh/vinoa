# vinoa

创建 Minecraft 服务端插件工程的 CLI：**生成一个真能 `./gradlew build` 通过的工程**，而不是一堆模板文件。

```bash
vinoa init my-plugin -p com.example.myplugin -m 1.21.11 --platform paper
cd my-plugin && ./gradlew build
```

## 它解决什么

现成的插件模板要么只支持一个平台，要么把版本号写死在脚本里。Minecraft 的版本世界是一张**稀疏表**：Paper 没有 1.8.9 的 API（最早 `1.9.4`），Folia 从 `1.19.4` 才有，Minestom 只支持四个版本，`paper-api` 的坐标在 1.17 与 26.1 各翻转一次，Java 地板从 8 一路走到 25。手写模板必然在某处写错，而**写错的表现是"用户拿到工程却构建不了"**。

vinoa 把这些事实做成一份**内置版本矩阵**（每一条坐标都对着持有仓库实测过），用户只选 MC 版本与平台，其余由矩阵推出。

## 特性

- **向导交互**，形态对齐 IDEA 的"新建项目"（工程 / 构建 / 目标服务端 / 附加四页，可回退）。
- **非交互与 AI 友好**：全参数可用、`--dry-run` 预览、`--json` 单文档输出、非 TTY 自动降级、稳定错误码与退出码、`vinoa schema` 自描述。
- **多模块工程**：`core/` 放平台无关抽象，`platforms/<平台>/` 各自实现；勾 `paper` 自动带上 `bukkit` 兼容模块。
- **版本矩阵**：MC **1.8.9 → 26.2**，平台 paper / bukkit / velocity / bungeecord / folia / sponge / minestom。默认离线、可复现；`vinoa versions --refresh` 可选联网刷新。
- **环境预检**：按目标推导 Java 版本并检查本机是否具备（>16 走 toolchain 需要本机 JDK；≤16 走 `options.release`，不需要）。
- **质量工程**：checkstyle + 单元测试 + CI 默认生成；SpotBugs / 覆盖率 / 发布工作流可选。
- **原子落盘**：先写临时目录再整体 rename，失败不留半个工程。

## 安装 / 构建

```bash
cargo build --release       # 产物在 target/release/vinoa
cargo test                  # 单测
```

## 常用命令

```bash
vinoa init                                  # 向导
vinoa init --help                           # 全部参数
vinoa init my-plugin -p com.a.b -m 1.21.11 --platform paper --platform velocity -y
vinoa init my-plugin -m 1.8.9 --platform bukkit -y
# 可选模块（sqlite / bstats / update-check / placeholderapi / gui / spotbugs / coverage / release-ci）
# 正在补齐中：模板集尚未实现的 feature 会被硬报错拦住，不会静默生成空工程。
vinoa init -c vinoa.toml -y                 # 从配置文件
vinoa init my-plugin -m 1.21.11 --platform paper -y --print-config   # 导出配置
vinoa init my-plugin -m 1.21.11 --platform paper -y --dry-run --json # 预览（agent 用）
vinoa versions --refresh                    # 刷新版本矩阵
vinoa schema --json                         # 自描述
```

## 生成出来的工程

```
my-plugin/
├─ settings.gradle.kts · build.gradle.kts · gradle/libs.versions.toml
├─ gradlew · gradle/wrapper/            # Gradle 9.8.0
├─ core/                                 # 平台无关：命令 / 配置 / 消息 / 权限 / 调度 / 发送者
├─ platforms/paper/                      # 主类 + 示例命令 + 示例监听器 + plugin.yml + config.yml
├─ platforms/bukkit/                     # 勾 paper 时自动带上
├─ config/checkstyle/ · .github/workflows/build.yml
└─ README.md · LICENSE(Apache-2.0) · CHANGELOG.md
```

平台差异由矩阵驱动：`plugin.yml` 与 `paper-plugin.yml` 的命令声明方式互斥，模板不会同时生成；老版本目标用 `options.release`、现代目标用 Java toolchain。

## 验收

```bash
bash scripts/acceptance.sh 1.21.11 paper    # 单行：生成 → 自查残留 → ./gradlew build → 查 jar
bash scripts/acceptance-all.sh              # A2/A3/A6/A7 + B 级
```

保证矩阵（`docs/spec/vinoa-cli.md` §12）：`1.8.9 bukkit` / `1.12.2` / `1.16.5` / `1.21.11` / `26.2` paper + 两个代理端——这些组合在 CI 上真跑 `./gradlew build`；`folia`/`sponge`/`minestom` 与其它变体为 best-effort。

## 文档

- 规格（唯一事实来源）：[`docs/spec/vinoa-cli.md`](docs/spec/vinoa-cli.md)
- 决策草稿：[`docs/spec/drafts/`](docs/spec/drafts)
- 事实研究（每条都有来源）：[`docs/research/`](docs/research)
- 工作方式（issue tracker、wayfinder 操作）：[`AGENTS.md`](AGENTS.md)、[`docs/agents/`](docs/agents)

## 许可证

Apache-2.0。参考了社区模板 `CrimsonWarpedcraft/plugin-template`（GPL-3.0，**仅参考结构与工程纪律，未复制代码**）与 `sVoxelDev/multi-platform-plugin-template`（MIT）。
