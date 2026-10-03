<script setup lang="ts">
import { computed } from "vue";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import {
  exportBackup,
  favoriteBooks,
  relinkBook,
  removeBooks,
  restoreBackup,
} from "../api/library";
import { pickFolder } from "../api/backend";
import { useLibraryStore } from "../stores/library";
import { useReaderStore } from "../stores/reader";

const props = defineProps<{ selectedIds: number[]; shownCount: number }>();
const emit = defineEmits<{ clear: []; selectShown: [] }>();
const library = useLibraryStore();
const reader = useReaderStore();
const busy = computed(
  () =>
    library.managing ||
    library.importing ||
    library.loading ||
    reader.loading ||
    library.favoritePending.size > 0 ||
    reader.favoritePending,
);

async function run(action: (ids: number[]) => Promise<void>) {
  if (busy.value) return;
  const ids = [...props.selectedIds];
  library.managing = true;
  library.error = "";
  library.importNotice = "";
  try {
    await reader.flushProgress();
    await action(ids);
  } catch (error) {
    library.error = String(error);
  } finally {
    library.managing = false;
  }
}
async function removeSelected() {
  await run(async (ids) => {
    if (!ids.length) return;
    if (
      !(await ask(
        `將 ${ids.length} 本書移出書庫？\n這些項目的收藏與進度會移除，原始漫畫檔案會保留。可先匯出備份。`,
        {
          title: "移除書庫項目",
          kind: "warning",
          okLabel: "確認",
          cancelLabel: "取消",
        },
      ))
    )
      return;
    await removeBooks(ids);
    if (reader.bookId !== null && ids.includes(reader.bookId))
      reader.discardBook();
    emit("clear");
    await library.refresh();
    library.importNotice = `已移除 ${ids.length} 本書，原始漫畫檔案已保留。`;
  });
}
async function setSelectedFavorite(favorite: boolean) {
  await run(async (ids) => {
    if (!ids.length) return;
    await favoriteBooks(ids, favorite);
    if (reader.bookId !== null && ids.includes(reader.bookId))
      reader.favorite = favorite;
    emit("clear");
    await library.refresh();
    library.importNotice = `已${favorite ? "收藏" : "取消收藏"} ${ids.length} 本書。`;
  });
}
async function relink(folder: boolean) {
  await run(async (ids) => {
    if (ids.length !== 1) return;
    const path = folder
      ? await pickFolder()
      : ((await open({
          multiple: false,
          filters: [{ name: "漫畫壓縮檔", extensions: ["zip", "cbz"] }],
        })) as string | null);
    if (!path) return;
    const title =
      library.books.find((book) => book.id === ids[0])?.title ?? "所選書籍";
    if (
      !(await ask(
        `為「${title}」改用所選來源？\n請選同一本漫畫，收藏、閱讀設定與進度會保留；頁面改動時優先依檔名定位。`,
        {
          title: "重新指定來源",
          kind: "warning",
          okLabel: "確認",
          cancelLabel: "取消",
        },
      ))
    )
      return;
    await relinkBook(ids[0], path);
    if (reader.bookId === ids[0]) reader.discardBook();
    emit("clear");
    await library.refresh();
    library.importNotice = "已更新來源，請從封面重新開啟閱讀。";
  });
}
async function backup() {
  await run(async () => {
    const path = await save({
      defaultPath: `MangaFolio-${new Date().toISOString().slice(0, 10)}.mangafolio.json`,
      filters: [{ name: "MangaFolio 書庫備份", extensions: ["json"] }],
    });
    if (!path) return;
    await exportBackup(path);
    library.importNotice =
      "備份已匯出：包含來源路徑、收藏、進度與閱讀設定。漫畫檔案需另外備份。";
  });
}
async function restore() {
  await run(async () => {
    const path = (await open({
      multiple: false,
      filters: [{ name: "MangaFolio 書庫備份", extensions: ["json"] }],
    })) as string | null;
    if (!path) return;
    if (
      !(await ask(
        "將備份合併到書庫？\n相同來源路徑會保留目前資料並略過；其他書籍會加入。備份不包含漫畫檔案，找不到來源時可重新指定。",
        {
          title: "還原書庫備份",
          kind: "info",
          okLabel: "確認",
          cancelLabel: "取消",
        },
      ))
    )
      return;
    const result = await restoreBackup(path);
    emit("clear");
    await library.refresh();
    library.importNotice = `還原完成：加入 ${result.added} 本，略過 ${result.skipped} 本既有來源。`;
  });
}
</script>

<template>
  <section
    class="manager"
    aria-labelledby="manager-title"
    :aria-busy="library.managing"
  >
    <h2 id="manager-title">管理與備份</h2>
    <p>先勾選卡片。移除只影響書庫紀錄；重新指定來源請選同一本漫畫。</p>
    <div class="actions">
      <span aria-live="polite">已選 {{ selectedIds.length }} 本</span>
      <button :disabled="busy || !shownCount" @click="emit('selectShown')">
        選取目前顯示的 {{ shownCount }} 本
      </button>
      <button :disabled="busy || !selectedIds.length" @click="emit('clear')">
        清除選取
      </button>
      <button
        :disabled="busy || !selectedIds.length"
        @click="setSelectedFavorite(true)"
      >
        批次收藏
      </button>
      <button
        :disabled="busy || !selectedIds.length"
        @click="setSelectedFavorite(false)"
      >
        批次取消收藏
      </button>
      <button :disabled="busy || !selectedIds.length" @click="removeSelected">
        移除所選
      </button>
      <button
        :disabled="busy || selectedIds.length !== 1"
        @click="relink(false)"
      >
        重新指定 ZIP／CBZ
      </button>
      <button
        :disabled="busy || selectedIds.length !== 1"
        @click="relink(true)"
      >
        重新指定圖片資料夾
      </button>
    </div>
    <div class="actions">
      <button :disabled="busy" @click="backup">匯出書庫備份</button>
      <button :disabled="busy" @click="restore">還原書庫備份</button>
    </div>
    <p>
      備份只含書庫資料，不含漫畫、封面或教學狀態。還原採合併，既有來源不覆寫；匯出請使用新檔名。
    </p>
    <p v-if="library.managing" role="status">正在處理書庫，請稍候…</p>
  </section>
</template>

<style scoped>
.manager {
  padding: 18px 20px;
  margin-bottom: 24px;
  background: var(--bg-soft);
  border: 1px solid var(--border);
  border-radius: 12px;
}
h2 {
  font-size: 16px;
}
p {
  color: var(--text-dim);
  font-size: 12px;
  line-height: 1.8;
  margin-top: 10px;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 14px;
}
span {
  color: var(--accent-soft);
  font-size: 12px;
}
button {
  font: inherit;
  font-size: 12px;
  padding: 9px 12px;
  background: var(--panel);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 8px;
  cursor: pointer;
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
