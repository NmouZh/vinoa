# vinoa

跑完一条 `vinoa init`，拿到手的工程 `./gradlew build` 直接绿。vinoa 干的就是这件事。

说白了，它给的是一份能直接构建的 Minecraft 服务端插件工程，不是一堆你还得自己改的模板文件。严格说，模板它也在用——区别是那些容易写错的坑，已经被矩阵提前填平了。

```bash
vinoa init my-plugin -p com.example.myplugin -m 1.21.11 --platform paper
cd my-plugin && ./gradlew build
```

## 它解决什么

Paper 没有 1.8.9 的 API，最早只到 `1.9.4`。Folia 要等到 `1.19.4` 才出现。Minestom 一共四个版本。`paper-api` 的坐标在 1.17 和 26.1 各翻过一次面。Java 最低版本从 8 一路涨到 25。

这些差异，靠手写模板记，记不全。

更麻烦的是，记错了不会当场报错。往往要等用户拿到工程，`./gradlew build` 失败，你才发现模板里那行版本号写歪了。

vinoa 把这些事实全收进一份内置版本矩阵，每条坐标都对着上游仓库实测过。你只用选 MC 版本和平台，剩下的矩阵自己推。现成的插件模板，要么只认一个平台，要么把版本号写死在脚本里，毛病都差不多。

## 特性

- 向导长得像 IDEA 的"新建项目"，工程、构建、目标服务端、附加四页，能往回退。
- 命令行这块对脚本和 AI 都友好。参数能全给，`--dry-run` 先看，`--json` 出单文档，非 TTY 自动降级，错误码和退出码是稳的，`vinoa schema` 自己描述自己。
- 多模块工程，`core/` 放平台无关的抽象，`platforms/<平台>/` 各自实现。勾上 `paper`，`bukkit` 兼容模块自动跟上。
- MC 1.8.9 到 26.2 都能出，平台七个：paper / bukkit / velocity / bungeecord / folia / sponge / minestom。默认离线、可复现；想要新数据，`vinoa versions --refresh` 联网拉。
- 生成前会跑一次环境预检，按目标反推需要的 Java 版本，再看本机有没有。16 以上走 toolchain，本机得装 JDK——16 及以下走 `options.release`，不用装。
- checkstyle、单元测试、build CI 默认就带；SpotBugs、覆盖率、发布工作流可选。
- 落盘是原子的，先写临时目录，最后整体 rename 过去，中途失败不留半个工程。

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

平台差异也交给矩阵。`plugin.yml` 和 `paper-plugin.yml` 声明命令的方式互斥，模板不会两个都生成。老版本目标走 `options.release`，新的走 Java toolchain。

## 验收

```bash
bash scripts/acceptance.sh 1.21.11 paper    # 单行：生成 → 自查残留 → ./gradlew build → 查 jar
bash scripts/acceptance-all.sh              # A2/A3/A6/A7 + B 级
```

下面这些组合属于 [`docs/spec/vinoa-cli.md`](docs/spec/vinoa-cli.md) §12 的保证矩阵，CI 上真跑 `./gradlew build`——`1.8.9 bukkit`、`1.12.2`、`1.16.5`、`1.21.11`、`26.2` 的 paper，再加两个代理端。`folia`/`sponge`/`minestom` 和其它变体是 best-effort。

## 文档

- 规格，唯一事实来源：[`docs/spec/vinoa-cli.md`](docs/spec/vinoa-cli.md)
- 决策草稿：[`docs/spec/drafts/`](docs/spec/drafts)
- 事实研究，每条都有来源：[`docs/research/`](docs/research)
- 工作方式，issue tracker 和 wayfinder 操作：[`AGENTS.md`](AGENTS.md)、[`docs/agents/`](docs/agents)

## 许可证

Apache-2.0。参考了社区模板 `CrimsonWarpedcraft/plugin-template`（GPL-3.0，只参考了结构和工程纪律，代码没复制）和 `sVoxelDev/multi-platform-plugin-template`（MIT）。
