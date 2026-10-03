<script setup lang="ts">
import { ref, computed, onUnmounted } from "vue";
import { useReaderStore } from "../stores/reader";
import { useLibraryStore } from "../stores/library";
import { useAppearanceStore } from "../stores/appearance";
import { nextVolume } from "../lib/series";
import BookmarksPanel from "./BookmarksPanel.vue";
const reader = useReaderStore(),
  library = useLibraryStore(),
  appearance = useAppearanceStore();
const dialog = ref<HTMLDialogElement>();
const current = computed(() =>
  library.books.find((b) => b.id === reader.bookId),
);
const next = computed(() =>
  current.value
    ? nextVolume(current.value, library.books)
    : { reason: "請返回書庫載入系列資訊。" },
);
const atEnd = computed(
  () => reader.hasBook && reader.viewIndices.includes(reader.pageCount - 1),
);
const busy = computed(
  () =>
    reader.loading ||
    library.managing ||
    library.importing ||
    library.loading ||
    reader.favoritePending ||
    library.favoritePending.size > 0,
);
let focus: HTMLElement | null = null;
function bookmarks() {
  if (busy.value) return;
  focus = document.activeElement as HTMLElement;
  dialog.value?.showModal();
}
function close() {
  if (busy.value) return;
  dialog.value?.close();
  if (focus?.isConnected) focus.focus();
}
async function openNext() {
  const book = next.value.book;
  if (!book || !book.available || busy.value) return;
  await reader.openBook(book.id);
}
onUnmounted(() => dialog.value?.close());
</script>
<template>
  <div class="reader-actions">
    <button :disabled="busy || !reader.bookId" @click="bookmarks">
      書籤／筆記
    </button>
    <label
      ><input
        type="checkbox"
        :checked="appearance.settings.readerPinned"
        @change="
          appearance.set(
            'readerPinned',
            ($event.target as HTMLInputElement).checked,
          )
        "
      />
      固定工具列</label
    >
    <template v-if="atEnd"
      ><button
        v-if="next.book"
        :disabled="busy || !next.book.available"
        @click="openNext"
      >
        閱讀下一集：{{ next.book.volume }}</button
      ><span>{{ next.reason }}</span></template
    >
    <Teleport to="body"
      ><dialog
        ref="dialog"
        role="dialog"
        aria-label="閱讀書籤與筆記"
        @cancel="busy ? $event.preventDefault() : close()"
      >
        <button autofocus :disabled="busy" @click="close">關閉書籤</button
        ><BookmarksPanel
          v-if="reader.bookId"
          :key="reader.bookId"
          :book-id="reader.bookId"
          :pages="reader.pages"
          :indices="reader.viewIndices"
          @opened="dialog?.close()"
        /></dialog
    ></Teleport>
  </div>
</template>
<style scoped>
.reader-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  padding: 8px 16px;
  background: var(--bg-soft);
  border-bottom: 1px solid var(--line);
  font-size: 12px;
}
.reader-actions label {
  display: flex;
  align-items: center;
  gap: 6px;
}
.reader-actions span {
  color: var(--text-dim);
  overflow-wrap: anywhere;
}
.reader-actions input {
  accent-color: var(--accent);
}
dialog {
  position: fixed;
  inset: 0;
  margin: auto;
  width: min(480px, calc(100vw - 32px));
  max-height: calc(100vh - 32px);
  padding: 24px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--panel);
  color: var(--text);
  overflow: auto;
}
dialog::backdrop {
  background: var(--modal-backdrop);
}
</style>
