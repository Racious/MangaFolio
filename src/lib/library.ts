import type { LibraryBook, ReadingStatus } from "../api/library";

export type LibraryFilter = "all" | "favorites" | "recent" | "missing";
export const statusLabels = { unread: "未讀", reading: "閱讀中", read: "已讀" };
export function progressPercent(book: LibraryBook): number {
  return book.lastReadAt === null
    ? 0
    : Math.min(
        100,
        Math.round(((book.lastIndex + 1) / Math.max(1, book.pageCount)) * 100),
      );
}
export function filterBooks(
  books: LibraryBook[],
  query: string,
  filter: LibraryFilter,
  status: ReadingStatus | "all" = "all",
  tagId: number | null = null,
): LibraryBook[] {
  const term = query.trim().normalize("NFKC").toLocaleLowerCase();
  return books.filter(
    (book) =>
      (filter !== "favorites" || book.favorite) &&
      (filter !== "recent" || book.lastReadAt !== null) &&
      (filter !== "missing" || !book.available) &&
      (status === "all" || book.readingStatus === status) &&
      (tagId === null || book.tags.some((tag) => tag.id === tagId)) &&
      (!term ||
        [
          book.title,
          book.sourceTitle,
          book.series,
          book.volume,
          book.notes,
          ...book.tags.map((tag) => tag.name),
        ].some((value) =>
          value.normalize("NFKC").toLocaleLowerCase().includes(term),
        )),
  );
}
export function sortBooks(
  books: LibraryBook[],
  sort: "recent" | "title" | "series",
): LibraryBook[] {
  const sorted = [...books];
  if (sort === "series")
    sorted.sort(
      (a, b) =>
        a.series.localeCompare(b.series, "zh-TW", { numeric: true }) ||
        a.volume.localeCompare(b.volume, "zh-TW", { numeric: true }) ||
        a.title.localeCompare(b.title, "zh-TW", { numeric: true }),
    );
  else if (sort === "title")
    sorted.sort((a, b) =>
      a.title.localeCompare(b.title, "zh-TW", { numeric: true }),
    );
  else
    sorted.sort(
      (a, b) => (b.lastReadAt ?? 0) - (a.lastReadAt ?? 0) || b.id - a.id,
    );
  return sorted;
}
