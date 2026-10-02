import { test } from "node:test";
import assert from "node:assert/strict";
import { filterBooks, sortBooks } from "../src/lib/library.ts";
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
