<script setup lang="ts">
import {
  PhBooks,
  PhStar,
  PhClockCounterClockwise,
  PhFolderNotchOpen,
} from "@phosphor-icons/vue";
import {
  computed, ref, watch, toRef, onMounted, onBeforeUnmount, nextTick,
} from "vue";
import BookDetailsPanel from "./BookDetailsPanel.vue";
import SeriesShelf from "./SeriesShelf.vue";
import BookCover from "./BookCover.vue";
import { groupSeries, seriesKey, sortVolumes } from "../lib/series";
import { filterBooks } from "../lib/library";
import ContinueReading from "./ContinueReading.vue";
import BookCard from "./BookCard.vue";
import BookEditor from "./BookEditor.vue";
import LibraryGuide from "./LibraryGuide.vue";
import LibraryManager from "./LibraryManager.vue";
import TagManager from "./TagManager.vue";
import AppearanceSettings from "./AppearanceSettings.vue";
import { useLibraryStore, type LibraryFilter } from "../stores/library";
import { useReaderStore } from "../stores/reader";
import { useAppearanceStore } from "../stores/appearance";
import { pickBookFiles, type LibraryBook } from "../api/library";
import { sortBooks } from "../lib/library";
import { ask } from "@tauri-apps/plugin-dialog";
import { assignSeries } from "../api/library";
import { pickFolder } from "../api/backend";
const library = useLibraryStore(),
  reader = useReaderStore(),
  appearance = useAppearanceStore();
const limit = computed({
  get: () => (library.activeSeries ? library.seriesBookLimit : library.bookLimit),
  set: (value) => {
    if (library.activeSeries) library.seriesBookLimit = value;
    else library.bookLimit = value;
  },
});
const guideTarget = ref(""),
  managementOpen = ref(false),
  navigationOpen = ref(false);
const selectedIds = toRef(library, "selectedIds"),
  editingBook = ref<LibraryBook | null>(null);
