<script setup lang="ts">
import { computed, ref, watch } from "vue";
import BookCard from "./BookCard.vue";
import LibraryGuide from "./LibraryGuide.vue";
import { useLibraryStore, type LibraryFilter } from "../stores/library";
import { useReaderStore } from "../stores/reader";
import { pickBookFiles } from "../api/library";
import { sortBooks } from "../lib/library";
import { pickFolder } from "../api/backend";

const library = useLibraryStore();
const reader = useReaderStore();
const limit = ref(60);
const guideTarget = ref("");
const sort = ref<"recent" | "title">("recent");
const filters: { id: LibraryFilter; label: string }[] = [
  { id: "all", label: "全部書籍" },
  { id: "favorites", label: "我的收藏" },
  { id: "recent", label: "最近閱讀" },
];
const sortedBooks = computed(() => sortBooks(library.visibleBooks, sort.value));
const shownBooks = computed(() => sortedBooks.value.slice(0, limit.value));
watch(
  () => [library.query, library.filter, sort.value],
  () => {
    limit.value = 60;
  },
);
async function addFiles() {
  try {
    const paths = await pickBookFiles();
    if (paths?.length) await library.add(paths);
  } catch (e) {
    library.error = String(e);
  }
}
async function addFolder() {
  try {
    const path = await pickFolder();
    if (path) await library.add([path]);
  } catch (e) {
    library.error = String(e);
  }
}
async function openBook(id: number) {
  if (await reader.openBook(id)) library.screen = "reader";
}
</script>

<template>
  <main class="library-view">
    <header class="library-header">
      <div>
        <div class="brand">MangaFolio<span>LIBRARY</span></div>
        <h1>你的漫畫書庫</h1>
        <p>收藏好故事，隨時接著讀。</p>
      </div>
      <div
        class="import-actions"
        :class="{ 'guide-highlight': guideTarget === 'import' }"
      >
        <button
          class="primary"
          :disabled="library.importing || reader.loading"
          @click="addFiles"
        >
          {{ library.importing ? "加入中…" : "＋ 加入 ZIP／CBZ" }}
        </button>
        <button
          :disabled="library.importing || reader.loading"
          @click="addFolder"
        >
          加入圖片資料夾
        </button>
        <button
          v-if="reader.hasBook"
          :disabled="reader.loading"
          @click="library.screen = 'reader'"
        >
          返回閱讀
        </button>
      </div>
    </header>
    <LibraryGuide @highlight="guideTarget = $event" />
    <p v-if="library.error" class="notice error" role="alert">
      {{ library.error }}
      <button :disabled="library.loading" @click="library.refresh">
        重新載入書庫
      </button>
    </p>
    <p v-if="reader.error" class="notice error" role="alert">
      {{ reader.error }}
    </p>
    <p v-if="library.importNotice" class="notice" role="status">
      {{ library.importNotice }}
    </p>
    <section
      v-if="
        library.continueBook &&
        library.filter === 'all' &&
        !library.query.trim()
      "
      class="continue-panel"
      :class="{ 'guide-highlight': guideTarget === 'recent' }"
      aria-labelledby="continue-title"
    >
      <div>
        <span class="eyebrow">上次讀到這裡</span>
        <h2 id="continue-title">{{ library.continueBook.title }}</h2>
        <p>
          第 {{ library.continueBook.lastIndex + 1 }} 頁 · 共
          {{ library.continueBook.pageCount }} 頁
        </p>
      </div>
      <button
        class="primary"
        :disabled="reader.loading"
        @click="openBook(library.continueBook.id)"
      >
        繼續閱讀 →
      </button>
    </section>
    <div class="library-controls">
      <nav aria-label="書庫篩選">
        <button
          v-for="filter in filters"
          :key="filter.id"
          :class="{ active: library.filter === filter.id }"
          :data-guide-highlight="
            (guideTarget === 'favorites' && filter.id === 'favorites') ||
            (guideTarget === 'recent' && filter.id === 'recent')
          "
          :aria-pressed="library.filter === filter.id"
          @click="library.filter = filter.id"
        >
          {{ filter.label
          }}<span>{{
            filter.id === "all"
              ? library.books.length
              : filter.id === "favorites"
                ? library.favoriteCount
                : library.recentCount
          }}</span>
        </button>
      </nav>
      <div
        class="search-controls"
        :class="{ 'guide-highlight': guideTarget === 'search' }"
      >
        <label class="search"
          ><span aria-hidden="true">⌕</span
          ><input
            v-model="library.query"
            type="search"
            placeholder="搜尋書名…"
            aria-label="搜尋書名" /></label
        ><select v-model="sort" aria-label="書籍排序">
          <option value="recent">最近閱讀／加入</option>
          <option value="title">書名排序</option>
        </select>
      </div>
    </div>
    <p v-if="library.loading" class="status" role="status">正在載入書庫…</p>
    <section
      v-else-if="!library.books.length"
      class="empty"
      :class="{ 'guide-highlight': guideTarget === 'books' }"
      aria-labelledby="empty-title"
    >
      <div aria-hidden="true">漫</div>
      <h2 id="empty-title">把你的第一本漫畫加入書庫</h2>
      <p>
        支援 ZIP、CBZ，以及直接包含圖片的資料夾。<br />原始檔案留在原位，閱讀進度與收藏保存在這台電腦。
      </p>
      <button class="primary" :disabled="library.importing" @click="addFiles">
        加入漫畫
      </button>
    </section>
    <section v-else-if="!sortedBooks.length" class="empty">
      <h2>
        {{
          library.query
            ? "找不到符合的書名"
            : library.filter === "favorites"
              ? "還沒有收藏的書籍"
              : "還沒有閱讀紀錄"
        }}
      </h2>
      <p>
        {{
          library.query
            ? "試試其他關鍵字，或清除搜尋。"
            : library.filter === "favorites"
              ? "點書籍旁的星號，把喜歡的作品留在這裡。"
              : "從書庫開啟一本漫畫，就會顯示在這裡。"
        }}
      </p>
      <button v-if="library.query" @click="library.query = ''">清除搜尋</button>
    </section>
    <template v-else
      ><p class="results" aria-live="polite">
        {{ sortedBooks.length }} 本書{{ reader.loading ? " · 正在開啟…" : "" }}
      </p>
      <section
        class="book-grid"
        :class="{
          'guide-highlight':
            guideTarget === 'books' || guideTarget === 'favorites',
        }"
        aria-label="書籍"
      >
        <BookCard
          v-for="book in shownBooks"
          :key="`${book.id}:${library.revision}`"
          :book="book"
          :opening="reader.loading"
          :favorite-pending="library.favoritePending.has(book.id)"
          @open="openBook"
          @favorite="library.toggleFavorite"
        />
      </section>
      <button
        v-if="limit < sortedBooks.length"
        class="load-more"
        @click="limit += 60"
      >
        顯示更多書籍
      </button></template
    >
    <footer>本機書庫 · 無需帳號 · 漫畫檔案留在原位</footer>
  </main>
