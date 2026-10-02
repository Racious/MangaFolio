import type { LibraryBook } from "../api/library";

export type LibraryFilter = "all" | "favorites" | "recent";
export function filterBooks(
  books: LibraryBook[],
  query: string,
  filter: LibraryFilter,
): LibraryBook[] {
  const term = query.trim().normalize("NFKC").toLocaleLowerCase();
  return books.filter(
    (book) =>
      (filter !== "favorites" || book.favorite) &&
      (filter !== "recent" || book.lastReadAt !== null) &&
      (!term ||
        book.title.normalize("NFKC").toLocaleLowerCase().includes(term)),
  );
}
export function sortBooks(
  books: LibraryBook[],
  sort: "recent" | "title",
): LibraryBook[] {
  const sorted = [...books];
  if (sort === "title")
    sorted.sort((a, b) =>
      a.title.localeCompare(b.title, "zh-TW", { numeric: true }),
    );
  else
    sorted.sort(
      (a, b) => (b.lastReadAt ?? 0) - (a.lastReadAt ?? 0) || b.id - a.id,
    );
  return sorted;
}