const detailBook = ref<LibraryBook | null>(null);
const busy = computed(
  () =>
    library.managing ||
    library.importing ||
    library.loading ||
    reader.loading ||
    reader.favoritePending ||
    library.favoritePending.size > 0,
);
const content = ref<HTMLElement | null>(null);
const renamedSeries = ref("");
async function renameSeries(remove = false) {
  const group = activeGroup.value;
  if (!group || busy.value) return;
  const name = remove ? "" : renamedSeries.value.trim();
  if (!remove && !name) {
    library.error = "請輸入系列名稱。";
    return;
  }
  library.managing = true;
  try {
    if (
      remove &&
      !(await ask("移除整個系列歸屬？書籍與來源仍保留。", {
        title: "移除系列歸屬",
        kind: "warning",
      }))
    )
      return;
    await reader.flushProgress();
    await assignSeries(
      group.books.map((b) => b.id),
      name,
    );
    library.activeSeries = remove ? null : seriesKey(name);
    await library.refresh();
  } catch (e) {
    library.error = String(e);
  } finally {
    library.managing = false;
  }
}
const sort = computed({
  get: () => (library.activeSeries ? library.seriesSort : library.sort),
  set: (value) => {
    if (library.activeSeries) library.seriesSort = value;
    else library.sort = value;
  },
});
const query = computed({
  get: () => (library.activeSeries ? library.seriesQuery : library.query),
  set: (value) => {
    if (library.activeSeries) library.seriesQuery = value;
    else library.query = value;
  },
});
const groups = computed(() => groupSeries(library.books));
const activeGroup = computed(() =>
  groups.value.find((g) => g.key === library.activeSeries),
);
watch(
  () => activeGroup.value?.name,
  (name) => {
    renamedSeries.value = name ?? "";
  },
  { immediate: true },
);
const seriesLimit = toRef(library, "seriesLimit");
const visibleGroups = computed(() => {
  const ids = new Set(library.visibleBooks.map((b) => b.id));
  return groups.value.filter((g) => g.books.some((b) => ids.has(b.id)));
});
// The narrow layout scrolls the outer container; desktop scrolls its content.
const layout = ref<HTMLElement | null>(null);
const narrow = ref(window.matchMedia("(max-width: 700px)").matches);
let restoring = true;
let restoreGeneration = 0;
let disposed = false;
function scrollOwner() {
  return narrow.value ? layout.value : content.value;
}
function positionCards(owner: HTMLElement) {
  return Array.from(owner.querySelectorAll<HTMLElement>("[data-book-id], [data-series-key]"));
}
function positionKey(card: HTMLElement) {
  return card.dataset.bookId
    ? `book:${card.dataset.bookId}`
    : `series:${card.dataset.seriesKey}`;
}
function recordScroll(event?: Event) {
  const owner = scrollOwner();
  if (!owner || restoring || (event && event.target !== owner)) return;
  const top = owner.getBoundingClientRect().top;
  const bottom = owner.getBoundingClientRect().bottom;
  const card = positionCards(owner).find((el) => {
    const rect = el.getBoundingClientRect();
    return rect.bottom > top && rect.top < bottom;
  });
  const anchor = card
    ? { key: positionKey(card), offset: card.getBoundingClientRect().top - top }
    : null;
  if (library.activeSeries) {
    library.seriesScrollTop = owner.scrollTop;
    library.seriesScrollAnchor = anchor;
  } else {
    library.scrollTop = owner.scrollTop;
    library.scrollAnchor = anchor;
  }
}
async function restoreScroll() {
  restoring = true;
  const generation = ++restoreGeneration;
  await nextTick();
  await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
  if (disposed || generation !== restoreGeneration || library.loading) return;
  const owner = scrollOwner();
  if (owner) {
    owner.scrollTop = library.activeSeries ? library.seriesScrollTop : library.scrollTop;
    const anchor = library.activeSeries ? library.seriesScrollAnchor : library.scrollAnchor;
    const card = anchor && positionCards(owner).find((el) => positionKey(el) === anchor.key);
    if (card && anchor) {
      owner.scrollTop += card.getBoundingClientRect().top
        - owner.getBoundingClientRect().top - anchor.offset;
    }
  }
  // Ignore scroll events generated by restoring the DOM after a refresh.
  await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
  if (!disposed && generation === restoreGeneration) restoring = false;
}
async function enterSeries(key: string) {
  recordScroll();
  restoring = true;
  library.activeSeries = key;
  library.seriesBookLimit = 60;
  library.seriesQuery = "";
  library.seriesScrollTop = 0;
  library.seriesScrollAnchor = null;
  renamedSeries.value = activeGroup.value?.name ?? "";
  await restoreScroll();
}
async function backSeries() {
  restoring = true;
  library.activeSeries = null;
  await restoreScroll();
}
function resize() {
  narrow.value = window.matchMedia("(max-width: 700px)").matches;
  void restoreScroll();
}
onMounted(() => {
  window.addEventListener("resize", resize);
  void restoreScroll();
});
onBeforeUnmount(() => {
  recordScroll();
  disposed = true;
  ++restoreGeneration;
  window.removeEventListener("resize", resize);
});
watch(() => library.loading, (loading) => {
  if (!loading) void restoreScroll();
}, { flush: "post" });
const filters: { id: LibraryFilter; label: string }[] = [
  { id: "all", label: "全部書籍" },
  { id: "favorites", label: "我的收藏" },
  { id: "recent", label: "最近閱讀" },
  { id: "missing", label: "來源失效" },
];
const sortedBooks = computed(() => {
  if (!library.activeSeries) return sortBooks(library.visibleBooks, sort.value);
  const books = filterBooks(
    library.books.filter((b) => seriesKey(b.series) === library.activeSeries),
    library.seriesQuery,
    library.filter,
    library.statusFilter,
    library.tagFilter,
  );
  return sort.value === "series"
    ? sortVolumes(books)
    : sortBooks(books, sort.value);
});
const shownBooks = computed(() => sortedBooks.value.slice(0, limit.value));
// Selection survives appearance/filter changes; only removed records are dropped.
watch(() => [library.query, library.sort, library.filter, library.statusFilter, library.tagFilter], () => {
  library.bookLimit = 60;
  library.seriesLimit = 60;
});
watch(() => [library.seriesQuery, library.seriesSort, library.filter, library.statusFilter, library.tagFilter], () => {
  library.seriesBookLimit = 60;
});
watch(
  () => library.books,
  () => {
    selectedIds.value = selectedIds.value.filter((id) =>
      library.books.some((book) => book.id === id),
    );
  },
);
async function addFiles() {
  if (busy.value) return;
  try {
    const paths = await pickBookFiles();
    if (paths?.length) await library.add(paths);
  } catch (e) {
    library.error = String(e);
  }
}
async function addFolder() {
  if (busy.value) return;
  try {
    const path = await pickFolder();
    if (path) await library.add([path]);
  } catch (e) {
    library.error = String(e);
  }
}
async function openBook(id: number) {
  if (busy.value) return;
  if (await reader.openBook(id)) library.screen = "reader";
}
function selectBook(id: number) {
  if (busy.value) return;
  selectedIds.value = selectedIds.value.includes(id)
    ? selectedIds.value.filter((x) => x !== id)
    : [...selectedIds.value, id];
}
function clearFilters() {
  library.query = "";
  library.seriesQuery = "";
  library.filter = "all";
  library.statusFilter = "all";
  library.tagFilter = null;
}
const resultLabels = {
  added: "新增",
  updated: "更新",
  failed: "失敗",
  conflict: "來源衝突",
  canceled: "未開始／已取消",
};
</script>
<template>
  <main ref="layout" class="library-layout" @scroll="recordScroll">
    <button
      class="navigation-toggle"
      :aria-expanded="navigationOpen"
      @click="navigationOpen = !navigationOpen"
    >
      書庫導覽與設定
    </button>
    <aside
      class="sidebar"
      :class="{ expanded: navigationOpen }"
      aria-label="書庫導覽與設定"
    >
      <div class="brand">
        <PhBooks :size="26" aria-hidden="true" /> MangaFolio<span
          >你的私人漫畫書庫</span
        >
      </div>
      <nav aria-label="書庫篩選">
        <button
          :aria-pressed="library.section === 'series'"
          :disabled="busy"
          @click="
            library.section = 'series';
            library.activeSeries = null;
          "
        >
          系列書架<span>{{ groups.length }}</span>
        </button>
        <button
          v-for="filter in filters"
          :key="filter.id"
          :aria-pressed="
            library.filter === filter.id &&
            library.section === 'books' &&
            !library.activeSeries
          "
          :class="{
            active:
              library.filter === filter.id &&
              library.section === 'books' &&
              !library.activeSeries,
          }"
          :disabled="library.managing"
          @click="
            library.filter = filter.id;
            library.section = 'books';
            library.activeSeries = null;
          "
        >
          <span class="nav-label"
            ><component
              :is="
                filter.id === 'all'
                  ? PhBooks
                  : filter.id === 'favorites'
                    ? PhStar
                    : filter.id === 'recent'
                      ? PhClockCounterClockwise
                      : PhFolderNotchOpen
              "
              :size="18"
              aria-hidden="true"
            />{{ filter.label }}</span
          ><span>{{
            filter.id === "all"
              ? library.books.length
              : filter.id === "favorites"
                ? library.favoriteCount
                : filter.id === "recent"
                  ? library.recentCount
                  : library.books.filter((b) => !b.available).length
          }}</span>
        </button>
      </nav>
      <div
        class="sidebar-section"
        :class="{ 'guide-highlight': guideTarget === 'status' }"
      >
        <label
          >閱讀狀態<select
            aria-label="閱讀狀態"
            v-model="library.statusFilter"
            :disabled="library.managing"
          >
            <option value="all">全部狀態</option>
            <option value="unread">未讀</option>
            <option value="reading">閱讀中</option>
            <option value="read">已讀</option>
          </select></label
        >
        <label
          >標籤篩選<select
            aria-label="標籤篩選"
            v-model="library.tagFilter"
            :disabled="library.managing"
          >
            <option :value="null">全部標籤</option>
            <option v-for="tag in library.tags" :key="tag.id" :value="tag.id">
              {{ tag.name }}
            </option>
          </select></label
        >
        <TagManager />
      </div>
      <div
        class="sidebar-section"
        :class="{ 'guide-highlight': guideTarget === 'appearance' }"
      >
        <AppearanceSettings />
      </div>
      <p class="local-note">本機資料 · 無需帳號<br />原始漫畫檔案留在原位</p>
    </aside>
    <div
      ref="content"
      class="library-content"
      :class="{ 'detail-open': detailBook }"
      @scroll="recordScroll"
    >
      <header class="library-header">
        <div>
          <p class="eyebrow">
            {{ managementOpen ? "整理你的收藏" : "收藏好故事，隨時接著讀" }}
          </p>
          <h1>
            {{
              activeGroup?.name ??
              (library.section === "series"
                ? "系列書架"
                : filters.find((f) => f.id === library.filter)?.label)
            }}
          </h1>
        </div>
        <div
          class="actions"
          :class="{ 'guide-highlight': guideTarget === 'import' }"
        >
          <button class="primary" :disabled="busy" @click="addFiles">
            {{ library.importing ? "加入中…" : "加入 ZIP／CBZ" }}</button
          ><button :disabled="busy" @click="addFolder">加入圖片資料夾</button
          ><button
            v-if="reader.hasBook"
            :disabled="busy"
            @click="library.screen = 'reader'"
          >
            返回閱讀</button
          ><button
            :disabled="busy"
            :aria-expanded="managementOpen"
            @click="managementOpen = !managementOpen"
          >
            {{ managementOpen ? "結束管理" : "管理與備份" }}
          </button>
        </div>
      </header>
      <div
        class="library-controls"
        :class="{ 'guide-highlight': guideTarget === 'search' }"
      >
        <label class="search"
          ><span class="sr-only">搜尋書庫</span
          ><input
            v-model="query"
            type="search"
            :placeholder="
              library.activeSeries
                ? '搜尋此系列…'
                : '搜尋書名、系列、標籤或備註…'
            "
            aria-label="搜尋書名"
            :disabled="library.managing"
        /></label>
        <select
          v-model="sort"
          aria-label="書籍排序"
          :disabled="library.managing"
        >
          <option value="recent">最近閱讀／加入</option>
          <option value="title">書名排序</option>
          <option value="series">系列／集數排序</option>
        </select>
        <div class="view-switch" role="group" aria-label="書庫檢視切換">
          <button
            v-for="view in ['grid', 'detail', 'compact'] as const"
            :key="view"
            :aria-pressed="appearance.settings.view === view"
            @click="appearance.set('view', view)"
          >
            {{
              view === "grid"
                ? "封面網格"
                : view === "detail"
                  ? "詳細列表"
                  : "緊湊列表"
            }}
          </button>
        </div>
      </div>
      <section v-if="library.activeSeries" class="series-heading">
        <button :disabled="busy" @click="backSeries">返回書架</button>
        <template v-if="activeGroup"
          ><BookCover
            :key="activeGroup.cover.id"
            :id="activeGroup.cover.id"
            :available="activeGroup.cover.available"
            :title="activeGroup.name"
          />
          <div>
            <h2>{{ activeGroup.name }}</h2>
            <p>
              {{ activeGroup.books.length }} 冊 · 已讀
              {{ activeGroup.read }} 冊（{{ activeGroup.percent }}%）
            </p>
            <p>
              頁面平均 {{ activeGroup.pagePercent }}% ·
              {{ activeGroup.missing }} 冊來源失效
            </p>
            <button
              v-if="activeGroup.resume"
              class="primary"
              :disabled="busy"
              @click="openBook(activeGroup.resume.id)"
            >
              繼續閱讀
              {{
                activeGroup.resume.volume
                  ? `第 ${activeGroup.resume.volume} 集`
                  : activeGroup.resume.title
              }}
            </button>
            <details class="series-management-disclosure">
              <summary>整理系列</summary>
              <div class="series-management">
                <label
                  >系列名稱<input
                    v-model="renamedSeries"
                    maxlength="256"
                    :disabled="busy" /></label
                ><button :disabled="busy" @click="renameSeries()">
                  重新命名系列</button
                ><button :disabled="busy" @click="renameSeries(true)">
                  移除系列歸屬
                </button>
              </div>
            </details>
          </div></template
        >
        <p v-else>系列已改名或移除歸屬，請返回書架。</p>
      </section>
      <LibraryManager
        v-if="managementOpen"
        :selected-ids="selectedIds"
        :shown-count="shownBooks.length"
        @clear="selectedIds = []"
        @select-shown="
          selectedIds = Array.from(
            new Set([...selectedIds, ...shownBooks.map((b) => b.id)]),
          )
        "
      />
      <p v-if="library.error" class="notice error" role="alert">
        {{ library.error }}
        <button :disabled="busy" @click="library.refresh">重新載入書庫</button>
      </p>
      <p v-if="reader.error" class="notice error" role="alert">
        {{ reader.error }}
      </p>
      <p v-if="library.importNotice" class="notice" role="status">
        {{ library.importNotice }}
      </p>
      <section
        v-if="library.importing || library.importProgress.results.length"
        class="import-results"
        aria-label="匯入結果"
      >
        <div class="actions">
          <strong
            >批次匯入 {{ library.importProgress.completed }}／{{
              library.importProgress.total
            }}</strong
          ><button
            v-if="library.importing"
            :disabled="library.importCanceled"
            @click="library.cancelImport"
          >
            {{
              library.importCanceled ? "正在停止未開始項目…" : "取消剩餘匯入"
            }}</button
          ><button
            v-else-if="
              library.importProgress.results.some((r) =>
                ['failed', 'conflict', 'canceled'].includes(r.kind),
              )
            "
            :disabled="busy"
            @click="library.retryImport"
          >
            重試失敗與未開始項目
          </button>
        </div>
        <p v-if="library.importing" role="status">
          正在處理：{{ library.importProgress.currentPath }}
        </p>
        <details>
          <summary>逐筆結果</summary>
          <ul>
            <li
              v-for="(result, index) in library.importProgress.results"
              :key="index"
            >
              <strong>{{ resultLabels[result.kind] }}</strong> · {{ result.path
              }}<span v-if="result.message"> · {{ result.message }}</span>
            </li>
          </ul>
        </details>
      </section>
      <ContinueReading
        v-if="
          library.continueBook &&
          library.filter === 'all' &&
          !library.query.trim() &&
          library.statusFilter === 'all' &&
          library.tagFilter === null &&
          !managementOpen &&
          !library.activeSeries
        "
        :key="`${library.continueBook?.id}:${library.revision}`"
        :book="library.continueBook!"
        :busy="busy"
        @open="openBook"
      />
      <section
        v-if="!library.activeSeries && !managementOpen && library.books.length"
        class="home-series"
      >
        <h2>{{ library.section === "series" ? "全部系列" : "我的系列" }}</h2>
        <SeriesShelf
          :groups="
            library.section === 'series'
              ? visibleGroups.slice(0, seriesLimit)
              : visibleGroups.slice(0, 4)
          "
          :busy="busy"
          @open="enterSeries"
        />
        <button
          v-if="
            library.section === 'series' && seriesLimit < visibleGroups.length
          "
          :disabled="busy"
          @click="seriesLimit += 60"
        >
          顯示更多系列
        </button>
      </section>
      <p v-if="library.filter === 'missing'" class="notice">
        來源可能暫時離線或已移動。資料會保留；進入管理、選取一本後重新指定同一本漫畫來源。
      </p>
      <p v-if="library.loading" class="status" role="status">正在載入書庫…</p>
      <section v-else-if="!library.books.length" class="empty">
        <h2>把第一本漫畫加入書庫</h2>
        <p>支援 ZIP、CBZ 與直接包含圖片的資料夾。收藏與進度儲存在這台電腦。</p>
        <button class="primary" :disabled="busy" @click="addFiles">
          加入漫畫
        </button>
      </section>
      <section v-else-if="!sortedBooks.length" class="empty">
        <h2>沒有符合搜尋與篩選的書籍</h2>
        <p>試試其他關鍵字或清除條件。</p>
        <button @click="clearFilters">清除搜尋與篩選</button>
      </section>
      <template
        v-else-if="
          library.section !== 'series' || library.activeSeries || managementOpen
        "
        ><p class="results" aria-live="polite">
          {{ sortedBooks.length }} 本書{{ reader.loading ? " · 正在開啟…" : ""
          }}{{
            managementOpen
              ? ` · 已選 ${selectedIds.length} 本（包含其他篩選中的選取）`
              : ""
          }}
        </p>
        <div
          v-if="appearance.settings.view === 'detail' && !managementOpen"
          class="detail-heading"
          aria-hidden="true"
        >
          <span>封面</span>
          <div>
            <span>書籍</span><span>系列／標籤</span><span>閱讀狀態／進度</span
            ><span>操作</span>
          </div>
        </div>
        <section
          class="book-collection"
          :class="appearance.settings.view"
          aria-label="書籍"
        >
          <BookCard
            v-for="book in shownBooks"
            :key="`${book.id}:${library.revision}`"
            :book="book"
            :data-book-id="book.id"
            :view="appearance.settings.view"
            :opening="busy"
            :favorite-pending="busy || library.favoritePending.has(book.id)"
            :selectable="managementOpen"
            :selected="selectedIds.includes(book.id)"
            :focused="detailBook?.id === book.id"
            @open="openBook"
            @favorite="library.toggleFavorite"
            @select="selectBook"
            @edit="detailBook = $event"
          />
        </section>
        <button
          v-if="limit < sortedBooks.length"
          class="load-more"
          :disabled="busy"
          @click="limit += 60"
        >
          顯示更多書籍
        </button>
      </template>
      <footer>
        <LibraryGuide @highlight="guideTarget = $event" />
        <p>閱讀狀態與續讀位置分開保存 · 來源失效不會自動移除書籍</p>
      </footer>
    </div>
    <BookDetailsPanel
      v-if="detailBook"
      :book="detailBook"
      @close="detailBook = null"
      @edit="
        editingBook = $event;
        detailBook = null;
      "
      @manage="
        selectedIds = [$event.id];
        managementOpen = true;
        detailBook = null;
      "
    />
    <BookEditor
      v-if="editingBook"
      :book="editingBook"
      @close="editingBook = null"
    />
  </main>
