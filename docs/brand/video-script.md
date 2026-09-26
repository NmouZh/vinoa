# vinoa 演示视频台本

> 时长目标 4 分 30 秒。全程对着终端录屏，不需要 PPT。
> 录制前先做 §0 的准备，否则现场会卡在 Java 版本和 Gradle 下载上。
> 括号里是动作提示，不要念出来。

---

## §0 录制前准备（不录）

```bash
# 1. 确认二进制
./target/release/vinoa --version        # 期望 vinoa 0.1.0

# 2. 预热的 Gradle 缓存（关键，否则现场 build 要等 3 分钟）
cd /root/vinoa/.tmp-demo && ./gradlew build && cd -

# 3. 清掉演示目录
rm -rf ~/demo && mkdir -p ~/demo

# 4. 终端设置
#    - 宽度 ≥ 100 列（向导的完整形态需要 100 列，窄了会丢预览面板）
#    - 字号 16–18，深色主题
#    - 关掉桌面通知
```

**开场前必须确认一件事**：本机装了 JDK 21。`1.21.11` 的 paper 工程需要它，缺了预检会当场问你。

```bash
java -version    # 期望 21
```

---

## §1 开场（0:00 – 0:35）

**画面**：空终端，光标闪。**不要**先跑命令。

> 「做一个 Minecraft 插件，最烦的不是写代码。
>
> 是开头那半小时。你得先搞清楚这个版本用哪个 API 坐标，Java 要几，Gradle 要几，然后把这些抄进 build 文件。抄错一个数字，不报错，等到 `./gradlew build` 那一刻才炸。中间你可能已经写了二十行业务代码了。
>
> vinoa 干的就是这一段。跑一条命令，给你一个 `./gradlew build` 直接绿的工程。」

（停一拍，再敲下一条）

---

## §2 主演示：一条命令出一个工程（0:35 – 1:45）

### 2.1 敲命令

```bash
cd ~/demo
vinoa init my-plugin -p com.example.myplugin -m 1.21.11 --platform paper
```

**画面**：四页向导出现。**不要**急着按。

> 「它给的是向导。四页，工程、构建、目标服务端、附加。长得像 IDEA 的新建项目。」

（在第 ① 页停 3 秒，让观众看清右边的「即将生成」面板）

> 「注意右边这块。这是它跟别的脚手架不一样的地方。你左边改什么，右边立刻告诉你最后会生成什么。」

### 2.2 走完向导（这是全片最重要的镜头）

（第 ① 页：输入名称，按 Enter）

（第 ③ 页：**慢下来**，勾选 `paper`）

> 「现在勾 paper。」

（停顿 3 秒，指着预览面板）

> 「看到没有。勾了 paper，预览里多出一个 `platforms/bukkit`。为什么？因为 paper 的插件在 bukkit 服务端上也要能跑，所以兼容模块自动跟上。
>
> 这件事你在任何文档里都查不到一句话说明。但在这里，它是可见的。」

（继续勾，走到第 ④ 页，确认）

**要点**：这个「勾 paper → 多出 bukkit」的因果是演示的核心。宁可多停 3 秒，也不要一带而过。

### 2.3 生成 + 构建

```bash
cd my-plugin
./gradlew build
```

**画面**：Gradle 跑完，出现 `BUILD SUCCESSFUL`。

> 「一个警告都没有，直接绿。刚才那半小时，现在是十秒。」

```bash
ls platforms/paper/build/libs/
```

> 「jar 在这。可以直接丢进服务端 `plugins/` 目录。」

---

## §3 讲清它到底解决什么（1:45 – 2:45）

**画面**：切到 `vinoa versions` 的输出。

```bash
vinoa versions
```

> 「它凭什么知道那些坐标？因为它内置了一份版本矩阵，61 个 MC 版本，7 个平台。每一条坐标都对着上游仓库实测过。
>
> 这份矩阵是硬编码进二进制的。默认不联网，离线也能跑。」

**画面**：滚一下那 61 个版本号。

> 「Paper 没有 1.8.9 的 API，最早只到 1.9.4。Folia 要等到 1.19.4 才出现。Minestom 一共四个版本。`paper-api` 的坐标在 1.17 和 26.1 各翻过一次面。Java 最低版本从 8 一路涨到 25。
>
> 这些东西，靠手写模板记，记不全。而且记错了不会当场报错，要等用户 build 失败才发现。」

**画面**：演示硬报错。

```bash
vinoa init d3 -m 1.8.9 --platform folia -y -o /tmp/y
```

> 「比如 folia 配 1.8.9，这个组合根本不存在。它不会给你生成一个假的工程，也不会偷偷把平台换成 bukkit。
>
> 它直接告诉你：这个组合不存在，1.8.9 能用的平台是 bukkit 和 sponge，folia 能用的版本是这些。」

（指一下屏幕上的可用项列表）

> 「报错里把能用的选项全列出来了。你不用回去翻文档。」

---

## §4 对脚本和 AI 友好（2:45 – 3:45）

**画面**：这一步是给技术观众的，语速可以快一点。

