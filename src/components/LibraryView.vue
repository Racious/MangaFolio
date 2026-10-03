<script setup lang="ts">
import { computed, ref, watch } from "vue";
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
import { pickFolder } from "../api/backend";
const library = useLibraryStore(),
  reader = useReaderStore(),
  appearance = useAppearanceStore();
const limit = ref(60),
  guideTarget = ref(""),
  managementOpen = ref(false),
  navigationOpen = ref(false);
const selectedIds = ref<number[]>([]),
  editingBook = ref<LibraryBook | null>(null);
const busy = computed(
  () =>
    library.managing ||
    library.importing ||
    library.loading ||
    reader.loading ||
    reader.favoritePending ||
    library.favoritePending.size > 0,
);
const sort = ref<"recent" | "title" | "series">("recent");
const filters: { id: LibraryFilter; label: string }[] = [
  { id: "all", label: "全部書籍" },
  { id: "favorites", label: "我的收藏" },
  { id: "recent", label: "最近閱讀" },
  { id: "missing", label: "來源失效" },
];
const sortedBooks = computed(() => sortBooks(library.visibleBooks, sort.value));
const shownBooks = computed(() => sortedBooks.value.slice(0, limit.value));
// Selection survives appearance/filter changes; only removed records are dropped.
watch(
  () => [
    library.query,
    library.filter,
    library.statusFilter,
    library.tagFilter,
    sort.value,
  ],
  () => {
    limit.value = 60;
  },
);
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
  <main class="library-layout">
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
      <div class="brand">MangaFolio<span>你的私人漫畫書庫</span></div>
      <nav aria-label="書庫篩選">
        <button
          v-for="filter in filters"
          :key="filter.id"
          :aria-pressed="library.filter === filter.id"
          :class="{ active: library.filter === filter.id }"
          :disabled="library.managing"
          @click="library.filter = filter.id"
        >
          {{ filter.label
          }}<span>{{
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
    <div class="library-content">
      <header class="library-header">
        <div>
          <p class="eyebrow">
            {{ managementOpen ? "整理你的收藏" : "收藏好故事，隨時接著讀" }}
          </p>
          <h1>{{ filters.find((f) => f.id === library.filter)?.label }}</h1>
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
            v-model="library.query"
            type="search"
            placeholder="搜尋書名、系列、標籤或備註…"
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
      <LibraryGuide @highlight="guideTarget = $event" />
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
      <section
        v-if="
          library.continueBook &&
          library.filter === 'all' &&
          !library.query.trim() &&
          library.statusFilter === 'all' &&
          library.tagFilter === null &&
          !managementOpen
        "
        class="continue-panel"
      >
        <div>
          <p class="eyebrow">繼續閱讀</p>
          <h2>{{ library.continueBook.title }}</h2>
          <p>
            第 {{ library.continueBook.lastIndex + 1 }}／{{
              library.continueBook.pageCount
            }}
            頁
          </p>
        </div>
        <button
          class="primary"
          :disabled="busy"
          @click="openBook(library.continueBook.id)"
        >
          接著讀
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
      <template v-else
        ><p class="results" aria-live="polite">
          {{ sortedBooks.length }} 本書{{ reader.loading ? " · 正在開啟…" : ""
          }}{{
            managementOpen
              ? ` · 已選 ${selectedIds.length} 本（包含其他篩選中的選取）`
              : ""
          }}
        </p>
        <section
          class="book-collection"
          :class="appearance.settings.view"
          aria-label="書籍"
        >
          <BookCard
            v-for="book in shownBooks"
            :key="`${book.id}:${library.revision}`"
            :book="book"
            :view="appearance.settings.view"
            :opening="busy"
            :favorite-pending="busy || library.favoritePending.has(book.id)"
            :selectable="managementOpen"
            :selected="selectedIds.includes(book.id)"
            @open="openBook"
            @favorite="library.toggleFavorite"
            @select="selectBook"
            @edit="editingBook = $event"
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
      <footer>閱讀狀態與續讀位置分開保存 · 來源失效不會自動移除書籍</footer>
    </div>
    <BookEditor
      v-if="editingBook"
      :book="editingBook"
      @close="editingBook = null"
    />
  </main>
</template>
<style scoped>
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
  border-right: 1px solid var(--border);
}
.brand {
  font-size: 21px;
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
  border-color: var(--border);
  color: var(--accent-soft);
  font-weight: 600;
}
.sidebar nav span {
  font-size: 11px;
}
.sidebar-section {
  border-top: 1px solid var(--border);
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
  font-size: 28px;
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
  gap: 4px;
}
.view-switch button {
  padding: 8px;
  font-size: 12px;
}
.view-switch button[aria-pressed="true"] {
  border-color: var(--accent);
  color: var(--accent-soft);
  background: var(--bg-soft);
}
.continue-panel {
  border: 1px solid var(--border);
  border-left: 4px solid var(--accent);
  border-radius: var(--radius);
  background: var(--bg-soft);
  padding: 16px 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: var(--space);
}
.continue-panel div {
  min-width: 0;
}
.continue-panel h2 {
  margin: 5px 0;
  font-size: 18px;
}
.continue-panel p:last-child {
  font-size: 12px;
  color: var(--text-dim);
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