</template>
<style scoped>
@media (min-width: 1100px) {
  .library-content.detail-open {
    padding-right: calc(var(--detail-width) + 24px);
  }
}
.series-management {
  display: flex;
  align-items: end;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 14px;
}
.series-management-disclosure {
  margin-top: 14px;
}
.series-management-disclosure summary {
  cursor: pointer;
  font-size: 12px;
  color: var(--accent-soft);
}
.series-management label {
  display: grid;
  gap: 6px;
  font-size: 12px;
}
.series-heading {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 24px;
  padding: 24px 0;
  border-bottom: 1px solid var(--line);
  margin-bottom: 24px;
}
.series-heading :deep(.book-cover) {
  width: 100px;
  flex: 0 0 100px;
}
.series-heading p {
  font-size: 13px;
  color: var(--text-dim);
  margin: 8px 0;
}
.home-series {
  margin: 28px 0;
}
.home-series > h2 {
  font-family: var(--heading-font);
  font-size: 24px;
  margin-bottom: 20px;
}
.library-layout {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}
.sidebar {
  flex: 0 0 var(--sidebar-width);
  overflow: auto;
  padding: 28px 20px;
  background: var(--bg-soft);
  border-right: 1px solid var(--line);
}
.brand {
  font-size: 24px;
  font-family: var(--heading-font);
  font-weight: 700;
  color: var(--accent-soft);
  letter-spacing: 0.3px;
}
.brand span {
  display: block;
  font-size: 11px;
  color: var(--text-dim);
  font-weight: 400;
  margin-top: 8px;
}
.sidebar nav {
  display: grid;
  gap: 6px;
  margin: 30px 0;
}
.sidebar nav button {
  text-align: left;
  background: transparent;
  display: flex;
  justify-content: space-between;
  border-color: transparent;
  padding: 12px;
}
.sidebar nav button.active {
  background: var(--panel);
  border-color: transparent;
  box-shadow: var(--surface-shadow);
  color: var(--accent-soft);
  font-weight: 600;
}
.sidebar nav .nav-label {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
}
.sidebar nav span {
  font-size: 11px;
}
.sidebar-section {
  border-top: 1px solid var(--line);
  padding-top: 12px;
  margin-top: 12px;
}
.sidebar-section > label {
  display: grid;
  gap: 6px;
  margin-bottom: 12px;
  font-size: 12px;
}
.sidebar select {
  width: 100%;
}
.local-note {
  font-size: 11px;
  color: var(--text-dim);
  line-height: 1.8;
  margin-top: 24px;
}
.library-content {
  flex: 1;
  min-width: 0;
  overflow: auto;
  padding: var(--content-padding);
}
.library-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space);
  margin-bottom: 24px;
}
h1 {
  font-family: var(--heading-font);
  font-size: 32px;
  margin-top: 6px;
}
h2 {
  font-size: 18px;
  overflow-wrap: anywhere;
}
.eyebrow {
  font-size: 12px;
  color: var(--accent-soft);
}
.actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}
.library-controls {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  align-items: center;
  margin-bottom: var(--space);
}
.search {
  flex: 1;
  min-width: 180px;
}
.search input {
  width: 100%;
}
.view-switch {
  display: flex;
  gap: 2px;
  background: var(--bg-soft);
  padding: 4px;
  border-radius: 10px;
}
.view-switch button {
  padding: 8px 12px;
  border-color: transparent;
  background: transparent;
  font-size: 12px;
}
.view-switch button[aria-pressed="true"] {
  border-color: var(--accent);
  color: var(--accent-soft);
  background: var(--panel);
  box-shadow: var(--surface-shadow);
}
.detail-heading {
  display: flex;
  gap: var(--space);
  padding: 12px;
  border-top: 1px solid var(--line);
  color: var(--text-dim);
  font-size: 11px;
}
.detail-heading > span {
  width: 76px;
  flex: 0 0 76px;
}
.detail-heading > div {
  flex: 1;
  display: grid;
  grid-template-columns:
    minmax(130px, 1.4fr) minmax(90px, 1fr) minmax(120px, 1fr)
    68px;
  gap: 20px;
  padding: 0 4px;
}
@media (max-width: 1100px) {
  .detail-heading {
    display: none;
  }
}
.book-collection {
  display: grid;
  gap: var(--space);
}
.book-collection.grid {
  grid-template-columns: repeat(
    auto-fill,
    minmax(min(var(--cover-min), 100%), 1fr)
  );
}
.book-collection.detail,
.book-collection.compact {
  grid-template-columns: 1fr;
  gap: 0;
  border-top: 1px solid var(--line);
}
.book-collection.compact {
  gap: 6px;
}
.results {
  font-size: 12px;
  color: var(--text-dim);
  margin-bottom: 14px;
}
.notice,
.import-results {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-soft);
  padding: 12px 16px;
  margin-bottom: var(--space);
  font-size: 13px;
  line-height: 1.7;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}