> 「如果你的使用者不是人，是脚本或者 AI，它还有一套机器接口。」

```bash
vinoa init my-plugin -p com.a.b -m 1.21.11 --platform paper -y --dry-run --json
```

> 「`--dry-run` 跑完所有校验、矩阵查询、渲染、自检，但一个文件都不写。它打印一份 JSON，告诉你它打算写哪些文件。」

（指屏幕）

> 「注意 `exit_code` 这个字段。机器只看这个。`0` 成功，`2` 校验失败，`64` 用法错误，`65` 数据错误。跟人类路径用的是同一张表，同一个判定。」

```bash
vinoa schema --json
```

> 「`schema` 是自描述的。它把命令、参数、平台清单、8 个可选模块、21 个错误码全部吐成 JSON。AI 拿到这个就能自己拼命令，不用你写文档教它。」

**这里要强调的一点**：

> 「`--dry-run` 和真实运行走的是同一条路径。预览即实际。它不会出现『预览说生成 A，实际生成 B』这种情况。」

---

## §5 收尾（3:45 – 4:30）

**画面**：回到刚才生成的工程目录，`tree` 或 `ls` 一下。

```bash
cd ~/demo/my-plugin && find . -type f -not -path "./.git/*" | wc -l
```

> 「43 个文件。core 模块放平台无关的抽象，`platforms/<平台>/` 各自实现。checkstyle、单元测试、build CI 默认就带上了。
>
> 落盘是原子的。先写临时目录，最后整体 rename 过去。中途 Ctrl-C，不会给你留半个残缺的工程。」

**最后一句**：

> 「所以它不是模板生成器。模板生成器给你一堆文件，然后你自己去填坑。vinoa 给的是能直接构建的工程，坑在矩阵里已经填平了。
>
> 装它一行命令。」

```bash
curl -fsSL https://raw.githubusercontent.com/NmouZh/vinoa/main/scripts/install.sh | bash
```

> 「跑完这一行，你的下一个插件工程，从敲下第一条命令到 `BUILD SUCCESSFUL`，十秒。」

（画面停在 `BUILD SUCCESSFUL` 上，不要马上切走）

---

## §6 录制备忘

### 该展示什么（按重要性排序）

| 优先级 | 画面 | 为什么重要 |
|---|---|---|
| ★★★ | 勾 `paper` → 预览面板多出 `platforms/bukkit` | 全片唯一的「啊哈」时刻，别的一带而过都行 |
| ★★★ | `./gradlew build` 一次就绿 | 这是产品承诺本身 |
| ★★ | `folia × 1.8.9` 硬报错 + 列出可用项 | 证明「不静默」不是口号 |
| ★★ | `--dry-run --json` 的 `exit_code` 字段 | 技术观众判断这工具是否可编排 |
| ★ | `vinoa versions` 滚过 61 个版本 | 直观的体量感 |
| ★ | 43 个文件的结构 | 证明不是空壳工程 |

### 别做的事

- **别念帮助文档**。参数表不用念，念了没人记得住。
- **别解释实现**。不要说「用 minijinja 渲染」这种话，观众不关心。
- **别开一堆窗口**。全程一个终端，最多切一次 `versions`。
- **别在 build 的时候说话**。Gradle 输出滚屏时闭嘴，等 `BUILD SUCCESSFUL` 出来再说。

### 数字口径（现场别说错）

| 说法 | 正确值 | 备注 |
|---|---|---|
| MC 版本数 | **61** | 从 `1.8.9` 到 `26.2` |
| 平台数 | **7** | paper / bukkit / velocity / bungeecord / folia / sponge / minestom |
| 可选模块 | **8** | sqlite / bstats / update-check / placeholderapi / gui / spotbugs / coverage / release-ci |
| 错误码 | **21** | `vinoa schema --json` 里数出来的 |
| 生成物文件数 | **43** | `1.21.11 paper` 的实测值，换配置会变 |
| 生成耗时 | **0.3 秒** | 实测，不含 Gradle 构建 |
| 安装方式 | `scripts/install.sh` | Windows 用 `install.ps1` |

### 兜底

如果现场 build 卡住或者失败：

> 「Gradle 第一次跑要下依赖，我提前跑过一次了，可能缓存被清了。」

**不要**现场 debug。直接切到已经构建好的目录展示 `build/libs/` 里的 jar，然后继续。录制可以重来，观众看不到。

---

## §7 15 秒短版（备用）

如果只需要一个短切片发社交平台：

```bash
vinoa init my-plugin -p com.example.myplugin -m 1.21.11 --platform paper -y
cd my-plugin && ./gradlew build
```

**旁白**：

> 「做 Minecraft 插件，开头那半小时全花在抄版本号上。API 坐标、Java 版本、Gradle 版本，抄错一个，等 build 才炸。
>
> 现在一条命令，十秒，`BUILD SUCCESSFUL`。61 个 MC 版本、7 个平台，坐标全在矩阵里实测过。」

（停在 `BUILD SUCCESSFUL`）
