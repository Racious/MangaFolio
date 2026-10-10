<script setup lang="ts">
import { ref, computed } from "vue";
import { ask } from "@tauri-apps/plugin-dialog";
import { createTag, renameTag, deleteTag } from "../api/library";
import { useLibraryStore } from "../stores/library";
import { useReaderStore } from "../stores/reader";
defineProps<{ expanded?: boolean }>();
const library = useLibraryStore(),
  reader = useReaderStore();
const name = ref(""),
  tagId = ref<number | null>(null);
const busy = computed(
  () =>
    library.managing ||
    library.importing ||
    library.loading ||
    reader.loading ||
    reader.favoritePending ||
    library.favoritePending.size > 0,
);
async function run(kind: "create" | "rename" | "delete") {
  if (busy.value) return;
  library.managing = true;
  library.error = "";
  try {
    if (kind === "create") await createTag(name.value);
    else if (tagId.value !== null) {
      if (kind === "rename") await renameTag(tagId.value, name.value);
      else if (
        await ask("刪除標籤及其書籍關聯？書籍與來源檔案會保留。", {
          title: "刪除標籤",
          kind: "warning",
          okLabel: "刪除",
          cancelLabel: "取消",
        })
      )
        await deleteTag(tagId.value);
      else return;
    } else return;
    await library.refresh();
    name.value = "";
    if (!library.tags.some((t) => t.id === tagId.value)) tagId.value = null;
    library.notifySuccess("標籤操作已完成。");
  } catch (e) {
    library.error = String(e);
  } finally {
    library.managing = false;
  }
}
</script>
<template>
  <details class="tag-manager" :open="expanded">
    <summary>管理標籤</summary>
    <div class="fields">
      <label
        >現有標籤<select aria-label="現有標籤" v-model="tagId" :disabled="busy">
          <option :value="null">選擇標籤</option>
          <option v-for="tag in library.tags" :key="tag.id" :value="tag.id">
            {{ tag.name }}
          </option>
        </select></label
      >
      <label
        >標籤名稱<input
          v-model="name"
          maxlength="64"
          :disabled="busy"
          placeholder="1–64 字，不區分大小寫"
      /></label>
      <div class="actions">
        <button :disabled="busy || !name.trim()" @click="run('create')">
          建立標籤</button
        ><button
          :disabled="busy || tagId === null || !name.trim()"
          @click="run('rename')"
        >
          重新命名</button
        ><button :disabled="busy || tagId === null" @click="run('delete')">
          刪除標籤
        </button>
      </div>
    </div>
  </details>
</template>
<style scoped>
summary {
  padding: 12px 0;
  cursor: pointer;
  font-weight: 600;
}
.fields {
  display: grid;
  gap: 10px;
}
label {
  display: grid;
  gap: 6px;
  font-size: 12px;
}
select,
input {
  width: 100%;
}
.actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
button {
  font-size: 11px;
  padding: 7px;
}
</style>
