import { test } from "node:test";
import assert from "node:assert/strict";
import { filterBooks, sortBooks, displayPath } from "../src/lib/library.ts";
import type { LibraryBook } from "../src/api/library.ts";

function book(
  id: number,
  title: string,
  favorite = false,
  lastReadAt: number | null = null,
): LibraryBook {
  return {
    id,
    title,
    sourceTitle: title,
    customTitle: "",
    series: "",
    volume: "",
    notes: "",
    tags: [],
    readingStatus: lastReadAt === null ? "unread" : "reading",
    statusManual: false,
    favorite,
    lastReadAt,
    path: `/books/${id}`,
    format: "cbz",
    pageCount: 3,
    lastIndex: 0,
    lastPageName: null,
    available: true,
    preferences: {
      direction: "rtl",
      pageMode: "single",
      zoom: "window",
      fixedScale: 1,
      doubleCover: false,
      transition: "book",
    },
  };
}

test("search handles Chinese names, width normalization, case and surrounding spaces", () => {
  const books = [book(1, "海邊故事 第1卷"), book(2, "ＭＡＮＧＡ 第2卷")];
  assert.deepEqual(
    filterBooks(books, " 海邊 ", "all").map((b) => b.id),
    [1],
  );
  assert.deepEqual(
    filterBooks(books, "manga", "all").map((b) => b.id),
    [2],
  );
  assert.equal(filterBooks(books, "missing", "all").length, 0);
});

test("favorite and recent filters combine with search and distinguish unread from page one", () => {
  const books = [
    book(1, "故事1", true),
    book(2, "故事2", false, 100),
    book(3, "別的書", true, 200),
  ];
  assert.deepEqual(
    filterBooks(books, "故事", "favorites").map((b) => b.id),
    [1],
  );
  assert.deepEqual(
    filterBooks(books, "故事", "recent").map((b) => b.id),
    [2],
  );
  assert.deepEqual(
    filterBooks(books, "", "recent").map((b) => b.id),
    [2, 3],
  );
});

test("natural title sorting and recent sorting do not mutate stored library order", () => {
  const books = [
    book(1, "第10卷", false, 200),
    book(2, "第2卷", false, 100),
    book(3, "第1卷"),
  ];
  assert.deepEqual(
    sortBooks(books, "title").map((b) => b.id),
    [3, 2, 1],
  );
  assert.deepEqual(
    sortBooks(books, "recent").map((b) => b.id),
    [1, 2, 3],
  );
  assert.deepEqual(
    books.map((b) => b.id),
    [1, 2, 3],
  );
});

test("full-width b searches titles without matching archive path extensions", () => {
  const books = [book(1, "海邊故事"), book(2, "Blue Stories")];
  books[0].path = "/books/海邊故事.cbz";
  books[1].path = "/books/Blue Stories.cbz";
  assert.deepEqual(
    filterBooks(books, "ｂ", "all").map((b) => b.id),
    [2],
  );
});

test("status, missing-source, favorites, tags and metadata search compose without resetting", () => {
  const a = book(1, "自訂標題", true, 10),
    b = book(2, "其他");
  a.sourceTitle = "原始檔名";
  a.series = "系列１０";
  a.volume = "外傳";
  a.notes = "重要備註";
  a.tags = [{ id: 4, name: "日常" }];
  a.readingStatus = "read";
  a.available = false;
  for (const query of ["自訂", "原始", "系列10", "外傳", "備註", "日常"]) {
    assert.deepEqual(
      filterBooks([a, b], query, "favorites", "read", 4).map((b) => b.id),
      [1],
    );
  }
  assert.equal(filterBooks([a, b], "", "missing", "read", 4).length, 1);
  assert.equal(filterBooks([a, b], "", "favorites", "unread", 4).length, 0);
  b.series = "系列2";
  assert.deepEqual(
    sortBooks([a, b], "series").map((b) => b.id),
    [2, 1],
  );
});

test("large library filtering and sorting keep a stable source array", () => {
  const books = Array.from({ length: 10000 }, (_, i) => {
    const b = book(i + 1, `漫畫第${i + 1}卷`, i % 2 === 0);
    b.tags = [{ id: i % 4, name: "分類" }];
    b.readingStatus = i % 3 === 0 ? "read" : "unread";
    return b;
  });
  const filtered = filterBooks(books, "第1", "favorites", "read", 0);
  assert.ok(filtered.length > 0);
  assert.ok(
    filtered.every(
      (b) => b.favorite && b.readingStatus === "read" && b.tags[0].id === 0,
    ),
  );
  assert.equal(books[0].id, 1);
  assert.equal(books.at(-1)?.id, 10000);
  assert.equal(sortBooks(books, "title")[9].id, 10);
});

test("Windows verbatim paths are simplified only for display", () => {
  assert.equal(displayPath("\\\\?\\C:\\books\\書.zip"), "C:\\books\\書.zip");
  assert.equal(displayPath("路徑不存在：\\\\?\\UNC\\server\\share\\書.zip"), "路徑不存在：\\\\server\\share\\書.zip");
  assert.equal(displayPath("/books/book.zip"), "/books/book.zip");
});
