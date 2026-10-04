import { test } from "node:test";
import assert from "node:assert/strict";
import {
  groupSeries,
  nextVolume,
  sortVolumes,
  bookmarkPage,
} from "../src/lib/series.ts";
import type { LibraryBook } from "../src/api/library.ts";
const book = (
  id: number,
  volume: string,
  extra: Partial<LibraryBook> = {},
): LibraryBook => ({
  id,
  volume,
  series: "aaa",
  title: `aaa ${volume}`,
  sourceTitle: "source",
  customTitle: "",
  notes: "",
  tags: [],
  path: `/isolated/${id}`,
  format: "cbz",
  favorite: false,
  available: true,
  pageCount: 10,
  lastIndex: 5,
  lastReadAt: 100,
  readingStatus: "reading",
  statusManual: false,
  preferences: {
    direction: "rtl",
    pageMode: "single",
    zoom: "window",
    fixedScale: 1,
    doubleCover: false,
    transition: "none",
  },
  ...extra,
});
test("series aggregate separates manual read count and page percentage; resume uses available recent unread state", () => {
  const books = [
    book(1, "1", { readingStatus: "read", statusManual: true, lastIndex: 0 }),
    book(2, "2", { lastReadAt: 400 }),
    book(3, "3", { available: false, lastReadAt: 500 }),
    book(4, "", { series: "" }),
  ];
  const group = groupSeries(books)[0];
  assert.equal(group.read, 1);
  assert.equal(group.percent, 33);
  assert.equal(group.pagePercent, 43);
  assert.equal(group.missing, 1);
  assert.equal(group.resume?.id, 2);
});
test("volume ordering is natural; ambiguous/missing/special/duplicate/gaps never choose arbitrary next", () => {
  assert.deepEqual(
    sortVolumes([
      book(10, "10"),
      book(2, "02"),
      book(1, "１"),
      book(8, "特別篇"),
      book(9, ""),
    ]).map((b) => b.id),
    [1, 2, 10, 9, 8],
  );
  const current = book(1, "1");
  assert.equal(nextVolume(current, [current, book(2, "02")]).book?.id, 2);
  for (const later of [
    book(2, ""),
    book(2, "特別篇"),
    book(2, "3"),
    book(2, "1"),
  ])
    assert.equal(nextVolume(current, [current, later]).book, undefined);
  assert.match(
    nextVolume(current, [current, book(2, "2", { available: false })]).reason,
    /來源/,
  );
  assert.equal(nextVolume(current, [current]).book, undefined);
});
test("bookmark page-name localization follows insertions and refuses missing pages", () => {
  assert.equal(bookmarkPage("2.png", ["new.png", "1.png", "2.png"]), 2);
  assert.throws(
    () => bookmarkPage("deleted.png", ["1.png", "2.png"]),
    /已不存在/,
  );
});
