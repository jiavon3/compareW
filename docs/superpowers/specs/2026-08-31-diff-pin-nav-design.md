# CompareW: 中间栏圆点「上一条 / 下一条」差异

Date: 2026-08-31  
Status: Approved

## Problem

中间分隔栏上的两颗黄圆点目前只是 CSS 装饰（`.rail::before` / `::after`，`pointer-events: none`）。栏身已经能点红条跳到不一致行，但没有「上一团 / 下一团」导航。这是比对工具里最缺的一块操作。

## Goal

两颗圆点变成上一条 / 下一条差异的按钮：点一次跳过一整团不一致（中间隔 1～2 行相同仍算同一团）。到头变暗、不绕圈。文本、Excel、文件夹三页都有。这版不做快捷键。

## Non-goals

- 快捷键（`F7` / `F8` / `Alt+↑↓` 以后再加）
- 走到头后绕回另一头
- 圆点跟着热点跑、书签钉、拖左右宽度、解开同步滚动
- 点圆点时自动展开文件夹
- 文件夹中间栏画红条缩略（这版仍只靠圆点）
- 为导航单独从 Rust 再传一份未采样的完整差异行号
- 改皮肤、改圆点颜色和大小（`disabled` 变暗除外）
- 合并、拷到对侧、三路比对

## Tech stack

不变：Tauri 2、React、TypeScript、Vite。比对仍在 Rust。前端不成 diff、不哈希、不读 zip。成团只是把后端已给的不一致行号收成区间。

## Architecture

```
文本 / Excel / 文件夹
  各自取出未过滤的可停行号
       ↓
  clusterMarks → pinTargets → 必要时 mapStartToFilteredIndex
       ↓
  DiffRail({ hasClusters, prevRow, nextRow, onJump })
       ↓
  现有 onJump(当前列表行号)
```

Rust 命令和 `DiffStore` / Excel store / 文件夹扫描都不改。

### 可停行号

| 页 | 来源 |
|---|---|
| 文本 | `summary.diffMarks`（与红条同一份；`cap_marks` 上限 4000） |
| Excel | 当前工作表 `dirtyRows`（同样上限 4000） |
| 文件夹 | 当前可见树里满足「可停」的行下标，无采样上限 |

大文件导航精度与点红条一致。这版不为圆点突破 4000 上限。

### 组件

- 新增 `src/lib/diffNav.ts`：`clusterMarks`、`pinTargets`、`folderStopRows`、以及筛选「差别」时用的 `mapStartToFilteredIndex`。
- 各页自己成团、算 `prev` / `next`（已映射到当前列表行号），再传给 `DiffRail`。红条 `marks` 与导航行号分开：筛选不是「全部」时红条仍可为空，圆点不受影响。
- `DiffRail` 新增可选 `prevRow: number | null`、`nextRow: number | null`。去掉伪元素圆点，改成两颗 `<button>`。点圆点调用已有 `onJump`。栏身点击仍按现有逻辑跳最近不一致行（按行，不成团）。
- 文本、Excel：继续挂 `DiffRail`，用已有 `scrollTop` / `linePx` 算顶行。
- 文件夹：把空的 `<div class="rail">` 换成 `DiffRail`。`marks` 传 `[]`（不画红条）。同步 `scrollTop` / `viewHeight` / `linePx`（`FOLDER_ROW_PX`）和 `onJump`。

## Clustering

输入：有序或无序的行号数组。先升序去重。

相邻两个可停行 `a < b`，若中间相同行数 ≤ 2，即 `b - a <= 3`，则并进同一团。

团：`{ start, end }`，`start` / `end` 含两端可停行。跳转落点永远是 `start`。

单行不一致也是一团。隔 3 行及以上相同则拆开。

例：`10,11,12,15,20` → `[10–15]` 与 `[20–20]`（12 与 15 中间两行相同，仍并）。

## Pin targets

`topRow = floor(scrollTop / linePx)`，与虚拟列表同一套行高。

- **下一条** `next`：`start > topRow` 的最近一团的 `start`；没有则为 `null`
- **上一条** `prev`：`start < topRow` 的最近一团的 `start`；没有则为 `null`

已经停在某团开头（`topRow === cluster.start`）时，再点才进邻团。视口停在某团中间（`start < topRow <= end`）时：下一条仍是下一团，上一条仍是上一团，不在本团内挪动。

只有一团且视口已在它开头：两颗都 `disabled`。视口在文件顶、第一团还在下方：上点 `disabled`，下点可点。

## 圆点外观与按钮状态

位置仍约栏身 18%（上）与距底 18%（下）。金色径向渐变、尺寸 8px、描边保持现有 `--pin` 样式。

| 状态 | 行为 |
|---|---|
| 一团都没有 | 两颗不渲染 |
| 该方向有目标 | 可点；悬停略亮 |
| 该方向无目标 | 渲染但 `disabled`：变暗、`cursor: default`、点了不跳 |

