<script setup lang="ts">
import { computed, ref, onMounted } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  previewBackup,
  restoreBackup,
  exportBackup,
  type RestorePreview,
} from "../api/library";
import { useLibraryStore } from "../stores/library";
import { useReaderStore } from "../stores/reader";
import { useBackupStore } from "../stores/backup";
const library = useLibraryStore(),
  reader = useReaderStore(),
  backup = useBackupStore();
const path = ref(""),
  preview = ref<RestorePreview | null>(null),
  error = ref("");
const busy = computed(
  () =>
    library.managing ||
    library.importing ||
    library.loading ||
    reader.loading ||
    reader.favoritePending ||
    library.favoritePending.size > 0 ||
    backup.pending,
);
onMounted(() => void backup.refresh());
async function run(action: () => Promise<void>) {
  if (busy.value) return;
  library.managing = true;
  error.value = "";
  try {
    await reader.flushProgress();
    await action();
  } catch (e) {
    error.value = String(e);
  } finally {
    library.managing = false;
  }
}
async function manual() {
  await run(async () => {
    const selected = await save({
      defaultPath: `MangaFolio-${Date.now()}.mangafolio.json`,
      filters: [{ name: "MangaFolio 備份", extensions: ["json"] }],
    });
    if (!selected) return;
    await exportBackup(selected);
    await backup.refresh();
    library.importNotice = "備份已匯出；不包含漫畫來源檔案。";
  });
}
async function inspect() {
  await run(async () => {
    preview.value = null;
    path.value = "";
    const selected = (await open({
      multiple: false,
      filters: [{ name: "MangaFolio 備份", extensions: ["json"] }],
    })) as string | null;
    if (!selected) return;
    const result = await previewBackup(selected);
    path.value = selected;
    preview.value = result;
  });
}
async function restore() {
  if (!preview.value?.canRestore) return;
  await run(async () => {
    const result = await restoreBackup(path.value);
    preview.value = null;
    path.value = "";
    reader.discardBook();
    library.selectedIds = [];
    await library.refresh();
    library.importNotice = `已合併還原 ${result.added} 本書，略過 ${result.skipped} 本既有來源。`;
  });
}
async function automatic() {
  await run(async () => {
    await backup.maintain(true);
    if (backup.error) throw new Error(backup.error);
    library.importNotice = "本機安全備份已建立。";
  });
}
</script>
<template>
  <details class="backup-panel">
    <summary>備份與還原</summary>
    <p>
      備份 v3
      包含書庫資訊、收藏、進度、偏好、標籤與書籤／筆記；不含漫畫來源、封面、外觀／工具列及本機自動備份設定。
    </p>
    <div class="panel-actions">
      <button :disabled="busy" @click="manual">匯出書庫備份</button
      ><button :disabled="busy" @click="inspect">還原書庫備份</button>
    </div>
    <section class="backup-options">
      <label
        ><input
          type="checkbox"
          :checked="backup.settings.enabled"
          :disabled="busy"
          @change="
            backup.configure(
              ($event.target as HTMLInputElement).checked,
              backup.settings.retention,
            )
          "
        />
        自動備份（每日最多一份，啟動及每 15 分鐘檢查）</label
      ><label
        >保留份數<select
          aria-label="保留份數"
          :value="backup.settings.retention"
          :disabled="busy"
          @change="
            backup.configure(
              backup.settings.enabled,
              Number(($event.target as HTMLSelectElement).value),
            )
          "
        >
          <option v-for="n in 20" :key="n" :value="n">{{ n }}</option>
        </select></label
      ><button :disabled="busy" @click="automatic">立即建立本機安全備份</button>
      <p>
        最近成功：{{
          backup.settings.lastSuccess
            ? new Date(backup.settings.lastSuccess).toLocaleString()
            : "尚無備份"
        }}
      </p>
      <p>升級前 SQLite 快照另行保存，不依自動保留數清理。</p>
    </section>
    <p
      v-if="error || backup.error || backup.settings.lastError"
      role="alert"
      class="error"
    >
      {{ error || backup.error || backup.settings.lastError }}
    </p>
    <section v-if="preview" class="restore-preview" aria-label="還原預覽">
      <h3>還原預覽</h3>
      <p class="path">{{ path }}</p>
      <p>
        新增 {{ preview.added }} · 略過 {{ preview.skipped }} · 衝突
        {{ preview.conflicts }} · 不支援 {{ preview.unsupported }}
      </p>
      <ul>
        <li v-for="issue in preview.issues" :key="issue">{{ issue }}</li>
      </ul>
      <p>預覽未修改書庫。實際還原會重新讀取及驗證檔案，既有來源不覆寫。</p>
      <div class="panel-actions">
        <button
          class="primary"
          :disabled="busy || !preview.canRestore"
          @click="restore"
        >
          確認合併還原</button
        ><button
          :disabled="busy"
          @click="
            preview = null;
            path = '';
          "
        >
          取消還原
        </button>
      </div>
    </section>
  </details>
</template>
<style scoped>
summary {
  cursor: pointer;
  font-weight: 600;
  padding: 14px 0;
}
p,
li {
  font-size: 12px;
  color: var(--text-dim);
  line-height: 1.8;
  margin: 10px 0;
  overflow-wrap: anywhere;
}
.panel-actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}
.backup-options {
  display: grid;
  gap: 12px;
  margin: 20px 0;
  padding: 20px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
}
.backup-options label {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
}
.restore-preview {
  margin-top: 20px;
  padding: 20px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
}
.error {
  color: var(--red);
}
ul {
  padding-left: 20px;
}
.path {
  font-size: 11px;
}
</style>
