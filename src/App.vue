<script setup lang="ts">
import { onMounted, onUnmounted, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import LibraryView from "./components/LibraryView.vue";
import Toolbar from "./components/Toolbar.vue";
import ReaderView from "./components/ReaderView.vue";
import PageScrubber from "./components/PageScrubber.vue";
import UpdateDialog from "./components/UpdateDialog.vue";
import { useLibraryStore } from "./stores/library";
import { useReaderStore } from "./stores/reader";
import { useUpdateStore } from "./stores/update";

const library = useLibraryStore();
const reader = useReaderStore();
const update = useUpdateStore();
let unlistenClose: (() => void) | undefined;
let disposed = false;
let closing = false;
watch(
  () => [
    reader.bookId,
    reader.index,
    reader.direction,
    reader.pageMode,
    reader.zoom,
    reader.fixedScale,
    reader.doubleCover,
    reader.transition,
  ],
  () => {
    if (!reader.loading && reader.hasBook) reader.scheduleProgress();
  },
);
async function returnToLibrary() {
  if (reader.loading) return;
  try {
    await reader.flushProgress();
    library.screen = "library";
    await library.refresh();
  } catch {
    /* Keep the reader open so an unsaved position can be retried. */
  }
}
function retrySave() {
  void reader.flushProgress().catch(() => {});
}
onMounted(async () => {
  void library.refresh();
  void update.checkForUpdates({ silent: true });
  try {
    const window = getCurrentWindow();
    const unlisten = await window.onCloseRequested(async (event) => {
      event.preventDefault();
      if (library.managing) {
        library.error = "書庫操作進行中，請完成後再關閉視窗。";
        return;
      }
      if (closing) return;
      closing = true;
      try {
        await reader.flushProgress();
        await window.destroy();
      } catch (e) {
        reader.progressError = `無法安全關閉，請重試儲存：${String(e)}`;
        closing = false;
      }
    });
    if (disposed) unlisten();
    else unlistenClose = unlisten;
  } catch (e) {
    reader.progressError = `無法註冊關閉時儲存：${String(e)}`;
  }
});
onUnmounted(() => {
  disposed = true;
  unlistenClose?.();
});
</script>

<template>
  <div class="app">
    <LibraryView v-if="library.screen === 'library'" />
    <template v-else>
      <Toolbar @library="returnToLibrary" />
      <ReaderView />
      <PageScrubber />
      <p v-if="reader.error" class="error-bar" role="alert">
        {{ reader.error }}
      </p>
    </template>
    <p v-if="reader.progressError" class="error-bar" role="alert">
      {{ reader.progressError }} <button @click="retrySave">重試儲存</button>
    </p>
    <p
      v-else-if="library.screen === 'reader' && reader.hasBook"
      class="save-status"
      role="status"
    >
      {{
        reader.savingProgress ? "正在儲存閱讀進度…" : "閱讀進度自動儲存在本機"
      }}
    </p>
    <UpdateDialog />
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
}
.error-bar {
  flex: 0 0 auto;
  background: rgba(217, 138, 138, 0.12);
  color: var(--red);
  border-top: 1px solid var(--border);
  padding: 8px 14px;
  font-size: 13px;
  margin: 0;
}
.error-bar button {
  color: var(--text);
  background: var(--panel);
  border: 1px solid var(--border);
  padding: 4px 8px;
  margin-left: 8px;
  cursor: pointer;
  border-radius: 4px;
}
.save-status {
  flex: 0 0 auto;
  color: var(--text-dim);
  font-size: 10px;
  padding: 4px 16px;
  background: var(--bg-soft);
  text-align: right;
}
</style>
