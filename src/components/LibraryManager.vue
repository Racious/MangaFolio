<script setup lang="ts">
import BackupPanel from "./BackupPanel.vue";
import { computed, ref } from "vue";
import { ask, open } from "@tauri-apps/plugin-dialog";
import {
  assignSeries,
  setReadingStatus,
  assignTag,
  favoriteBooks,
  relinkBook,
  removeBooks,
} from "../api/library";
import { pickFolder } from "../api/backend";
import { useLibraryStore } from "../stores/library";
import { useReaderStore } from "../stores/reader";

const props = defineProps<{ selectedIds: number[]; shownCount: number; hiddenCount: number }>();
const emit = defineEmits<{ clear: []; selectShown: []; retainShown: [] }>();
const library = useLibraryStore();
const reader = useReaderStore();
const tagId = ref<number | null>(null);
const series = ref("");
async function setSeries() { await run(async(ids) => { if (!ids.length) return; await assignSeries(ids, series.value); await library.refresh(); library.notifySuccess("系列歸屬已更新，書籍與來源保留。"); }); }
async function status(status: "read" | "unread" | "auto") {
  await run(async (ids) => {
    if (!ids.length) return;
    await setReadingStatus(ids, status);
    await library.refresh();
    library.notifySuccess("閱讀狀態已更新，續讀位置保持不變。");
  });
}
async function tag(add: boolean) {
  await run(async (ids) => {
    if (!ids.length || tagId.value === null) return;
    await assignTag(ids, tagId.value, add);
    await library.refresh();
    library.notifySuccess("書籍標籤已更新。");
  });
}
const busy = computed(
  () =>
    library.managing ||
    library.importing ||
    library.loading ||
    reader.loading ||
    library.favoritePending.size > 0 ||
    reader.favoritePending,
);
const relinkSelectionReason = computed(() =>
  props.selectedIds.length !== 1
    ? `重新指定一次只能選 1 本；目前已選 ${props.selectedIds.length} 本。`
    : "",
);
const relinkReason = computed(() =>
  busy.value ? "書庫或閱讀操作進行中，請稍候。" : relinkSelectionReason.value,
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
        `將 ${ids.length} 本書移出書庫？\n這些項目的收藏、閱讀進度與偏好、系列／集數、標籤關聯、書籤／頁面筆記、自訂書名與備註會一併移除。原始漫畫檔案與共用標籤會保留。可先匯出備份。`,
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
    library.notifySuccess(`已移除 ${ids.length} 本書，原始漫畫檔案已保留。`);
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
    library.notifySuccess(`已${favorite ? "收藏" : "取消收藏"} ${ids.length} 本書。`);
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
    else if (reader.missingBookId === ids[0]) {
      reader.error = "";
      reader.missingBookId = null;
    }
    emit("clear");
    await library.refresh();
    library.notifySuccess("已更新來源，請從封面重新開啟閱讀。");
  });
}
</script>

<template>
  <section
    class="manager"
    aria-labelledby="manager-title"
    :aria-busy="busy"
  >
    <h2 id="manager-title">管理與備份</h2>
    <p>點擊書卡切換選取。移除只影響書庫紀錄；重新指定來源請選同一本漫畫。</p>
    <div class="actions">
      <span aria-live="polite">已選 {{ selectedIds.length }} 本</span>
      <template v-if="hiddenCount">
        <span>（包含 {{ hiddenCount }} 本未顯示的選取）</span>
        <button :disabled="busy" @click="emit('retainShown')">只保留目前顯示的選取</button>
      </template>
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
        :title="relinkReason"
        :aria-describedby="relinkReason ? 'relink-reason' : undefined"
        @click="relink(false)"
      >
        重新指定 ZIP／CBZ
      </button>
      <button
        :disabled="busy || selectedIds.length !== 1"
        :title="relinkReason"
        :aria-describedby="relinkReason ? 'relink-reason' : undefined"
        @click="relink(true)"
      >
        重新指定圖片資料夾
      </button>
    </div>
    <p
      v-if="relinkReason"
      id="relink-reason"
      :class="{ 'sr-only': !busy && selectedIds.length === 0 }"
    >{{ relinkReason }}</p>
    <p id="relink-status" class="sr-only" role="status" aria-atomic="true">{{ selectedIds.length > 1 ? relinkSelectionReason : '' }}</p>
    <div class="actions">
      <label>批次系列<input v-model="series" maxlength="256" :disabled="busy" placeholder="系列名稱；留白移除歸屬" /></label>
      <button :disabled="busy || !selectedIds.length" @click="setSeries">指定系列</button>
    </div>
    <div class="actions">
      <button :disabled="busy || !selectedIds.length" @click="status('read')">
        標記已讀
      </button>
      <button :disabled="busy || !selectedIds.length" @click="status('unread')">
        標記未讀
      </button>
      <button :disabled="busy || !selectedIds.length" @click="status('auto')">
        依進度判定
      </button>
      <select v-model="tagId" aria-label="批次標籤" :disabled="busy">
        <option :value="null">選擇標籤</option>
        <option v-for="item in library.tags" :key="item.id" :value="item.id">
          {{ item.name }}
        </option>
      </select>
      <button
        :disabled="busy || !selectedIds.length || tagId === null"
        @click="tag(true)"
      >
        批次加入標籤
      </button>
      <button
        :disabled="busy || !selectedIds.length || tagId === null"
        @click="tag(false)"
      >
        批次移除標籤
      </button>
    </div>
    <BackupPanel />
    <p v-if="library.managing" role="status">正在處理書庫，請稍候…</p>
  </section>
</template>

<style scoped>
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
  border: 0;
}
.manager {
  padding: 18px 20px;
  margin-bottom: var(--space);
  background: var(--bg-soft);
  border: 1px solid var(--border);
  border-radius: var(--radius);
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
summary {
  margin-top: 12px;
  cursor: pointer;
  font-size: 13px;
}
</style>
