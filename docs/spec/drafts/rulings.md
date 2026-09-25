# Lead 裁定层（2026-09-25）

> 六份草稿的"待定 / 风险"节里，凡是属于实现取舍的，由 Lead 在此裁定；合成规格时以本文件覆盖草稿中的对应待定项。
> 用户已拍板的产品决策不在本文件内（见 [init-flow.md](init-flow.md) §4 的 14 条）。

## A. 来自 [template-contract.md](template-contract.md) 的 10 项

| # | 待定项 | 裁定 |
|---|---|---|
| A1 | `lang = both` 的注释形态 | **注释中英双语**（中文在前、英文在后），README 双语，语言包双语。依据：用户在早期明确说过"注释中英文"。代码内不做逐行双语以外的花样（不做"仅文件头双语"这种模糊规则） |
| A2 | `bstatsPluginId` 无来源 | 勾选 bStats 时**向导追问一格**数字 id；非交互路径用 `--bstats-id <id>`，缺失则硬报错并说明去哪拿（bstats.org 的 plugin id）。**绝不生成假 id** |
| A3 | 三方坐标（PlaceholderAPI / bStats / sqlite-jdbc / GUI 库）的版本事实源 | **并入版本矩阵**，新增 `[thirdparty]` 一节。理由：矩阵已经是"版本事实的唯一来源"，再多一处必然漂移；且这些库本身也有 MC 版本兼容差异 |
| A4 | paper 与 bukkit 是否合并 | **已定**：两个模块，勾 `paper` 自动带上 `platforms/bukkit`（用户拍板）。`plugin.yml` 每模块各一份，入口类每模块各一个 |
| A5 | 是否生成溯源文件（`.vinoa.toml`） | **不落盘**。溯源信息只出现在 `--json` 输出与首次提交的 message 里——落盘会引入"生成时间"，直接破坏"同一输入 → 同一文件树"的可复现硬要求 |
| A6 | `copyrightYear` 与可复现性冲突 | **默认不写任何年份**：Apache-2.0 的 LICENSE 文本本身不需要年份；源文件版权头默认不生成。将来若确需年份，走 `SOURCE_DATE_EPOCH`，且该差异必须在 `--json` 里标注 |
| A7 | 外部模板 `max_cli_version` | **只校验 `min_cli_version`，不设上限**。上限只会挡住合理用法，收益为零 |
| A8 | 外部 git 模板的信任边界 | 保持现有四项：无 hook 执行、记录 commit、尺寸上限、二次确认。**v1 不做签名校验**（cosign/sigstore），**不做私有仓库凭据透传**——后者要在文档里明说 |
| A9 | 跨平台实测缺口（Windows 路径、CRLF、`gradlew` 权限位、`$` 转义） | 记为**已知缺口**，写入 acceptance 的风险节；实现阶段补 Windows 真机用例（生成路径 + CRLF + `gradlew.bat` + `$` 转义） |
| A10 | minijinja 细节（`{% raw %}`、`UndefinedBehavior::Strict`、错误位置精度） | 实现阶段先用 20 行最小实验确认，再写死 spec 措辞；草稿措辞保留但标注"待实验确认"。质量工具降级表同理——以生成物的**首次真机构建**为准，草稿表标注 TODO 待实测替换 |

## B. 来自 [acceptance.md](acceptance.md) / [template-content.md](template-content.md) 的 6 项

| # | 待定项 | 裁定 |
|---|---|---|
| B1 | 1.16.5 的 toolchain 写 8 还是 16 | **写 `java_min`**：旧版本模块统一 = 该 MC 版本的 `java_min`（1.16.5 → 8）。`java_recommended` 只用于向导提示。避开 Java 16 这种非 LTS、CI 难装的坑 |
| B2 | `--offline` 与依赖 warm-up 的矛盾 | `--verify` 在**用户机器**默认离线；**CI 的保证行允许联网**（先 warm-up 再构建）。"可复现"只约束 CLI 的**生成阶段**，不约束构建阶段 |
| B3 | folia / sponge / minestom 是否配 A 级 | **降为 B 级**（它们已标 experimental，承诺级别应一致）。A 级只留 `paper` / `bukkit` / `velocity` / `bungeecord`；B 级验证"渲染 + 依赖解析"，不跑完整构建 |
| B4 | 退出码表演进 | `2 = VERIFY_FAILED` 冻结；未来 `build` 子命令若需重排属于另一个 effort（当前 Out of scope） |
| B5 | `config.yml` 归属 | **每个平台模块各一份**；共享的是 `core/` 里的**配置模型**。各平台资源加载机制不同，共享单个资源文件会打架 |
| B6 | `integrationTest` source set | **只在勾了服务端平台（`paper` / `folia`）时生成**，且必须至少有一个真跑的测试；否则不生成空源集 |

## C. 全局裁定（合成规格时必须遵守）

1. **单一事实来源**：平台坐标、Java 目标、Gradle/插件地板、三方库版本全部来自版本矩阵；任何地方出现字符串拼接坐标即为缺陷。
2. **可复现定义**：同一 CLI 版本 + 同一输入 → **字节相同**的文件树（时间戳、年份、随机种子、机器路径一律不得进入产物）。
3. **不静默**原则（贯穿三处）：组合不存在 → 硬报错列可用项；缺 Java → 询问；缺 bStats id → 硬报错。
4. **clean-room**：参考两个社区模板的结构与工程纪律，**不复制代码正文**；vinoa 与生成物均 Apache-2.0；README 注明参考来源。
5. **措辞**：规格正文用中文；代码、CLI 参数、文件路径用英文；生成物的语言由向导决定（见 init-flow §4.13）。
