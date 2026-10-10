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
import { filterBooks, type LibraryFilter, type MediaFilter } from "../lib/library";
import { openVideo } from "../api/media";
interface MediaView { query: string; filter: LibraryFilter; status: ReadingStatus | "all"; tag: number | null; sort: "recent" | "title" | "series" }
export type { LibraryFilter } from "../lib/library";
const noticeTimers = new WeakMap<object, ReturnType<typeof setTimeout>>();
export const useLibraryStore = defineStore("library", {
  state: () => ({
    books: [] as LibraryBook[],
    query: "",
    mediaFilter: "all" as MediaFilter,
    mediaViews: {} as Partial<Record<MediaFilter, MediaView>>,
    section: "books" as "books" | "series",
    activeSeries: null as string | null,
    seriesQuery: "",
    sort: "recent" as "recent" | "title" | "series",
    seriesSort: "series" as "recent" | "title" | "series",
    selectedIds: [] as number[],
    bookLimit: 60,
    seriesBookLimit: 60,
    seriesLimit: 60,
    scrollAnchor: null as { key: string; offset: number } | null,
    seriesScrollAnchor: null as { key: string; offset: number } | null,
    scrollTop: 0,
    seriesScrollTop: 0,
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
        state.mediaFilter,
      );
    },
    continueBook: (state): LibraryBook | undefined =>
      state.books.find(
        (book) =>
          book.lastReadAt !== null &&
          book.format !== "video" && state.mediaFilter !== "video" &&
          book.available &&
          book.readingStatus !== "read",
      ),
    favoriteCount: (state) =>
      filterBooks(state.books, "", "favorites", "all", null, state.mediaFilter).length,
    recentCount: (state) =>
      filterBooks(state.books, "", "recent", "all", null, state.mediaFilter).length,
  },
  actions: {
    setMediaFilter(media: MediaFilter) {
      if (this.mediaFilter === media) return;
      this.mediaViews[this.mediaFilter] = { query: this.query, filter: this.filter, status: this.statusFilter, tag: this.tagFilter, sort: this.sort };
      const view = this.mediaViews[media];
      this.mediaFilter = media;
      this.query = view?.query ?? ""; this.filter = view?.filter ?? "all";
      this.statusFilter = view?.status ?? "all"; this.tagFilter = view?.tag ?? null; this.sort = view?.sort ?? "recent";
      this.section = "books"; this.activeSeries = null; this.bookLimit = 60; this.scrollTop = 0; this.scrollAnchor = null;
    },
    async openMedia(id: number): Promise<boolean> {
      const reader = useReaderStore();
      if (this.managing || this.importing || this.loading || this.favoritePending.size || reader.loading || reader.favoritePending) return false;
      const book = this.books.find(b => b.id === id);
      if (!book) return false;
      if (book.format !== "video") {
        if (await reader.openBook(id)) { this.screen = "reader"; return true; }
        return false;
      }
      this.managing = true; this.error = "";
      try {
        await reader.flushProgress();
        await openVideo(id);
        await this.refresh(false);
        this.notifySuccess("已交由系統預設播放器開啟。");
        return true;
      } catch (e) {
        await this.refresh();
        this.error = String(e);
        return false;
      } finally { this.managing = false; }
    },
    notifySuccess(message: string) {
      clearTimeout(noticeTimers.get(this));
      this.importNotice = message;
      noticeTimers.set(this, setTimeout(() => {
        if (this.importNotice === message) this.importNotice = "";
        noticeTimers.delete(this);
      }, 5000));
    },
    async setFilter(filter: LibraryFilter) {
      this.filter = filter;
      if (filter === "missing") await this.refresh();
    },
    async refresh(refreshCovers = true) {
      // Metadata-only refreshes run under openMedia's managing guard and keep cards mounted.
      if (refreshCovers) this.loading = true;
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
        if (refreshCovers) this.revision++;
      } catch (e) {
        this.error = String(e);
      } finally {
        if (refreshCovers) this.loading = false;
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
        const message = `新增 ${results.filter((r) => r.kind === "added").length} 本，更新 ${results.filter((r) => r.kind === "updated").length} 本，失敗／衝突 ${results.filter((r) => r.kind === "failed" || r.kind === "conflict").length} 本，取消 ${results.filter((r) => r.kind === "canceled").length} 本。`;
        if (results.some(result => ["failed", "conflict", "canceled"].includes(result.kind)))
          this.importNotice = message;
        else this.notifySuccess(message);
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