</template>

<style scoped>
.guide-highlight,
[data-guide-highlight="true"] {
  outline: 2px solid var(--accent);
  outline-offset: 4px;
  border-radius: 8px;
}
.library-view {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 36px 40px 20px;
}
.library-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  margin-bottom: 28px;
}
.brand {
  color: var(--accent-soft);
  font-size: 16px;
  font-weight: 600;
  letter-spacing: 0.5px;
}
.brand span {
  margin-left: 12px;
  color: var(--text-dim);
  font-size: 10px;
  letter-spacing: 2px;
}
h1 {
  font-size: 28px;
  font-weight: 600;
  margin: 14px 0 6px;
}
p {
  color: var(--text-dim);
  font-size: 13px;
  line-height: 1.7;
}
button,
select {
  background: var(--panel);
  border: 1px solid var(--border);
  color: var(--text);
  padding: 10px 14px;
  border-radius: 8px;
  font: inherit;
  font-size: 13px;
  cursor: pointer;
  white-space: nowrap;
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
button.primary {
  background: var(--accent);
  color: #17191d;
  border-color: var(--accent);
  font-weight: 600;
}
.import-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}
.continue-panel {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  padding: 22px 24px;
  margin-bottom: 28px;
  border: 1px solid #c8a96a55;
  background: linear-gradient(110deg, #c8a96a12, var(--bg-soft));
  border-radius: 12px;
}
.continue-panel div {
  min-width: 0;
}
.continue-panel h2 {
  font-size: 19px;
  margin: 6px 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.eyebrow {
  color: var(--accent-soft);
  font-size: 11px;
  letter-spacing: 1px;
}
.library-controls {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}
nav {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
nav button {
  background: transparent;
  border-color: transparent;
}
nav button.active {
  border-color: var(--border);
  background: var(--panel);
  color: var(--accent-soft);
}
nav span {
  color: var(--text-dim);
  margin-left: 8px;
  font-size: 11px;
}
.search-controls {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.search {
  display: flex;
  gap: 8px;
  align-items: center;
  border: 1px solid var(--border);
  background: var(--bg-soft);
  border-radius: 8px;
  padding: 0 12px;
}
.search span {
  color: var(--text-dim);
  font-size: 24px;
}
input {
  width: 180px;
  border: 0;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  padding: 10px 0;
}
.book-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
  gap: 20px;
}
.results {
  font-size: 12px;
  margin: 0 0 14px;
}
.empty {
  padding: 65px 20px;
  text-align: center;
  border: 1px dashed var(--border);
  border-radius: 12px;
}
.empty > div {
  font-size: 72px;
  font-weight: bold;
  color: var(--border);
}
.empty h2 {
  margin: 14px 0 10px;
  font-size: 20px;
}
.empty button {
  margin-top: 22px;
}
.notice {
  padding: 10px 14px;
  margin-bottom: 16px;
  border-radius: 8px;
  background: var(--bg-soft);
  white-space: pre-wrap;
}
.notice.error {
  color: var(--red);
  background: #d98a8a12;
}
.notice button {
  margin-left: 10px;
}
.status {
  padding: 30px;
  text-align: center;
}
.load-more {
  display: block;
  margin: 24px auto;
}
footer {
  margin: 30px 0 10px;
  font-size: 11px;
  color: var(--text-dim);
  text-align: center;
}
@media (max-width: 800px) {
  .library-view {
    padding: 24px 20px 16px;
  }
  .library-header {
    align-items: flex-start;
    flex-direction: column;
    gap: 16px;
  }
  .import-actions {
    justify-content: flex-start;
  }
  .search-controls {
    width: 100%;
  }
  .search {
    flex: 1;
  }
  input {
    width: 100%;
    min-width: 0;
  }
  .book-grid {
    gap: 14px;
  }
}
</style>
