import { describe, expect, it } from "vitest";
import {
  clusterMarks,
  folderStopRows,
  mapStartToFilteredIndex,
  pinNav,
  pinTargets,
  remapClusterStarts,
} from "./diffNav";

describe("clusterMarks", () => {
  it("returns no clusters for an empty list", () => {
    expect(clusterMarks([])).toEqual([]);
  });

  it("treats a single row as one cluster", () => {
    expect(clusterMarks([7])).toEqual([{ start: 7, end: 7 }]);
  });

  it("merges consecutive dirty rows", () => {
    expect(clusterMarks([10, 11, 12])).toEqual([{ start: 10, end: 12 }]);
  });

  it("merges when one equal row sits between marks", () => {
    expect(clusterMarks([10, 12])).toEqual([{ start: 10, end: 12 }]);
  });

  it("merges when two equal rows sit between marks", () => {
    expect(clusterMarks([10, 11, 12, 15, 20])).toEqual([
      { start: 10, end: 15 },
      { start: 20, end: 20 },
    ]);
  });

  it("splits when three equal rows sit between marks", () => {
    expect(clusterMarks([10, 14])).toEqual([
      { start: 10, end: 10 },
      { start: 14, end: 14 },
    ]);
  });

  it("sorts and dedupes before clustering", () => {
    expect(clusterMarks([12, 10, 10, 11])).toEqual([{ start: 10, end: 12 }]);
  });
});

describe("pinTargets", () => {
  const clusters = [
    { start: 10, end: 15 },
    { start: 40, end: 42 },
    { start: 80, end: 80 },
  ];

  it("disables prev on the first cluster start", () => {
    expect(pinTargets(clusters, 10)).toEqual({ prev: null, next: 40 });
  });

  it("disables next on the last cluster start", () => {
    expect(pinTargets(clusters, 80)).toEqual({ prev: 40, next: null });
  });

  it("picks neighbors when the viewport is between clusters", () => {
    expect(pinTargets(clusters, 20)).toEqual({ prev: 10, next: 40 });
  });

  it("skips the current hunk when the viewport is inside it", () => {
    expect(pinTargets(clusters, 12)).toEqual({ prev: 10, next: 40 });
  });

  it("disables both pins when sitting on the only cluster", () => {
    expect(pinTargets([{ start: 5, end: 8 }], 5)).toEqual({
      prev: null,
      next: null,
    });
  });
});

describe("mapStartToFilteredIndex", () => {
  it("counts marks strictly before start", () => {
    expect(mapStartToFilteredIndex([10, 11, 12, 20], 20)).toBe(3);
    expect(mapStartToFilteredIndex([10, 11, 12, 20], 10)).toBe(0);
  });
});

describe("pinNav", () => {
  const marks = [10, 11, 12, 20];

  it("hides pins for the same-only filter", () => {
    expect(pinNav(marks, 0, "same")).toEqual({
      hasClusters: false,
      prevRow: null,
      nextRow: null,
    });
  });

  it("uses unfiltered indexes for the all filter", () => {
    expect(pinNav(marks, 10, "all")).toEqual({
      hasClusters: true,
      prevRow: null,
      nextRow: 20,
    });
  });

  it("maps hunk starts into the diff-only list", () => {
    expect(pinNav(marks, 0, "diff")).toEqual({
      hasClusters: true,
      prevRow: null,
      nextRow: 3,
    });
  });

  it("hides pins when there are no marks", () => {
    expect(pinNav([], 0, "all")).toEqual({
      hasClusters: false,
      prevRow: null,
      nextRow: null,
    });
  });
});

describe("folderStopRows", () => {
  it("stops on a collapsed dirty directory", () => {
    expect(
      folderStopRows([
        { status: "different", kind: "dir", expanded: false },
      ]),
    ).toEqual([0]);
  });

  it("skips an expanded dirty directory and stops on dirty files", () => {
    expect(
      folderStopRows([
        { status: "different", kind: "dir", expanded: true },
        { status: "different", kind: "file", expanded: false },
        { status: "equal", kind: "file", expanded: false },
      ]),
    ).toEqual([1]);
  });

  it("does not stop on equal files", () => {
    expect(
      folderStopRows([{ status: "equal", kind: "file", expanded: false }]),
    ).toEqual([]);
  });

  it("stops on left-only, right-only, and type-conflict rows", () => {
    expect(
      folderStopRows([
        { status: "leftOnly", kind: "file", expanded: false },
        { status: "rightOnly", kind: "file", expanded: false },
        { status: "typeConflict", kind: "file", expanded: false },
      ]),
    ).toEqual([0, 1, 2]);
  });
});

describe("remapClusterStarts", () => {
  it("drops clusters whose start is not visible", () => {
    expect(
      remapClusterStarts(
        [
          { start: 0, end: 0 },
          { start: 2, end: 2 },
        ],
        (start) => (start === 2 ? 0 : null),
      ),
    ).toEqual([{ start: 0, end: 0 }]);
  });
});
