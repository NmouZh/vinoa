# 模板变量契约与渲染管线（草稿 v1）

> 供 ticket [#4 模板变量契约与渲染管线](https://github.com/NmouZh/vinoa/issues/4) 使用。
> 前置：[init-flow](init-flow.md) v8（向导 / flag 面 / 失败语义）、[project-layout](project-layout.md)（生成树）、[version-matrix](version-matrix.md)（数据契约）。
> 事实来源：[rust-scaffold-cli-facts](../../research/rust-scaffold-cli-facts.md)（rust-embed 8.12.0、minijinja 2.24.0）、[paper-plugin-project-shape](../../research/paper-plugin-project-shape.md)、[proxy-plugin-project-shape](../../research/proxy-plugin-project-shape.md)、[legacy-mc-build-feasibility](../../research/legacy-mc-build-feasibility.md)。

## 1. 变量清单

命名规则：**模板变量一律 camelCase**，与 `-c/--config` 的 TOML 键**同名同义、不做别名**（避免两套事实源；若 config ticket 改用 snake_case，则模板清单是唯一映射点）。
`相位`：`G` = generate-time（vinoa 用 minijinja 渲染）、`B` = build-time（Gradle `processResources` 展开）。`只读` = 用户不能直接赋值。
`可入路径`：该变量允许出现在文件名 / 目录名占位符里。

### 1.1 工程标识

| 变量 | 类型 | 默认 / 来源 | 派生规则 | 相位 | 可入路径 |
|---|---|---|---|---|---|
| `projectName` | string | `vinoa init <name>`；省略时为当前目录名 | 校验 `^[A-Za-z][A-Za-z0-9_-]{0,63}$` | G | ✓ |
| `pluginId` / `pluginName` | string 只读 | — | `pluginId = slug(projectName)`（转小写、非 `[a-z0-9-_]` 丢弃；须匹配 Velocity `[a-z][a-z0-9-_]{0,63}`，否则 validate 报错）；`pluginName = PascalCase(projectName)`，如 `my-plugin` → `MyPlugin`，用作类名叶子与 `plugin.yml: name` | G | ✗ / ✓ |
| `packageName` | string | `-p/--package`；默认 `com.example.` + `pluginId` 去掉 `-` | 校验每段 `^[a-z_][a-z0-9_]*$` 且不是 Java 关键字；不合法 → `var.invalid_value` | G | ✗ |
| `packagePath` | string 只读 | — | `packageName.replace('.', '/')` | G | ✓ |
| `mainClass.<platform>` | string 只读 | — | `packageName + "." + <platform> + "." + pluginName`（单平台形式即 ticket 示例 `packageName + "." + pluginName` 去掉平台段） | G | ✗ |
| `commandName` / `permissionNode` | string 只读 | — | `commandName = slug(projectName)`（截断 32 字符、仅 `[a-z0-9_-]`）；`permissionNode = pluginId + ".command"` | G | ✗ |
| `projectVersion` | string | `0.1.0-SNAPSHOT`（常量；**不在向导里问**） | — | G | ✗ |
| `group` / `artifactName` | string 只读 | — | `group = packageName`；`artifactName = projectName` | G | ✗ |

### 1.2 构建

| 变量 | 类型 | 默认 / 来源 | 相位 |
|---|---|---|---|
| `gradleVersion` | string | 矩阵 `[gradle].version`（9.8.0） | G |
| `javaTarget` | map<platform,int> | 矩阵 `java_min`（每模块一份；代理端取 `[platform.<p>].java`） | G |
| `toolchainAutoDownload` | bool | 向导"缺 Java 是否让 Gradle 下载"；`--download-jdk/--no-download-jdk`；`--yes` 默认 `false` | G |
| `qualityToolVersions` | map<string,string> | 矩阵 `[quality]` + `javaTarget`（checkstyle/spotbugs/junit 降级下界） | G |

### 1.3 目标服务端与能力开关

| 变量 | 类型 | 默认 / 来源 | 相位 |
|---|---|---|---|
| `mcVersion` | string | `-m/--mc`；向导先选版本 | G |
| `platforms` | list<string> | `--platform`（可重复）/ 向导多选；**选 `paper` 自动并入 `bukkit`**，去重后按固定优先级排序 | G |
| `metadataFormat` | enum `plugin.yml`\|`paper-plugin.yml` | 向导第 ③ 页；默认 `plugin.yml`（paper-plugin.yml 仍 Experimental）。**二者互斥，绝不并列生成** | G |
| `apiCoordinate.<platform>` | string 只读 | **一律查矩阵** `coordinate_map` / `pinned` / `api_versions`；禁止字符串插值 | G |
| `apiFlavor` | enum `spigot`\|`paper` | bukkit 模块：`mcVersion ≤ 1.16.5` 且用户选 paper 时可为 `paper`（走 `com.destroystokyo.paper`），否则 `spigot` | G |
| `pluginYmlApiVersion` | enum `omit`\|`1.13` | `mcVersion ≥ 1.13` 写 `1.13`（最宽松合法值）；否则整行省略 | G |
| `librariesEnabled` | bool | `mcVersion ≥ 1.16.5`（`libraries:` 自该 API 版本才有） | G |
| `runTask` | enum `run-paper`\|`run-velocity`\|`none` | Paper ≥ 1.8.8 → `run-paper`（`minecraftVersion = mcVersion`）；Velocity → `run-velocity`；1.8.9（无 Paper）→ `none` + README 提示 BuildTools/vanilla | G |
| `paperweightEnabled` / `reobfEnabled` | bool | 前者仅 `≥ 1.17.1` 且用户显式要 NMS（默认 `false`）；后者 = `paperweightEnabled && mcVersion < 26.1`（26.1 起官方移除 reobf） | G |
| `legacyNamespace` / `apiVersionFormat` | bool / enum | 矩阵 `legacy_namespace_until`（`≤ 1.16.5` 用 `com.destroystokyo.paper`）；格式取 `legacy_version_until` / `pinned`：`R0.1-SNAPSHOT`\|`build`\|`pinned` | G |

### 1.4 元数据 / 语言 / 许可 / 可选模块

| 变量 | 类型 | 默认 / 来源 | 说明 |
|---|---|---|---|
| `pluginDescription` | string | `--description`；默认文案**取自语言包**（见 §4.3） | 玩家可见，跟随 `lang` |
| `authors` / `website` | list<string> / string? | `--author`（可重复）/ 无默认 | 空则**整个 `authors:` / `website:` 片段不生成** |
| `exampleEnabled` / `permissionsEnabled` / `qualityEnabled` / `gitInit` | bool | 向导 ④ 页默认勾选；`--no-example/--no-permissions/--no-quality/--no-git` | `gitInit` 只影响 `.gitignore` 与收尾动作 |
| `featureSqlite` / `featureBstats` / `featureUpdateCheck` / `featurePlaceholderApi` / `featureGui` | bool | `--features sqlite,bstats,update-check,placeholderapi,gui`；默认 `false` | 五个独立开关 |
| `bstatsPluginId` | int? | 无默认 | bStats 需要数字 id；缺失策略见 §8 |
| `lang` | enum `zh`\|`en`\|`both` | `--lang`；默认 `zh`（中文系统）| 只影响生成物；与 `--ui-lang` 无关 |
| `license` | string | `Apache-2.0` | 模板集只内嵌 Apache-2.0 全文；其它 SPDX id → `template.license_unsupported` |
| `copyrightYear` | int | `SOURCE_DATE_EPOCH` 的年份，否则当前年 | 可复现性见 §8 |

**推导顺序是固定的**（`pluginId → 校验 → packageName → pluginName → mainClass → 能力开关`），因为每一步都消费前一步；实现上就是 init-flow 附录的阶段 1→3。

## 2. 占位符语法与两个替换相位

### 2.1 generate-time（vinoa 渲染，minijinja 2.24.0）

- 语法 `{{ var }}`、`{% if flag %}…{% endif %}`、`{% for p in platforms %}…{% endfor %}`；**不用 `${}`**，以免与 Gradle 自己的展开撞车。
- **表达式受限**：模板里只允许 `{{ var }}`（纯变量）、分支/循环；不允许算术、函数调用、过滤器链。所有派生量在 Rust 侧算好（§1 的"只读"列）。分支只允许判 **能力开关** `cap_*` / `has_*` / `is_*`（§4.1），不允许直接判 `mcVersion`、`platforms`——这样模板可被静态校验，清单能声明依赖。
- 未定义变量是**硬错误**（`UndefinedBehavior::Strict`，配 `render_named_str` 以便报错带文件名），不允许渲染成空串。
- 需要输出字面 `{{` / `{%`（例如 README 里教人写模板）时用 `{% raw %}…{% endraw %}` 包裹；关自动转义（`set_auto_escape_callback(|_| AutoEscape::None)`），模板是文本拼接、不经过 HTML 语义。

### 2.2 build-time（Gradle `processResources`，SimpleTemplateEngine）

仅 `plugin.yml` 参与，且**只展开一个变量**：

| build-time 变量 | 值来源 | 生成物中的写法 |
|---|---|---|
| `version` | `project.version`（Gradle 单一事实源，CI/发布工具 bump 它） | `version: "${version}"` |

```kotlin
// 生成的 platforms/<p>/build.gradle.kts 片段
tasks.processResources {
    val props = mapOf("version" to project.version)
    inputs.properties(props)              // 不加这行 up-to-date 判定会错
    filesMatching("plugin.yml") { expand(props) }
}
```

规则：`plugin.yml` 里任何字面 `$` 必须写成 `\$`（SimpleTemplateEngine 的转义）。模板清单可用 `build_time_vars = [...]` 声明扩展，**默认集合就是上面这一行**。

### 2.3 相位归属（必须背下来的一条）

**除 `version` 外，§1 所有变量都在 generate-time 落盘定型。**`mcVersion`、坐标、Java 目标、能力开关一律**烘焙进生成物**，生成物构建时不再查询版本矩阵、不再联网。理由：版本矩阵 §4 的可复现硬要求，以及离线构建。

### 2.4 文件 / 目录名占位符

- 路径段允许 `{{ projectName }}`、`{{ pluginName }}`、`{{ packagePath }}`、`{{ platform }}`（**仅这四个**，见 §1 "可入路径"列）；其余变量出现在路径里 → `template.bad_path_var`。渲染后每段必须非空、不含 `/` `\` `:`、不是 `.`/`..`（防路径穿越）。
- 二进制文件（`gradle/wrapper/gradle-wrapper.jar`、图片）**只复制不渲染**，由清单 `render = "copy"` 声明。
- 编码统一 UTF-8；行尾统一 LF，**例外**：`gradlew.bat`、`*.bat` 为 CRLF。Unix 上 `gradlew` 落盘后 `set_permissions(0o755)`（Windows 忽略）。

## 3. 重命名规则与自检

| # | 对象 | 规则 |
|---|---|---|
| R1 | 包目录路径 | `src/main/java/{{packagePath}}/…`（core 与每个平台模块各一份）；平台子包 = `{{packagePath}}/<platform>/`，**不平铺**（project-layout 已定） |
| R2 | 主类文件 + 类名 | 文件 `{{packagePath}}/<platform>/{{pluginName}}.java`，内容 `public final class {{pluginName}}`；两者由同一变量驱动，不允许只改一个 |
| R3 | `settings.gradle.kts` | `rootProject.name = "{{projectName}}"`；`include(...)` 由 `{% for p in platforms %}` 生成 `"platforms:<p>"`（Gradle 用 `:` 不是 `/`） |
| R4 | 跨模块引用 | 模块路径变量 `gradlePath = "platforms:" + platform`，`implementation(project(":core"))`、`project(":{{gradlePath}}")` 全部走它，禁止模板里手写路径 |
| R5 | 跨模块 Java 引用 | 平台模块 `import {{packageName}}.core.*`；core **不 import 任何平台 API**（project-layout 已定），由 A3 的文件扫描一并检查 |
| R6 | 元数据 | `plugin.yml: main: {{mainClass.<platform>}}`；`name: {{pluginName}}`；`version: "${version}"`（必须加引号，防 SnakeYAML 数字强制转换）。Velocity **不生成**任何描述符文件（注解处理器产出 `velocity-plugin.json`） |

**渲染后自检（在 `--dry-run` 里也执行，因为 dry-run 走同一条渲染路径）**：

| ID | 断言 | 失败码 |
|---|---|---|
| A1 | 全部文本文件与文件/目录路径中不再出现 `{{`、`}}`、`{%`、`%}`（二进制文件与 `{% raw %}` 区块内的字面量除外） | `render.leftover_placeholder` |
| A2 | 模板集默认身份串 `com.example`、`my-plugin`、`MyPlugin` 不出现在生成物里，**除非**用户的选择恰好等于它（比对变量实际值） | `render.stale_default` |
| A3 | 每个平台模块的入口类文件存在、`plugin.yml: main` 的 FQCN 能按 `src/main/java/<pkg 转路径>/<类>.java` 定位到；且 core 模块源码不出现任何平台 API 包名 | `render.main_class_mismatch` |
| A4 | 清单声明的变量集合与实际渲染消费的变量集合**互为子集**（双向），捕捉清单漏声明 / 模板里的拼写错 | `template.variable_mismatch` |
| A5 | `settings.gradle.kts` 的 `include` 集合 == `platforms` 目录集合 == 清单 `foreach` 产出集合 | `render.module_set_mismatch` |

A4 是"零残留"的关键：只靠正则扫残留会漏掉**渲染成空串**的变量，双向核对才能兜住。

## 4. 条件文件与条件片段

### 4.1 条件语言只有一套

Rust 侧把原始变量折叠成布尔开关，模板与清单**只认开关**：

```toml
[conditions]
cap_bukkit_module    = "platforms contains 'bukkit'"
cap_libraries        = "mc_version >= 1.16.5"
cap_api_version_line = "mc_version >= 1.13"
cap_legacy_namespace = "mc_version <= 1.16.5"
cap_dual_lang        = "lang == 'both'"
is_paper_metadata    = "metadata_format == 'paper-plugin.yml'"
has_authors          = "len(authors) > 0"
```

支持 `all_of` / `any_of` / `not` + 叶子 `var == '值'`、`var >= 值`、`list contains '值'`；**不引入 Rhai/表达式引擎**，避免 cargo-generate 那种"模板即代码"的信任成本。

### 4.2 条件文件 / 条件片段矩阵

| 维度 | 触发的开关 | 影响文件（`F`）与片段（`S`） |
|---|---|---|
| 平台 | `cap_bukkit_module` | F `platforms/bukkit/**`；S `settings.gradle.kts` 的 include 行、根 README 的模块表、CI 矩阵 |
| 平台 | 每个 `platforms` 元素 | F `platforms/<p>/build.gradle.kts`、`src/main/**`；velocity 模块**没有 resources 目录**（描述符由注解处理器生成） |
| 元数据格式 | `is_paper_metadata` | F `paper-plugin.yml`（替代 `plugin.yml`）；S `api-version`（**paper-plugin.yml 必填且 ≥1.19**）、依赖用 `dependencies.server.*`、命令注册走 `LifecycleEvents.COMMANDS`；`plugin.yml` 路径走 `commands:` 声明 —— 两者**必须二选一** |
| MC 能力 | `cap_api_version_line` / `cap_libraries` | S `api-version: '1.13'` 一行（否则整行省略 = legacy 加载 + 控制台警告）；S `libraries:` 块，为 `false` 时该依赖改走 shading 并在 README 注明 |
| MC 能力 | `cap_legacy_namespace` / `runTask` / `reobfEnabled` | 坐标与仓库全部取自 `apiCoordinate`，模板不分支；F 根 `build.gradle.kts` 的 `run-paper`/`run-velocity` 插件块（`none` → README 给"起测试服"替代说明）；S `paperweight` 的 `reobfJar` 接线（≥26.1 绝不生成） |
| 语言 | `is_lang_zh` / `is_lang_en` / `cap_dual_lang` | F `README.md`、`README.zh-CN.md`；S 源码注释块、`config.yml` 注释、玩家可见消息字符串（见 §4.3） |
| 可选模块 | `cap_sqlite` | F `core/.../storage/SqliteStorage.java`；S core 依赖、`config.yml` 的 db 段 |
| 可选模块 | `cap_bstats` / `cap_update_check` / `cap_placeholderapi` / `cap_gui` | 各自 F 一个类：`metrics/Metrics.java`（+ `bstatsPluginId` 常量）、`update/UpdateChecker.java`、`hook/PlaceholderHook.java`、`gui/Menu.java`；S 启动代码、`config.yml` 开关、PlaceholderAPI 的 `softdepend` + 仓库/坐标 |
| 质量工程 / 示例 / 权限 / git | `cap_quality` / `cap_example` / `cap_permissions` / `cap_git` | F `config/checkstyle/checkstyle.xml`、`.github/workflows/build.yml`、示例命令与监听器、权限常量类、`.gitignore`（`cap_git=false` 时连它也不生成）；S 根 `build.gradle.kts` 的 checkstyle/spotbugs/junit 块（版本取自 `qualityToolVersions`，Java 8 目标自动降到 `≤9.3 / ≤4.8.6 / ≤5.14.4`） |

### 4.3 语言包机制

模板集内 `i18n/zh.toml`、`i18n/en.toml` 存**字符串表**（README 段落、注释块、玩家消息、`pluginDescription` 默认值）。渲染时按 `lang` 选择：

- `zh` → 中文串；`en` → 英文串。`both` → `README.md`（英文）+ `README.zh-CN.md`（中文），源码注释用英文，玩家消息落入 `resources/lang/en.yml` + `lang/zh_CN.yml` 两份、`config.yml` 的 `language: en` 为默认。
- 语言包缺 key → `template.i18n_missing_key`（硬错误，不允许回退成空串）。

## 5. 模板清单：`vinoa-template.toml`

每个模板集根目录一份，**随模板集一起被 rust-embed 内嵌**；schema 版本化、字段扁平、无继承，`serde(deny_unknown_fields)` 校验（未知键即拒，这同时是 §6 信任规则的一部分）。

| 顶层键 | 必填 | 语义 |
|---|---|---|
| `schema` / `id` / `template_version` | ✓ | schema 版本；模板集标识；模板集自身版本 |
| `min_cli_version` | ✓ | CLI 更低 → `template.version_too_old` |
| `[variables.<name>]` | ✓ | `type` / `required` / `default` / `values` / `pattern` / `derived` / `readonly` / `description` |
| `[conditions]` | 按需 | 开关名 → §4.1 条件表达式 |
| `[dependencies]` | 按需 | 模板集自持的三方坐标（不在版本矩阵；见 §8 风险 3） |
| `[[files]]` | ✓ | `template` / `target` / `render`（`template`\|`copy`）/ `when` / `foreach` + `paths` / `build_time_vars` |
| `[[assertions]]` | 按需 | `target` + `contains[]`（可引用变量），渲染后比对，属 A1–A3 的内容级同类检查 |

```toml
schema           = 1
id               = "vinoa/paper-gradle-v1"
template_version = "1.0.0"
min_cli_version  = "0.1.0"          # CLI 更低 → template.version_too_old
kind             = "gradle-multimodule"

[variables.project_name]             # 键 = 变量名的 snake_case 落盘形式，见下注
type        = "string"
required    = true
pattern     = "^[A-Za-z][A-Za-z0-9_-]{0,63}$"
description = "工程名；进 settings.gradle.kts 的 rootProject.name"
# 其余变量同构声明；只读派生量用 derived = "..." + readonly = true

[conditions]
cap_libraries = "mc_version >= 1.16.5"
cap_sqlite    = "feature_sqlite"
is_lang_zh    = "lang == 'zh'"
# [dependencies] 形如 placeholderapi = "me.clip:placeholderapi:2.11.6"（模板集自持，不在版本矩阵）

[[files]]
template = "platforms/_p_/build.gradle.kts.jinja"
target   = "platforms/{{platform}}/build.gradle.kts"
foreach  = "platforms"                 # 逐元素渲染，路径段 _p_ 被平台键替换；foreach 即条件
paths    = ["platform", "gradle_path"]

[[files]]
template = "platforms/paper/src/main/resources/plugin.yml.jinja"
target   = "platforms/paper/src/main/resources/plugin.yml"
when     = "platforms contains 'paper'"
build_time_vars = ["version"]          # 仅该文件参与 Gradle expand

[[files]]
template = "bin/gradle-wrapper.jar"
target   = "gradle/wrapper/gradle-wrapper.jar"
render   = "copy"

[[assertions]]
target   = "platforms/paper/src/main/resources/plugin.yml"
contains = ["name: {{pluginName}}", "main: {{mainClass.paper}}", 'version: "${version}"']
```

`foreach` + `paths` 声明循环渲染时路径段可用的变量；`when` 只接受 §4.1 的条件表达式。清单随 `--json` 原样导出（见 §7）：**`vinoa-template.toml` 是 agent 理解模板集的唯一入口**，不需要读模板正文。

> 注：上例 `[variables.*]` 用 snake_case 只为 TOML 书写习惯；**契约上变量名是 camelCase**（§1），单一映射函数 `snake↔camel`，不允许另起别名。见 §8 风险 9。

## 6. 内嵌 vs 外部模板：优先级与信任

**优先级（不叠加、不合并）**：`--template <本地路径>` > `--template <git-url#ref>` > 内嵌（默认，无 `--template`）。三者只取其一：给定外部模板时**完全不读内嵌模板**。内嵌树由 rust-embed 提供（release 烘焙进二进制；dev 直读磁盘，CI 加 `debug-embed` 以覆盖内嵌路径）。只支持"整套替换"，v1 **不做覆盖层/文件级合并**——否则"哪个文件赢"无法静态校验，A4/A5 也失去意义。

**外部模板的硬规则**：

1. 根目录必须有 `vinoa-template.toml` 且 `schema` 被本 CLI 认识。
2. **模板是纯数据，永不执行**：清单 schema 里没有 `hooks`/`scripts`/`commands` 字段，未知键报错；渲染过程不调用 `git`、不调用 Gradle、不做网络请求。
3. `target` / `template` 必须是相对路径、无 `..`、无绝对路径、无符号链接逃逸（解析后须落在模板根内）。
4. git URL：resolve 到**具体 commit SHA** 并回显；`--json` 记录 `{kind:"git", url, ref, commit}`；可变 ref（分支/tag）额外警告一句。不拉 submodule，模板内的 `.git` 目录忽略。
5. 渲染前限额：文件数 ≤ 5000、解压后总量 ≤ 32 MiB，超限 `template.too_large`。
6. `--offline` + git URL → 直接报错（**不回退内嵌**：静默换一套模板比失败更糟）；用 git 模板时确认摘要必须打印来源 + commit + 文件数，非 `--yes` 需二次确认。

## 7. 错误如何呈现

**人类可读（stderr）**：`错误[码] 一句话` + `模板文件:行:列` + 输出路径 + `hint`。例：

```
错误[render.leftover_placeholder] 渲染后仍有未替换占位符
  templates/platforms/_p_/build.gradle.kts.jinja:37:18
  → platforms/paper/build.gradle.kts
  hint: 变量 cap_placeholderapi 未在 vinoa-template.toml 的 [conditions] 中声明
```

**`--json`（stdout，恰好一个 JSON 文档；人读文本走 stderr，便于管道）**：

```json
{
  "schema": 1, "ok": false, "command": "init", "phase": "render",
  "template": {"id": "vinoa/paper-gradle-v1", "version": "1.0.0", "source": {"kind": "embedded"}},
  "errors": [
    {
      "code": "template.undefined_variable", "phase": "render", "severity": "error",
      "template_file": "platforms/_p_/build.gradle.kts.jinja",
      "target": "platforms/paper/build.gradle.kts",
      "line": 37, "column": 18, "variable": "cap_placeholderapi",
      "message": "模板引用了未定义的变量",
      "hint": "在 [conditions] 中声明 cap_placeholderapi"
    }
  ]
}
```

契约要点：

- 每个错误必有**稳定 ASCII 码**（`命名空间.小写_下划线`）与 `phase` ∈ `input|validate|matrix|precheck|plan|render|verify|write|post`；`line`/`column` 为 1 基，**只有错误发生在文件内时才有**；`message` 跟随 `--ui-lang`，`code` 不跟随语言。
- **校验阶段聚合报错**（一次列全所有非法字段）；**渲染阶段首错即停**（后续多半是级联噪声），但不写盘。退出码沿用 init-flow 附录的阶段划分，数字映射（0 / 64 usage / 65 变量非法 / 73 不能写 / 74 写中失败 / 78 模板缺失或损坏）**尚未在 init-flow 定稿**。
- `--dry-run --json` 成功时给 `files[]`（`target`、`render`、`bytes`、`sha256`）与被条件排除的 `skipped[]`（带 `when` 原因）——这是"预览即实际"的可核对形式。

| 码 | 触发 |
|---|---|
| `template.not_found` / `template.manifest_invalid` / `template.manifest_unknown_key` / `template.version_too_old` | 清单缺失、TOML 语法错（带行列）、未知键、`min_cli_version` 过高 |
| `template.undefined_variable` / `template.variable_mismatch` / `template.bad_path_var` / `template.bad_target` | 未定义变量、清单与模板声明不对称（A4）、路径用了不可入路径的变量、目标路径非法或逃逸 |
| `template.i18n_missing_key` / `template.license_unsupported` | 语言包缺 key / 非 Apache-2.0 且模板集无该全文 |
| `var.invalid_value` | 名称、包名、`mcVersion`×平台、`bstatsPluginId` 等校验失败 |
| `render.*`（`syntax_error` / `leftover_placeholder` / `stale_default` / `main_class_mismatch` / `module_set_mismatch` / `assertion_failed`） | 渲染与自检（§3 的 A1–A5） |
| `write.exists` / `write.io` | 目标目录非空 / 落盘失败（原子回滚，无残留） |

## 待定 / 风险

1. **`lang = both` 的注释形态**：本稿定为"英文注释 + 双 README + 双语言包"，但 init-flow #13 只说"注释与 README 跟随"。需要用户拍一次，或先做 prototype 看双语文档读起来如何。
2. **`bstatsPluginId` 没有来源**：bStats 必须数字 id，向导 ④ 页只有一个复选框，flag 面也没有对应参数。当前契约只能"勾了 bstats 却缺 id → 报错"或"向导勾选后追问"，后者是 init-flow 未覆盖的增量，需回写 init-flow。
3. **三方坐标的版本事实源**：placeholderapi / bstats / sqlite-jdbc / GUI 库不在版本矩阵里，本稿放在清单 `[dependencies]`（模板集自持），代价是多一个版本来源。是否并入矩阵需与 #9 对齐。
4. **paper 与 bukkit 是否合并模块**（project-layout 仍未定）：影响 `plugin.yml` 份数与入口类数量。本契约对两种形态都成立，但 A5 的"模块集合"断言要等该决定才能定稿。
5. **是否生成溯源文件**（如 `.vinoa.toml` 记录模板 id/commit/生成时间）：会改变 project-layout 已画出的树，且"生成时间"与可复现性冲突。倾向只在 `--json` 里给，不落盘。与之相关的 `copyrightYear`（当前年 vs `SOURCE_DATE_EPOCH`）同样影响 version-matrix §4 的"同一输入两次运行结果完全相同"硬要求，是否默认冻结年份需拍板；模板集也只有 `min_cli_version` 下界，外部模板比 CLI 新时是否要拒（`max_cli_version`）未定。
6. **外部 git 模板的信任边界**：当前是"无 hook + 记录 commit + 尺寸上限 + 二次确认"。是否需要签名校验（cosign/sigstore）、是否允许私有仓库凭据透传，未定。
7. **跨平台实测缺口**：`{{packagePath}}` 作为真实目录名三系统都合法，但 Windows 大小写不敏感 + `core.protectNTFS` 与 git 的交互要跑一次真机；CRLF / `gradlew` 权限位只在 Linux 验证过（手头无 Windows）。`processResources` 的 `\$` 转义规则也只来自 SimpleTemplateEngine 的通用行为，含 `$` 的 `pluginDescription`/`authors` 需一条实测用例。
8. **质量工具降级表**：checkstyle/spotbugs/junit 的 Java 8 上界来自 research 的字节码探测，Gradle 9.8 的内置默认版本（10.24.0）未在 9.8 上实跑；生成物 `--verify` 的第一次真机构建才是可信验证。
9. **minijinja 细节待验**：`{% raw %}` 可用性、`UndefinedBehavior::Strict` 与 `render_named_str` 组合下的错误位置精度、空列表 `for` 行为 —— 需要一个 20 行最小实验确认后再写死 §2.1 的措辞。变量名与 config 键同名的假设（见 §5 注）也随 config schema ticket 复核。
