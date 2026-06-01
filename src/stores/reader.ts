// 閱讀器狀態：當前書、頁碼、閱讀方向、單／雙頁、縮放模式、翻頁特效、封面配對。

import { defineStore } from "pinia";
import { openPath, renderPageUrl, type FitMode } from "../api/backend";

/** 閱讀方向：rtl = 右開（日漫慣用），ltr = 左開。 */
export type Direction = "rtl" | "ltr";
/** 版面：單頁／雙頁。 */
export type PageMode = "single" | "double";
/** 翻頁特效。 */
export type Transition = "book" | "slide" | "fade" | "none";

/** 雙頁時兩頁之間的間隙（像素）。漫畫跨頁緊貼，不留黑溝。 */
const DOUBLE_GAP = 0;

/** 一個顯示槽：邏輯頁碼與其已縮放影像的物件 URL。 */
interface ViewSlot {
  index: number;
  url: string;
}

/** 求頁碼所屬跨頁的起始頁碼。doubleCover：封面（第 0 頁）單獨成頁，其後 1-2、3-4… 配對。 */
function pairStartOf(index: number, double: boolean, cover: boolean): number {
  if (!double) return index;
  if (cover) {
    if (index <= 0) return 0;
    return index - ((index - 1) % 2);
  }
  return index - (index % 2);
}

/** 求以 start 起始的跨頁包含的邏輯頁碼（邏輯順序，未套左右開）。 */
function indicesOf(start: number, double: boolean, cover: boolean, count: number): number[] {
  if (count === 0) return [];
  if (!double) return [start];
  if (cover && start === 0) return [0]; // 封面單獨
  return start + 1 < count ? [start, start + 1] : [start];
}

