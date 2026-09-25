# 版本矩阵（草稿 v1）

> 供 ticket [#9 版本矩阵契约](https://github.com/NmouZh/vinoa/issues/9) 使用。
> 数据来源：[version-matrix-sources.md](../../research/version-matrix-sources.md)（§7 有 66 行完整表，全部对官方端点实测于 2026-09-25）。

## 1. 它解决什么

用户只选 **MC 版本 + 平台**（见 [init-flow §1](init-flow.md)），矩阵负责把这选择翻译成能构建的工程参数：平台坐标、Java 目标、Gradle 与插件版本、元数据格式能力，以及**哪些组合根本不成立**。

## 2. 数据模型：按平台白名单，不做范围推导

每个平台一份记录，**只有明确存在的版本才进表**。范围推导必然出错——Paper 缺 `1.8.9` / `1.20.3` / `1.21.2` / 裸 `26.1`，Folia 最早 `1.19.4`，Minestom 只有 4 个版本，Sponge 有 6 个缺口。

```toml
# templates/version-matrix.toml（编译进二进制）
schema       = 1
generated_at = "2026-09-25"

[java]                       # java_min 必须 / java_recommended 只用于提示
"1.8.9"   = { min = 8,  recommended = 8  }
"1.12.2"  = { min = 8,  recommended = 11 }
"1.16.5"  = { min = 8,  recommended = 16 }
"1.17.1"  = { min = 16, recommended = 17 }
"1.20.6"  = { min = 21, recommended = 21 }
"1.21.11" = { min = 21, recommended = 21 }
"26.2"    = { min = 25, recommended = 25 }
# …完整 66 行见研究 §7

[platform.bukkit]            # 1.8.x 的唯一选择
api            = "org.spigotmc:spigot-api"
versions       = ["1.8.8", "1.8.9", "1.12.2", "1.16.5", "1.21.11", "26.2", "…"]
coordinate_map = { "1.8.9" = "org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT" }

[platform.paper]
api                     = "io.papermc.paper:paper-api"
versions                = ["1.9.4", "1.10.2", "1.11.2", "1.12.2", "…", "1.21.11", "26.1.1", "26.1.2", "26.2", "26.3"]
legacy_namespace_until  = "1.16.5"        # com.destroystokyo → io.papermc 从 1.17 起
legacy_version_until    = "1.21.11"       # {MC}-R0.1-SNAPSHOT → {MC}.build.+ 从 26.1 起
pinned                  = { "26.2" = "io.papermc.paper:paper-api:26.2.build.129-stable" }

[platform.velocity]          # 代理端：协议范围，不锁 MC 版本
model          = "protocol"
protocol_min   = "1.7.2"
protocol_max   = { "4.2.0" = "26.2" }
java           = 25

[platform.bungeecord]        # 代理端：只有大颗粒版本
model    = "major-grain"
versions = ["1.8", "1.16-R0.5", "1.21-R0.4", "26.1-R0.1-SNAPSHOT"]
java     = 8                 # release 线；快照线已到 17

[platform.folia]
api         = "io.papermc.paper:paper-api"
min_version = "1.19.4"

[platform.minestom]
versions = ["1.21.11", "26.1.1", "26.1.2", "26.2"]   # 就这么多
java     = 25

[platform.sponge]
api_versions = { "1.8.9" = "4.2.0", "1.9.4" = "5.0.0", "1.12.2" = "7.4.8", "1.16.5" = "8.2.1", "1.21.11" = "18.0.0", "26.2" = "20.0.0" }

[gradle]
version    = "9.8.0"
run_on_jvm = "17-27"
# 插件地板取自各自 .module 的 org.gradle.plugin.api-version，不抄文档
shadow      = { version = "9.6.1",          gradle_min = "9.2.0", java = 17 }
run_paper   = { version = "3.1.0",          gradle_min = "9.7.0", java = 17 }
paperweight = { version = "2.0.0-beta.24",  gradle_min = "9.7.1", java = 21 }

[quality]                    # 老版本模块的降级下界
checkstyle_java8_max = "9.3"
spotbugs_java8_max   = "4.8.6"
junit_java8_max      = "5.14.4"
junit6_java          = 17
```

## 3. 查询语义

- 输入 `(mc, platform)` → 命中 / 未命中。未命中时返回**该 mc 的可用平台**与**该平台的可用版本**，供交互过滤与报错文案使用。
- Java 取 `java_min`；`java_recommended` 只用于提示文案。
- 坐标**一律查 `coordinate_map`**，禁止字符串拼接——`1.8.9 → spigot-api:1.8.8` 这类映射必须显式存在。
- 代理端（velocity / bungeecord）不参与 MC 版本过滤，恒可选。

## 4. 离线 / 联网语义

- **默认离线**：内置矩阵即唯一事实来源；同一输入两次运行必须给出完全相同的结果（可复现是硬要求）。
- `vinoa versions --refresh`：从 `fill.papermc.io/v3` 拉取校验后写入用户缓存目录（不写进生成的工程），后续 init 使用。
- `--online`：单次使用联网结果（仍落缓存）；离线时回退内置矩阵并警告。
- 请求必须带非通用 `User-Agent`；`?channel=STABLE` 只对 `/builds` 有效，`/builds/latest` 需要客户端自己过滤。
- 数据带 `generated_at`；`--json` 输出里标注来源（内置 / 缓存 / 在线）。

## 5. 裁剪规则（已定，见 [#9 评论](https://github.com/NmouZh/vinoa/issues/9)）

- 交互：只列支持项，用户选不到不成立的组合。
- 非交互：硬报错 + 列出可用项，绝不静默裁剪。

## 6. 待下一轮

- 断点粒度：保留全部 66 行（体积很小、不会漏）还是压成断点区间 + 例外表。**倾向保留全量**。
- 未来 `build` 命令复用本表的方式（本 effort 只留口）。