`aria-label`：上点「上一条差异」，下点「下一条差异」。`title` 同文案。`disabled` 时 title 可为「没有上一条差异」/「没有下一条差异」。

点圆点走 `onJump(团.start)`，沿用各页现有 `jumpToRow`（改 `scrollTop`、拉虚拟窗口）。不绕圈、不闪动。

栏身点红条与圆点独立：红条仍跳最近不一致**行**。

## 筛选

团的合并必须按「未把相同行藏掉」时的间隔，否则「差别」模式下所有不一致行相邻，会并成一大团。

**文本 / Excel**

- 筛选「全部」：`marks` 即 `diffMarks` / `dirtyRows`，行号与 `jumpToRow` 同一空间。
- 筛选「相同」：无不一致团，圆点不渲染。
- 筛选「差别」：仍用未过滤的 `diffMarks` / `dirtyRows` 成团。跳转时把未过滤行号 `start` 映射为过滤列表下标：`marks` 里严格小于 `start` 的个数。当 marks 未被 `cap_marks` 截断时映射精确；截断时与缩略条一样是近似，这版接受。

**文件夹**

可见列表在前端完整。先在**未按筛选裁切的 flatten 结果**上算可停行并成团，再把 `start` 映射到当前筛选后列表的下标（该行仍可见时）。筛选「相同」时没有可停行，圆点不渲染。筛选「差别」时映射精确（无 4000 上限）。

## 文件夹可停行

`folderStopRows(items, expanded)` 在 flatten 后的可见树（尚未应用全部/差别/相同筛选）上返回可停行下标：

- `status === equal`：不停
- 文件，或 `leftOnly` / `rightOnly` / `typeConflict`：可停
- 目录或压缩包：未展开且 `status !== equal` → 可停；已展开 → 跳过（子行自己决定）

展开/折叠后重算团，不自动展开。落到折叠红目录时只滚动选中该行，用户自己双击展开。

## Data flow

圆点点击不得在前端算 diff。只读各页已经拿到的行号。

文件夹 `DiffRail` 需要与左右列表同步滚动位置。列表已有 `syncFrom` 式双向同步，把同一 `scrollTop` 传给 `DiffRail` 即可。`totalRows` 用筛选后可见行数，供栏高度与（若将来画红条）比例；这版文件夹 `marks` 传 `[]`。

文本在 `mode === "edit"` 且尚未出结果时：`diffMarks` 可能仍在，但圆点只在有团且用户能跳到结果行时有意义。与现有栏点击一致：编辑态点跳转会先切到 `result`（已有 `pendingJump`）。圆点同样走 `onJump`，因此编辑态点圆点也会切到结果并跳。若当前没有团（空比对），不渲染圆点。

Excel 换工作表后用新表 `dirtyRows` 重算；视口通常已归零，圆点跟新表走。

## Error handling

- `marks` 为空或 `totalRows === 0`：不渲染圆点；栏身点击仍按现有空数据保护（不跳）。
- `onJump` 行号夹紧到 `[0, totalRows)`。成团结果若因筛选映射落到空，该方向视为无目标（`disabled`）。
- 文件夹尚未扫描完 / 两侧都空：无可见行，无圆点。

## Testing

仓库目前只有 Rust 测。这版加 Vitest，只测 `src/lib/diffNav.ts`。不测页面组件。Rust 域测试不新增。

**clusterMarks**

- `[]` → `[]`
- `[7]` → `[{start:7,end:7}]`
- 连续行并成一团
- 中间隔 1 行相同、隔 2 行相同 → 一团
- 中间隔 3 行相同 → 两团
- 乱序输入与重复行号 → 排序去重后与有序结果相同

**pinTargets**

- 视口在第一团开头：`prev` 空，`next` 为第二团 `start`
- 视口在最后一团开头：`next` 空
- 视口在两团之间：`prev` 上一团、`next` 下一团
- 视口在某团中间：`prev` 上一团，`next` 下一团
- 只有一团且 `topRow === start`：两侧都空

**folderStopRows**

- 折叠红目录：可停
- 同一目录展开后：目录本身不可停，红文件可停
- 相同文件：不可停
- 仅左侧 / 仅右侧 / 类型冲突：可停

页面级不新开 E2E。实现后手动：文本/Excel/文件夹多团上下跳、到头变暗、筛选「相同」圆点消失、文件夹展开前后落点变化。

## Implementation notes

- 不要重做皮肤。纸色比对区、深色顶栏、一致黑/不一致红不变。
- 圆点保持 `--pin` 金色；不要换成红点或新图标。
- 文件夹行高 24px，文本/Excel 行高 20px，`linePx` 按页传入。
- 点击圆点须 `stopPropagation`，避免落到栏身「跳最近红条」逻辑上。
- `hasClusters === false`：不渲染圆点。`hasClusters === true`：两颗都渲染，没有目标的那颗 `disabled`。