export const useReaderStore = defineStore("reader", {
  state: () => ({
    title: "",
    pages: [] as string[],
    index: 0,
    direction: "rtl" as Direction,
    pageMode: "single" as PageMode,
    zoom: "window" as FitMode,
    fixedScale: 1.0,
    viewportW: 0,
    viewportH: 0,
    slots: [] as ViewSlot[],
    loading: false,
    error: "",
    renderToken: 0,
    transition: "book" as Transition,
    /** 翻頁方向：1 = 往後，-1 = 往前。供特效決定方位。 */
    flow: 1,
    /** 內容換好的序號；特效在此遞增時觸發。 */
    viewSeq: 0,
    /** 雙頁時封面（第 0 頁）是否單獨成頁，以校正後續跨頁配對。 */
    doubleCover: false,
  }),

  getters: {
    pageCount: (s) => s.pages.length,
    hasBook: (s) => s.pages.length > 0,
    isDouble: (s) => s.pageMode === "double",
    atFirst: (s) => s.index <= 0,
    /** 當前視圖涵蓋的邏輯頁碼（邏輯順序，未套用左右開）。 */
    viewIndices(s): number[] {
      const dbl = s.pageMode === "double";
      const start = pairStartOf(s.index, dbl, s.doubleCover);
      return indicesOf(start, dbl, s.doubleCover, s.pages.length);
    },
    atLast(): boolean {
      const v = this.viewIndices;
      return v.length ? Math.max(...v) >= this.pages.length - 1 : true;
    },
  },

  actions: {
    /** 頁碼 → 所屬跨頁起始頁碼。 */
    pairStart(index: number): number {
      return pairStartOf(index, this.pageMode === "double", this.doubleCover);
    },

    /** 以 start 起始的跨頁邏輯頁碼。 */
    indicesForStart(start: number): number[] {
      return indicesOf(start, this.pageMode === "double", this.doubleCover, this.pages.length);
    },

    /** 下一個／上一個跨頁的起始頁碼；無則回傳 -1。 */
    nextStart(): number {
      const v = this.viewIndices;
      if (!v.length) return -1;
      const last = Math.max(...v);
      if (last + 1 > this.pages.length - 1) return -1;
      return this.pairStart(last + 1);
    },
    prevStart(): number {
      const v = this.viewIndices;
      if (!v.length) return -1;
      const first = Math.min(...v);
      if (first - 1 < 0) return -1;
      return this.pairStart(first - 1);
    },

    async open(path: string) {
      this.loading = true;
      this.error = "";
      try {
        const info = await openPath(path);
        this.title = info.title;
        this.pages = info.pages;
        this.index = this.pairStart(info.startIndex);
        await this.render();
      } catch (e) {
        this.error = String(e);
        this.pages = [];
        this.title = "";
        this.clearSlots();
      } finally {
        this.loading = false;
      }
    },

    /** 核心渲染：依模式向後端索取已縮放的頁面並排版。 */
    async render() {
      if (!this.hasBook || this.viewportW === 0 || this.viewportH === 0) return;

      const indices = this.viewIndices;
      const double = indices.length === 2;
      const slotW = double
        ? Math.max(1, Math.floor((this.viewportW - DOUBLE_GAP) / 2))
        : this.viewportW;

      const token = ++this.renderToken;
      this.error = "";
      try {
        const urls = await Promise.all(
          indices.map((i) =>
            renderPageUrl({
              index: i,
              mode: this.zoom,
              viewportW: slotW,
              viewportH: this.viewportH,
              fixedScale: this.fixedScale,
            })
          )
        );
        if (token !== this.renderToken) {
          urls.forEach((u) => URL.revokeObjectURL(u));
          return;
        }
        const next: ViewSlot[] = indices.map((i, k) => ({ index: i, url: urls[k] }));
        this.clearSlots();
        // rtl（右開）時，邏輯較前的頁面顯示於右側。
        this.slots = this.direction === "rtl" ? next.reverse() : next;
        this.viewSeq++;
      } catch (e) {
        if (token === this.renderToken) this.error = String(e);
      }
    },

    /** 跳至指定頁（自動對齊跨頁起點）。 */
    async goto(target: number) {
      const max = Math.max(0, this.pages.length - 1);
      const t = Math.max(0, Math.min(target, max));
      const newIndex = this.pairStart(t);
      this.flow = newIndex >= this.index ? 1 : -1;
      this.index = newIndex;
      await this.render();
    },

    next() {
      const n = this.nextStart();
      if (n >= 0) return this.goto(n);
    },
    prev() {
      const p = this.prevStart();
      if (p >= 0) return this.goto(p);
    },

    setViewport(w: number, h: number) {
      if (w === this.viewportW && h === this.viewportH) return;
      this.viewportW = w;
      this.viewportH = h;
      this.render();
    },

    /** 以當前縮放參數索取指定頁碼影像 URL（供翻頁特效取相鄰跨頁，不改變狀態）。 */
    async peekUrls(indices: number[]): Promise<string[]> {
      const double = indices.length === 2;
      const slotW = double
        ? Math.max(1, Math.floor((this.viewportW - DOUBLE_GAP) / 2))
        : this.viewportW;
      return Promise.all(
        indices.map((i) =>
          renderPageUrl({
            index: i,
            mode: this.zoom,
            viewportW: slotW,
            viewportH: this.viewportH,
            fixedScale: this.fixedScale,
          })
        )
      );
    },

    /** 翻頁特效完成後，以已備好（已解碼）的影像直接換頁，避免閃爍。 */
    commitWith(target: number, indices: number[], urls: string[]) {
      this.flow = target >= this.index ? 1 : -1;
      this.index = this.pairStart(target);
      const next: ViewSlot[] = indices.map((i, k) => ({ index: i, url: urls[k] }));
      this.clearSlots();
      this.slots = this.direction === "rtl" ? next.reverse() : next;
      this.viewSeq++;
    },

    setZoom(mode: FitMode) {
      this.zoom = mode;
      this.render();
    },

    setTransition(mode: Transition) {
      this.transition = mode;
    },

    setFixedScale(scale: number) {
      this.fixedScale = Math.min(8, Math.max(0.1, scale));
      if (this.zoom === "fixed") this.render();
    },

    togglePageMode() {
      this.pageMode = this.pageMode === "single" ? "double" : "single";
      this.index = this.pairStart(this.index);
      this.render();
    },

    toggleDoubleCover() {
      this.doubleCover = !this.doubleCover;
      this.index = this.pairStart(this.index);
      this.render();
    },

    toggleDirection() {
      this.direction = this.direction === "rtl" ? "ltr" : "rtl";
      this.slots = [...this.slots].reverse();
    },

    clearSlots() {
      this.slots.forEach((s) => URL.revokeObjectURL(s.url));
      this.slots = [];
    },
  },
});
