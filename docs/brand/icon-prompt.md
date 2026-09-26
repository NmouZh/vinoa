# vinoa 图标生图提示词（草稿，不是规格）

> **状态**：提示词草稿。本文不是 spec，不进入 [`docs/spec/vinoa-cli.md`](../spec/vinoa-cli.md) 的权威链，也不改动 [`docs/spec/ui/`](../spec/ui/) 下任何锁定文档。
> **用途**：交给生图 AI（GPT Images / Midjourney / Flux / 即梦 / 混元）产出应用图标。
> **约束来源**：调色板与视觉纪律取自 [`docs/spec/ui/tokens.md`](../spec/ui/tokens.md) §3.1 与 [`direction.md`](../spec/ui/direction.md) §1、§3。

---

## 0. 先读这段（决定图标长什么样）

vinoa 的视觉纪律是**颜色只承载语义、不做装饰**，全屏只有一个强调位；[`direction.md`](../spec/ui/direction.md) §3 还明确否掉了打字机、彩虹渐变、ASCII logo 庆典、完成庆祝动画。

所以图标不能是常见的 AI 应用图标套路。**以下四项出现即废稿**：

| 废稿特征 | 为什么 |
|---|---|
| 3D 等距立方体 / 玻璃拟态 / 霓虹发光 | 装饰而非语义，与 §3 被否项同类 |
| 彩虹渐变 / 多色光晕 | 违反「一个强调位」 |
| 吉祥物 / 拟人角色 | 本项目没有吉祥物 |
| 图标里的文字、字母、版本号 | 生图模型必然把文字画错；且 vinoa 是 CLI，图标不承担字号可读性 |

图标要同时通过两场考试，第二场是**本项目自己的降级契约**（[`tokens.md`](../spec/ui/tokens.md) §3.5、[`degradation.md`](../spec/ui/degradation.md)）：

1. **彩色态**：在 `#1e1e2e` 深底上成立。
2. **无色态**：去掉全部颜色、只留单色剪影，**仍然可辨**。这正是 vinoa 界面「无色靠字形 + 缩进 + 列位」的同一条原则。

---

## 1. 调色板（唯一来源，不得增色）

取自 [`tokens.md`](../spec/ui/tokens.md) §3.1 真彩档，Catppuccin Mocha 角色。**最多用 4 个**：

| 角色 | hex | 语义（必须与 tokens.md 一致） | 图标里用在哪 |
|---|---|---|---|
| 底色 | `#1e1e2e` | 画布 | 图标背景 |
| 结构 | `#585b70` | 边框 / 分栏线 | 圆角外框、树形连线 |
| 主体 | `#cdd6f4` | 主要文字 / 主要形态 | 主体线条与节点 |
| **唯一强调** | `#74c7ec` | **当前位置 / 当前选中** | 只有一个节点填此色 |
| 成功（可选） | `#a6e3a1` | ok / 校验通过 | 仅当要表达「build 绿」时用，且只出现一处 |

强调色**只能出现一次**。出现两处就废稿，回到 §0 重来。

---

## 2. 主提示词（英文，直接粘贴）

生图模型对英文结构描述更稳。**负面词已含在末尾**。

```text
A flat vector application icon on a square 1:1 canvas. Dark, terminal-native, engineering-grade.

SUBJECT: A single minimalist terminal window drawn as a rounded square frame. Inside it, a small file-tree branch grows from a short vertical trunk on the left: one horizontal branch going right, ending in a small square node, and one branch going down-right, ending in a small square node. The lower-right node is the only element filled with a bright color. A small, simple checkmark sits just outside the frame's lower-right corner, barely overlapping it.

STYLE: Flat geometric vector. Uniform 2px stroke weight everywhere, no stroke weight variation. No gradients, no glow, no bevel, no drop shadow, no texture, no noise, no 3D. Rounded corners on the outer frame, radius about 22% of the canvas. Generous negative space, calm and quiet composition, optically centered. Reads clearly as a silhouette at 32x32 pixels.

PALETTE, exactly five values, nothing else:
- background: #1e1e2e
- frame and tree connector lines: #585b70
- trunk and upper node: #cdd6f4
- the single lower-right node: #74c7ec
- the checkmark: #a6e3a1

The blue node is the ONLY blue element in the entire image. The green checkmark is the ONLY green element.

NEGATIVE: no text, no letters, no words, no numbers, no version strings, no monospace glyphs, no mascot, no character, no face, no hands, no 3D isometric cube, no glassmorphism, no neon glow, no lens flare, no rainbow gradient, no multi-color gradient, no confetti, no celebration, no sparkles, no drop shadow, no busy detail, no photorealism, no watermark, no signature.
```

