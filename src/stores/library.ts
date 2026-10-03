import { defineStore } from "pinia";
import { useReaderStore } from "./reader";
import {
  importBookResult,
  listTags,
  type Tag,
  type ReadingStatus,
  listLibrary,
  setFavorite,
  type LibraryBook,
} from "../api/library";

import { importSources, type ImportProgress } from "../lib/import";
import { filterBooks, type LibraryFilter } from "../lib/library";
export type { LibraryFilter } from "../lib/library";
export const useLibraryStore = defineStore("library", {
  state: () => ({
    books: [] as LibraryBook[],
    query: "",
    tags: [] as Tag[],
    statusFilter: "all" as ReadingStatus | "all",
    tagFilter: null as number | null,
    importCanceled: false,
    importProgress: {
      completed: 0,
      total: 0,
      currentPath: "",
      results: [],
    } as ImportProgress,
    filter: "all" as LibraryFilter,
    screen: "library" as "library" | "reader",
    revision: 0,
    loading: false,
    importing: false,
    managing: false,
    error: "",
    importNotice: "",
    favoritePending: new Set<number>(),
  }),
  getters: {
    visibleBooks(state): LibraryBook[] {
      return filterBooks(
        state.books,
        state.query,
        state.filter,
        state.statusFilter,
        state.tagFilter,
      );
    },
    continueBook: (state): LibraryBook | undefined =>
      state.books.find(
        (book) =>
          book.lastReadAt !== null &&
          book.available &&
          book.readingStatus !== "read",
      ),
    favoriteCount: (state) =>
      state.books.filter((book) => book.favorite).length,
    recentCount: (state) =>
      state.books.filter((book) => book.lastReadAt !== null).length,
  },
  actions: {
    async refresh() {
      this.loading = true;
      this.error = "";
      try {
        const [books, tags] = await Promise.all([listLibrary(), listTags()]);
        this.books = books;
        this.tags = tags;
        if (
          this.tagFilter !== null &&
          !tags.some((tag) => tag.id === this.tagFilter)
        )
          this.tagFilter = null;
        this.revision++;
      } catch (e) {
        this.error = String(e);
      } finally {
        this.loading = false;
      }
    },
    cancelImport() {
      this.importCanceled = true;
    },
    async retryImport() {
      const paths = this.importProgress.results
        .filter((item) =>
          ["failed", "conflict", "canceled"].includes(item.kind),
        )
        .map((item) => item.path);
      if (paths.length) await this.add(paths);
    },
    async add(paths: string[]) {
      const reader = useReaderStore();
      if (
        this.importing ||
        this.managing ||
        this.loading ||
        this.favoritePending.size ||
        reader.loading ||
        reader.favoritePending
      )
        return;
      this.importing = true;
      this.importCanceled = false;
      this.error = "";
      this.importNotice = "";
      try {
        await reader.flushProgress();
        await importSources(paths, {
          importSource: importBookResult,
          shouldCancel: () => this.importCanceled,
          onProgress: (progress) => {
            this.importProgress = progress;
          },
        });
        await this.refresh();
        const results = this.importProgress.results;
        this.importNotice = `新增 ${results.filter((r) => r.kind === "added").length} 本，更新 ${results.filter((r) => r.kind === "updated").length} 本，失敗／衝突 ${results.filter((r) => r.kind === "failed" || r.kind === "conflict").length} 本，取消 ${results.filter((r) => r.kind === "canceled").length} 本。`;
      } catch (error) {
        this.error = String(error);
      } finally {
        this.importing = false;
      }
    },
    async toggleFavorite(book: LibraryBook) {
      if (this.managing || this.importing || this.favoritePending.has(book.id))
        return;
      this.favoritePending.add(book.id);
      this.error = "";
      try {
        const favorite = !book.favorite;
        await setFavorite(book.id, favorite);
        const current = this.books.find((entry) => entry.id === book.id);
        if (current) current.favorite = favorite;
        const reader = useReaderStore();
        if (reader.bookId === book.id) reader.favorite = favorite;
      } catch (e) {
        this.error = String(e);
      } finally {
        this.favoritePending.delete(book.id);
      }
    },
  },
});
