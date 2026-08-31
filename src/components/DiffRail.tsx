import { useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent } from "react";

type Props = {
  totalRows: number;
  marks: number[];
  scrollTop: number;
  viewHeight: number;
  linePx: number;
  onJump: (row: number) => void;
  hasClusters?: boolean;
  prevRow?: number | null;
  nextRow?: number | null;
};

type Band = {
  top: number;
  height: number;
  row: number;
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export default function DiffRail({
  totalRows,
  marks,
  scrollTop,
  viewHeight,
  linePx,
  onJump,
  hasClusters = false,
  prevRow = null,
  nextRow = null,
}: Props) {
  const railRef = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState(0);

  useEffect(() => {
    const node = railRef.current;
    if (!node) return;
    const frame = () => setHeight(node.clientHeight);
    frame();
    const observer = new ResizeObserver(frame);
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  const bands = useMemo(() => {
    if (totalRows <= 0 || height <= 0 || marks.length === 0) return [];
    const items: Band[] = [];
    const unit = Math.max(2, height / totalRows);
    for (const row of marks) {
      const top = (row / totalRows) * height;
      const last = items[items.length - 1];
      if (last && top <= last.top + last.height + 1) {
        last.height = Math.max(last.height, top + unit - last.top);
      } else {
        items.push({ top, height: unit, row });
      }
    }
    return items;
  }, [marks, totalRows, height]);

  const contentHeight = Math.max(totalRows, 1) * linePx;
  const thumbTop =
    height > 0 && contentHeight > 0 ? (scrollTop / contentHeight) * height : 0;
  const thumbHeight =
    height > 0 && contentHeight > 0
      ? Math.max(10, (viewHeight / contentHeight) * height)
      : 0;

  function rowAt(clientY: number): number {
    const node = railRef.current;
    if (!node || totalRows <= 0) return 0;
    const rect = node.getBoundingClientRect();
    const y = clamp(clientY - rect.top, 0, rect.height);
    let target = clamp(Math.floor((y / rect.height) * totalRows), 0, totalRows - 1);
    let best = 8;
    for (const mark of marks) {
      const markY = (mark / totalRows) * rect.height;
      const distance = Math.abs(markY - y);
      if (distance <= best) {
        best = distance;
        target = mark;
      }
    }
    return target;
  }

  function handleClick(event: MouseEvent<HTMLDivElement>) {
    if (totalRows <= 0) return;
    onJump(rowAt(event.clientY));
  }

  function jumpPin(event: MouseEvent<HTMLButtonElement>, row: number | null) {
    event.stopPropagation();
    if (row == null) return;
    onJump(row);
  }

  return (
    <div
      ref={railRef}
      className="rail"
      role="navigation"
      aria-label="不一致位置"
      title="点击跳到不一致行"
      onClick={handleClick}
    >
      {thumbHeight > 0 ? (
        <div
          className="rail-thumb"
          style={{ top: thumbTop, height: thumbHeight }}
        />
      ) : null}
      {bands.map((band) => (
        <div
          key={band.row}
          className="rail-mark"
          style={{ top: band.top, height: band.height }}
        />
      ))}
      {hasClusters ? (
        <>
          <button
            type="button"
            className="rail-pin is-prev"
            aria-label="上一条差异"
            title={prevRow == null ? "没有上一条差异" : "上一条差异"}
            disabled={prevRow == null}
            onClick={(event) => jumpPin(event, prevRow)}
          />
          <button
            type="button"
            className="rail-pin is-next"
            aria-label="下一条差异"
            title={nextRow == null ? "没有下一条差异" : "下一条差异"}
            disabled={nextRow == null}
            onClick={(event) => jumpPin(event, nextRow)}
          />
        </>
      ) : null}
    </div>
  );
}