---

## 3. 主提示词（中文版，给中文生图工具）

```text
一个方形 1:1 的扁平矢量应用图标。深色、终端原生、工程感。

主体：一个极简终端窗口，画成圆角方框。框内从左侧一小段竖直主干长出一条文件树分支，向右伸出一根横枝、末端是一个小方块节点，再向下右伸出一根枝、末端也是一个小方块节点。右下那个节点是整张图里唯一被填成亮色的元素。方框右下角外侧有一个小小的对勾，只轻微压住边框。

风格：扁平几何矢量。所有线条统一 2px 粗细，不许有粗细变化。不要渐变、不要发光、不要浮雕、不要投影、不要纹理、不要噪点、不要 3D。外框圆角，圆角半径约为画布的 22%。留白充足，构图安静克制，视觉居中。缩到 32×32 像素时剪影仍然清晰可辨。

配色，严格只用这五个值，不得增加：
- 背景 #1e1e2e
- 外框与树形连线 #585b70
- 主干与上方节点 #cdd6f4
- 唯一的右下节点 #74c7ec
- 对勾 #a6e3a1

蓝色节点是全图唯一的蓝色元素，绿色对勾是全图唯一的绿色元素。

禁止：文字、字母、单词、数字、版本号、等宽字符、吉祥物、拟人角色、人脸、手、3D 等距立方体、玻璃拟态、霓虹发光、镜头光晕、彩虹渐变、多色渐变、彩带、庆祝效果、闪光点、投影、繁复细节、照片写实、水印、签名。
```

---

## 4. 备选方向（主图不满意时换，别混着用）

### V2 字母形

把树形主干与分支画成几何化的字母 v，右侧节点即 v 的收笔。**不要**让模型直接写字母，而是描述「两条等宽直线在底部汇成一点」。风险：模型容易把 v 画成常规字标，需要多抽几张。

### V3 无色优先

先按单色画完（`#cdd6f4` 线条 + `#1e1e2e` 底），确认剪影成立之后，**只**给一个节点上强调色。这个顺序能保证无色态一定通过。想省事就用这条。

### V4 版本跨度

一根水平轴，左端一个小方块、右端一个小方块，中间由细线连成一条跨度。取自项目的版本矩阵事实（MC `1.8.9` → `26.2`，Java 8 → Java 25）。抽象度最高，风险也最大，只适合做次级图标。

---

## 5. 负面提示词（单独填负面框时用）

```text
text, letters, words, numbers, typography, logo text, watermark, signature,
3D, isometric, cube, glassmorphism, frosted glass, bevel, emboss, drop shadow,
neon, glow, bloom, lens flare, light rays, rainbow, multi-color gradient,
mesh gradient, confetti, sparkles, celebration, party,
mascot, character, face, hands, robot, animal,
photorealistic, cluttered, busy, noisy, low contrast, blurry, soft focus,
colorful, saturated, purple, pink, orange, yellow, red
```

---

## 6. 参数与产出

| 项 | 值 |
|---|---|
| 比例 | 1:1 |
| 主尺寸 | 1024×1024 |
| 导出 | 1024 / 512 / 256 / 128 / 64 / 32 / 16 PNG + `icon.ico`（Windows）+ `icon.icns`（macOS） |
| 源文件 | 矢量为准。生图产出是位图，**需要描摹成 SVG 后再定稿**，否则小尺寸会糊 |
| 描摹技巧 | 生图时加 `flat vector, solid fills, hard edges, no anti-aliasing softness`；描摹后在 16px 下逐张目检 |
| 生成批次 | 一次 4 张，从剪影是否成立开始筛，再筛配色是否越界 |

---

## 7. 验收清单（每张图逐条打勾）

- [ ] 缩到 16×16，还能认出「一个框 + 一条分叉」
- [ ] 转成纯黑单色剪影，形状仍然成立（**本项目降级契约的同一条考试**）
- [ ] 全图只有一处蓝色
- [ ] 全图只有一处绿色，或干脆没有绿色
- [ ] 没有出现任何文字 / 字母 / 数字
- [ ] 没有渐变、发光、投影、3D
- [ ] 线条粗细处处一致
- [ ] 圆角半径与边框粗细比例协调，不像「圆角矩形素材图」
- [ ] 在 `#1e1e2e` 深色底与白色底上各看一眼，边缘都干净

任一条不过就重抽，别靠后期修补。修补出来的图标会在小尺寸暴露。
