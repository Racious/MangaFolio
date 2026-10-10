import type { LibraryBook } from "../api/library";
import { progressPercent } from "./library.ts";
export const seriesKey = (name: string) => name.trim().toLowerCase();
export interface SeriesGroup {
  key: string;
  name: string;
  books: LibraryBook[];
  cover: LibraryBook;
  read: number;
  percent: number;
  pagePercent: number;
  missing: number;
  resume?: LibraryBook;
}
export function groupSeries(books: LibraryBook[]): SeriesGroup[] {
  const groups = new Map<string, LibraryBook[]>();
  for (const book of books) {
    if (book.format === "video") continue;
    const key = seriesKey(book.series);
    if (key) {
      const group = groups.get(key) ?? [];
      group.push(book);
      groups.set(key, group);
    }
  }
  return [...groups]
    .map(([key, members]) => {
      const ordered = sortVolumes(members);
      const read = members.filter((b) => b.readingStatus === "read").length;
      return {
        key,
        name: members[0].series.trim(),
        books: ordered,
        cover: ordered.find((b) => b.available) ?? ordered[0],
        read,
        percent: Math.round((read / members.length) * 100),
        pagePercent: Math.round(
          members.reduce((n, b) => n + progressPercent(b), 0) / members.length,
        ),
        missing: members.filter((b) => !b.available).length,
        resume: [...members]
          .filter(
            (b) =>
              b.available &&
              b.lastReadAt !== null &&
              b.readingStatus !== "read",
          )
          .sort(
            (a, b) => (b.lastReadAt ?? 0) - (a.lastReadAt ?? 0) || b.id - a.id,
          )[0],
      };
    })
    .sort((a, b) => a.name.localeCompare(b.name, "zh-TW", { numeric: true }));
}
export function numericVolume(value: string): number | null {
  const text = value.trim().normalize("NFKC");
  if (!/^\d+$/.test(text)) return null;
  const n = Number(text);
  return Number.isSafeInteger(n) && n <= 1_000_000 ? n : null;
}
export function sortVolumes(books: LibraryBook[]): LibraryBook[] {
  return [...books].sort((a, b) => {
    const av = numericVolume(a.volume),
      bv = numericVolume(b.volume);
    if (av !== null && bv !== null && av !== bv) return av - bv;
    if ((av === null) !== (bv === null)) return av === null ? 1 : -1;
    return (
      a.volume.localeCompare(b.volume, "zh-TW", { numeric: true }) ||
      a.title.localeCompare(b.title, "zh-TW", { numeric: true }) ||
      a.id - b.id
    );
  });
}
export function nextVolume(
  current: LibraryBook,
  books: LibraryBook[],
): { book?: LibraryBook; reason: string } {
  const key = seriesKey(current.series);
  if (!key) return { reason: "這本書尚未指定系列。" };
  const series = books.filter((b) => b.format !== "video" && seriesKey(b.series) === key),
    seen = new Set<number>();
  for (const b of series) {
    const n = numericVolume(b.volume);
    if (n === null)
      return { reason: "系列有缺少、非數字或特殊集數，請從系列詳情選書。" };
    if (seen.has(n)) return { reason: "系列集數重複，請從系列詳情選書。" };
    seen.add(n);
  }
  const n = numericVolume(current.volume);
  if (n === null) return { reason: "目前集數無法判定。" };
  const higher = series
    .filter((b) => numericVolume(b.volume)! > n)
    .sort((a, b) => numericVolume(a.volume)! - numericVolume(b.volume)!);
  if (!higher.length) return { reason: "已是目前系列的最後一集。" };
  if (numericVolume(higher[0].volume) !== n + 1)
    return { reason: "下一集集數跳號，請从系列詳情確認。" };
  const book = higher[0];
  return {
    book,
    reason: book.available
      ? ""
      : "下一集來源無法存取，請重新連結；不會略過到其他集。",
  };
}
export function bookmarkPage(pageName: string, pages: string[]): number {
  const index = pages.indexOf(pageName);
  if (index < 0)
    throw new Error("書籤頁面已不存在，未跳轉。請確認來源或重新連結。");
  return index;
}
