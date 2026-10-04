<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from "vue";
import BookCover from "./BookCover.vue";
import BookmarksPanel from "./BookmarksPanel.vue";
import type { LibraryBook } from "../api/library";
import { useLibraryStore } from "../stores/library";
import { useReaderStore } from "../stores/reader";
import { progressPercent, statusLabels, displayPath } from "../lib/library";
import { nextVolume } from "../lib/series";
const props = defineProps<{ book: LibraryBook }>();
const emit = defineEmits<{
  close: [];
  edit: [book: LibraryBook];
  manage: [book: LibraryBook];
}>();
const dialog = ref<HTMLDialogElement>(),
  library = useLibraryStore(),
  reader = useReaderStore();
const book = computed(
  () => library.books.find((b) => b.id === props.book.id) ?? props.book,
);
const next = computed(() => nextVolume(book.value, library.books));
const busy = computed(
  () =>
    library.managing ||
    library.importing ||
    library.loading ||
    reader.loading ||
    reader.favoritePending ||
    library.favoritePending.size > 0,
);
let focus: HTMLElement | null = null;
const wideScreen = window.matchMedia("(min-width: 1100px)");
const modal = ref(!wideScreen.matches);
function updateMode() {
  const panel = dialog.value;
  if (!panel) return;
  const active = document.activeElement as HTMLElement | null;
  const switching = panel.open;
  panel.close();
  modal.value = !wideScreen.matches;
  if (modal.value) panel.showModal();
  else panel.show();
  if (switching && active?.isConnected) active.focus({ preventScroll: true });
}
function escape(event: KeyboardEvent) {
  if (modal.value || event.key !== "Escape" || busy.value) return;
  // An editor or native modal owns Escape while it overlays the sidebar.
  if (document.querySelector("dialog:modal")) return;
  event.preventDefault();
  emit("close");
}
watch(() => book.value.id, () => {
  if (dialog.value) dialog.value.scrollTop = 0;
}, { flush: "post" });
onMounted(() => {
  focus = document.activeElement as HTMLElement;
  updateMode();
  wideScreen.addEventListener("change", updateMode);
  window.addEventListener("keydown", escape);
});
onUnmounted(() => {
  wideScreen.removeEventListener("change", updateMode);
  window.removeEventListener("keydown", escape);
  if (focus?.isConnected) focus.focus({ preventScroll: true });
});
async function read(id: number) {
  if (busy.value) return;
  if (await reader.openBook(id)) {
    library.screen = "reader";
    emit("close");
  }
}
</script>
<template>
  <dialog
    ref="dialog"
    class="detail-panel"
    role="dialog"
    :aria-modal="modal"
    aria-labelledby="book-detail-title"
    @cancel="busy ? $event.preventDefault() : emit('close')"
  >
    <header>
      <span>書籍詳情</span
      ><button
        autofocus
        :disabled="busy"
        @click="emit('close')"
        aria-label="關閉書籍詳情"
      >
        關閉
      </button>
    </header>
    <BookCover
      :key="book.id"
      :id="book.id"
      :available="book.available"
      :title="book.title"
    />
    <h2 id="book-detail-title">{{ book.title }}</h2>
    <dl>
      <dt>系列／集數</dt>
      <dd>
        {{ book.series || "未分系列"
        }}{{ book.volume ? ` · 第 ${book.volume} 集` : "" }}
      </dd>
      <dt>閱讀狀態</dt>
      <dd>
        {{ statusLabels[book.readingStatus]
        }}{{ book.statusManual ? "（手動）" : "" }}
      </dd>
      <dt>頁面進度</dt>
      <dd>
        {{ progressPercent(book) }}% ·
        {{ book.lastReadAt ? book.lastIndex + 1 : 0 }}／{{ book.pageCount }} 頁
      </dd>
      <dt>標籤</dt>
      <dd>{{ book.tags.map((t) => t.name).join("、") || "無標籤" }}</dd>
      <dt>來源</dt>
      <dd>
        {{ book.available ? "可存取" : "來源失效；資料保留"
        }}<span class="path">{{ displayPath(book.path) }}</span>
      </dd>
    </dl>
    <p class="note">{{ book.notes || "尚無備註。" }}</p>
    <div class="panel-actions">
      <button
        class="primary"
        :disabled="busy || !book.available"
        @click="read(book.id)"
      >
        閱讀</button
      ><button
        :disabled="busy"
        :aria-pressed="book.favorite"
        @click="library.toggleFavorite(book)"
      >
        {{ book.favorite ? "取消收藏" : "收藏" }}</button
      ><button :disabled="busy" @click="emit('edit', book)">編輯資訊</button
      ><button :disabled="busy" @click="emit('manage', book)">
        重新連結／管理
      </button>
    </div>
    <section class="next-volume">
      <h3>下一集</h3>
      <p>{{ next.reason }}</p>
      <button
        v-if="next.book"
        :disabled="busy || !next.book.available"
        @click="read(next.book.id)"
      >
        {{ next.book.title }}
      </button>
    </section>
    <BookmarksPanel :key="book.id" :book-id="book.id" @opened="emit('close')" />
  </dialog>
</template>
<style scoped>
.detail-panel {
  position: fixed;
  inset: 0 0 0 auto;
  margin: 0;
  width: min(var(--detail-width), 100vw);
  height: 100%;
  max-height: 100%;
  max-width: 100%;
  border: 0;
  border-left: 1px solid var(--line);
  background: var(--panel);
  color: var(--text);
  padding: 24px;
  overflow: auto;
  z-index: 20;
}
header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
  font-size: 12px;
  color: var(--text-dim);
}
.detail-panel :deep(.book-cover) {
  width: 100%;
  max-width: 260px;
  margin: auto;
}
h2 {
  font-family: var(--heading-font);
  font-size: 26px;
  margin: 20px 0;
  overflow-wrap: anywhere;
}
dl {
  display: grid;
  grid-template-columns: 86px minmax(0, 1fr);
  gap: 12px;
  font-size: 13px;
  line-height: 1.6;
}
dt {
  color: var(--text-dim);
}
dd {
  margin: 0;
  overflow-wrap: anywhere;
}
.path {
  display: block;
  font-size: 11px;
  color: var(--text-dim);
}
.note {
  white-space: pre-wrap;
  line-height: 1.8;
  font-size: 13px;
  margin: 20px 0;
  overflow-wrap: anywhere;
}
.panel-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.next-volume {
  padding: 20px 0;
  border-block: 1px solid var(--line);
  margin: 24px 0;
}
.next-volume p {
  font-size: 12px;
  line-height: 1.7;
  color: var(--text-dim);
  margin: 10px 0;
}
.detail-panel::backdrop {
  background: var(--modal-backdrop);
}
@media (min-width: 1100px) {
  .detail-panel::backdrop {
    background: transparent;
  }
}
</style>
