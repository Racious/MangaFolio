import { defineStore } from "pinia";
import {
  importBook,
  listLibrary,
  setFavorite,
  type LibraryBook,
} from "../api/library";

import { filterBooks, type LibraryFilter } from "../lib/library";
export type { LibraryFilter } from "../lib/library";
export const useLibraryStore = defineStore("library", {
  state: () => ({
    books: [] as LibraryBook[],
    query: "",
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
      return filterBooks(state.books, state.query, state.filter);
    },
    continueBook: (state): LibraryBook | undefined =>
      state.books.find((book) => book.lastReadAt !== null && book.available),
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
        this.books = await listLibrary();
        this.revision++;
      } catch (e) {
        this.error = String(e);
      } finally {
        this.loading = false;
      }
    },
    async add(paths: string[]) {
      if (this.importing || this.managing) return;
      this.importing = true;
      this.error = "";
      this.importNotice = "";
      const failures: string[] = [];
      let added = 0;
      try {
        for (const path of paths) {
          try {
            await importBook(path);
            added++;
          } catch (e) {
            failures.push(`${path.split(/[\\/]/).pop()}: ${String(e)}`);
          }
        }
        await this.refresh();
        if (failures.length) this.error = failures.join("\n");
        this.importNotice = `已加入／更新 ${added} 本書${failures.length ? `，${failures.length} 本無法加入` : ""}。`;
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
      } catch (e) {
        this.error = String(e);
      } finally {
        this.favoritePending.delete(book.id);
      }
    },
  },
});
