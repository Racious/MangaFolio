import type { LibraryBook } from "../api/library.ts";
export type ImportKind =
  | "added"
  | "updated"
  | "failed"
  | "conflict"
  | "canceled";
export interface ImportItem {
  path: string;
  kind: ImportKind;
  message: string;
  bookId?: number;
}
export interface ImportProgress {
  completed: number;
  total: number;
  currentPath: string;
  results: ImportItem[];
}
export async function importSources(
  paths: string[],
  options: {
    importSource: (
      path: string,
    ) => Promise<{ book: LibraryBook; kind: "added" | "updated" }>;
    shouldCancel: () => boolean;
    onProgress: (progress: ImportProgress) => void;
  },
): Promise<ImportItem[]> {
  const results: ImportItem[] = [];
  const report = (currentPath = "") =>
    options.onProgress({
      completed: results.length,
      total: paths.length,
      currentPath,
      results: [...results],
    });
  report();
  for (const path of paths) {
    if (options.shouldCancel()) {
      results.push({ path, kind: "canceled", message: "尚未開始，已取消" });
      report();
      continue;
    }
    report(path);
    try {
      const result = await options.importSource(path);
      results.push({
        path,
        kind: result.kind,
        message: result.kind === "added" ? "已新增" : "已更新來源資訊",
        bookId: result.book.id,
      });
    } catch (error) {
      const message = String(error);
      results.push({
        path,
        kind: message.includes("來源衝突") ? "conflict" : "failed",
        message,
      });
    }
    report();
  }
  return results;
}
