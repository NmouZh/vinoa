# ADR 目录约定与索引

> **作用**：本文件只定义**编号与状态约定**，并索引 `docs/adr/` 下的决策记录。它**不含**任何一条 ADR 的决策内容——具体决策写在各 ADR 文件里。
> **上下文**：本仓库是 single-context，ADR 放在仓库根的 `docs/adr/`（见 [`docs/agents/domain.md`](../agents/domain.md) §File structure）。术语用 [`CONTEXT.md`](../../CONTEXT.md) 的词汇。

## 何时写 ADR

写 ADR 的条件是"这个决定**以后会被重新提起**"，并且它满足至少一条：

- 排除了其它可行方案（不写理由，下个会话会把被否的方案重新提出来）；
- 约束了后续实现（比如"谁负责某件事"、"某个能力我们不做"）；
- 与既有文档的默认口径不同，或**取代**了此前的某条 ADR。

不写 ADR 的情况：可以从代码直接读出的实现细节、纯数据更新（例如版本矩阵数据上移）、以及**尚未拍板**的待定项（那些留在 spec 附录"已知风险"或草稿的"待定"节）。

## 编号约定

| 项 | 约定 |
|---|---|
| 格式 | 4 位零填充序号 + 短横线 + kebab-case 标题，扩展名 `.md`：`NNNN-<slug>.md` |
| 序号 | 从 `0001` 起**严格递增**，不复用、不重排。删掉一条 ADR 时**保留空号**，不把后面的号往前挪 |
| slug | 英文、kebab-case、只描述**被决定的对象**（不是结论）：`render-stack`、`exit-code-table`。避免 `use-ratatui` 这种把结论写进文件名的做法 |
| 一个文件一条决策 | 不写"ADR-0007 附带也决定了 A、B、C"；附带决定另开号，并在两条之间互相链接 |
| 不可变 | ADR 一旦置为 `accepted` 就**不改正文**。要改口径就新开一条，把旧的置为 `superseded by ADR-NNNN` |
| 允许的编辑 | 只有两处可以原地改：`status` 字段（状态流转），以及文末的"Superseded by / Supersedes"链接 |

## 文件名与状态

每份 ADR 的**第一段**（正文之前）给出元信息，固定四行：

```markdown
# ADR-0001 渲染方案选型

- **状态**：proposed
- **日期**：YYYY-MM-DD
- **相关**：issue #21 · supersedes: — · superseded by: —
- **依赖**：ADR-XXXX（若有）
```

状态取值**只有这五个**，大小写与拼写固定：

| 状态 | 含义 | 允许的流转 |
|---|---|---|
| `proposed` | 已写出，等裁定。可以改正文 | → `accepted` / `rejected` |
| `accepted` | 已拍板，**正文冻结**。实现必须遵守 | → `superseded` / `deprecated` |
| `rejected` | 被明确否掉。**正文冻结**——它存在的价值就是让下个会话不再重提 | → `superseded`（若后来又被采纳，开新号） |
| `superseded` | 被更新的 ADR 取代。必须在 `superseded by:` 给出新号 | 终态 |
| `deprecated` | 不再适用，但没有替代品（例如被决定的对象已移出范围） | 终态 |

## 正文结构

固定四节，按此顺序；**Decision 必须唯一**（一条 ADR 不能给两个并列的结论）：

```markdown
## Context
现状是什么、为什么现在要决定。带证据（实测输出 / 源码路径 / 文件行数），不写"业界普遍认为"。

## Decision
唯一的结论。写清**谁负责什么**——尤其是责任边界（例如"某能力由 vinoa 自己保证，不依赖上游库提供"）。

## Consequences
分正面与负面两栏。负面必须写**具体代价**（要重写哪些文件、放弃哪些能力、多出多少维护面），不写"可能有一些影响"。

## Alternatives considered
每个被否方案 + **可复述的理由**。理由不能是"不够好"，要是代价（占用多少列、多编译几个版本、失去哪个能力）。
```

## 与其它文档的关系

- **spec 是正文，ADR 是决策记录**：ADR 不重复 spec 的契约条款，只记录"为什么是这个契约"。ADR 与 [`docs/spec/vinoa-cli.md`](../spec/vinoa-cli.md) 冲突时，以 spec 正文为准，并把冲突显式记进 ADR 的 Context。
- **与草稿裁定的关系**：[`docs/spec/drafts/rulings.md`](../spec/drafts/rulings.md) 是合成 spec 之前的裁定层。凡已被 rulings 覆盖、且已经并入 spec 正文的取舍，**不再另开 ADR**；rulings 里未并入 spec 的长期决策才升格为 ADR。
- **发现冲突时的写法**：任何产出（issue、提案、PR）若与既有 ADR 矛盾，必须显式指出，不要静默覆盖。格式照 [`docs/agents/domain.md`](../agents/domain.md) §Flag ADR conflicts：

  > _Contradicts ADR-0007 (event-sourced orders), but worth reopening because…_

## 索引

| 编号 | 标题 | 状态 | 相关 | 产出 |
|---|---|---|---|---|
| [ADR-0001](0001-render-stack.md) | 渲染方案选型（`inquire` 之上加样式层 vs `ratatui` + `crossterm` 全屏接管） | proposed | issue #21 | 本次 effort（界面 #19–#28）产出 |
| — | *（下一条 ADR 从 0002 起）* | — | — | — |

> **索引维护规则**：新增 ADR 时同时更新本表——编号、标题、状态、相关 issue、产出方。表中条目的状态必须与 ADR 文件的 `status` 字段一致；**索引条目不先于 ADR 文件存在而声称内容**。