.error {
  color: var(--red);
}
.notice button {
  margin-left: 8px;
}
.import-results summary {
  cursor: pointer;
  margin-top: 8px;
}
.import-results ul {
  padding-left: 20px;
  max-height: 200px;
  overflow: auto;
}
.empty {
  border: 1px dashed var(--border);
  border-radius: var(--radius);
  padding: 48px 20px;
  text-align: center;
}
.empty p {
  margin: 12px 0 20px;
  color: var(--text-dim);
  font-size: 13px;
  line-height: 1.8;
}
.status {
  text-align: center;
  padding: 30px;
}
.load-more {
  display: block;
  margin: 24px auto;
}
footer {
  color: var(--text-dim);
  font-size: 11px;
  text-align: center;
  margin: 28px 0 0;
}
.navigation-toggle {
  display: none;
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
}
.guide-highlight {
  outline: 2px solid var(--accent);
  outline-offset: 3px;
}
@media (max-width: 900px) {
  .library-header {
    align-items: flex-start;
    flex-direction: column;
  }
  .library-content {
    padding: 20px;
  }
  .sidebar {
    flex-basis: 200px;
    padding: 20px 14px;
  }
}
@media (max-width: 700px) {
  .library-layout {
    display: block;
    overflow: auto;
  }
  .navigation-toggle {
    display: block;
    margin: 12px;
    width: calc(100% - 24px);
  }
  .sidebar {
    display: none;
    border-right: 0;
    border-bottom: 1px solid var(--border);
  }
  .sidebar.expanded {
    display: block;
  }
  .sidebar nav {
    display: flex;
    flex-wrap: wrap;
    margin: 16px 0;
  }
  .sidebar-section {
    max-width: 100%;
  }
  .library-content {
    overflow: visible;
    padding: 16px;
  }
  .library-header {
    margin-bottom: 16px;
  }
  h1 {
    font-size: 24px;
  }
  .view-switch {
    flex-wrap: wrap;
  }
  .continue-panel {
    padding: 14px;
  }
  .search {
    min-width: 0;
    flex-basis: 100%;
  }
  .library-controls select {
    max-width: 100%;
  }
  .actions button {
    white-space: normal;
  }
}
</style>
