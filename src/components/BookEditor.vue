<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { editBook, type LibraryBook } from "../api/library";
import { useLibraryStore } from "../stores/library";
import { useReaderStore } from "../stores/reader";
import { displayPath } from "../lib/library";
const props = defineProps<{ book: LibraryBook }>();
const emit = defineEmits<{ close: [] }>();
const library = useLibraryStore(),
  reader = useReaderStore();
const dialog = ref<HTMLDialogElement>();
const details = ref({
  customTitle: props.book.customTitle,
  series: props.book.series,
  volume: props.book.volume,
  notes: props.book.notes,
});
const error = ref("");
const busy = computed(
  () =>
    library.managing ||
    library.importing ||
    library.loading ||
    reader.loading ||
    reader.favoritePending ||
    library.favoritePending.size > 0,
);
onMounted(() => dialog.value?.showModal());
async function save() {
  if (busy.value) return;
  library.managing = true;
  error.value = "";
  try {
    await reader.flushProgress();
    const updated = await editBook(props.book.id, details.value);
    if (reader.bookId === updated.id) reader.title = updated.title;
    await library.refresh();
    library.notifySuccess("書籍資訊已保存，來源檔案未更名。");
    emit("close");
  } catch (e) {
    error.value = String(e);
  } finally {
    library.managing = false;
  }
}
</script>
<template>
  <dialog
    ref="dialog"
    aria-labelledby="edit-title"
    @cancel="busy ? $event.preventDefault() : emit('close')"
  >
    <form @submit.prevent="save">
      <h2 id="edit-title">編輯書籍資訊</h2>
      <p class="source">
        原始來源名稱：{{ book.sourceTitle }}<br />{{ displayPath(book.path) }}
      </p>
      <label
        >自訂書名<input
          v-model="details.customTitle"
          maxlength="256"
          :disabled="busy"
          :placeholder="book.sourceTitle"
          autofocus
      /></label>
      <p>留白使用來源名稱；重新加入不會覆寫自訂資訊。</p>
      <label
        >系列<input v-model="details.series" maxlength="256" :disabled="busy"
      /></label>
      <label
        >集數<input v-model="details.volume" maxlength="64" :disabled="busy"
      /></label>
      <label
        >備註<textarea
          v-model="details.notes"
          maxlength="4000"
          rows="4"
          :disabled="busy"
        ></textarea>
      </label>
      <p v-if="error" role="alert">{{ error }}</p>
      <div class="actions">
        <button type="button" :disabled="busy" @click="emit('close')">
          取消</button
        ><button class="primary" :disabled="busy" type="submit">
          保存資訊
        </button>
      </div>
    </form>
  </dialog>
</template>
<style scoped>
dialog {
  margin: auto;
  width: min(540px, calc(100vw - 32px));
  max-height: calc(100vh - 32px);
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--panel);
  color: var(--text);
  padding: 24px;
}
dialog::backdrop {
  background: #0008;
}
h2 {
  font-size: 20px;
  margin-bottom: 12px;
}
label {
  display: grid;
  gap: 6px;
  margin-top: 14px;
  font-size: 13px;
}
p {
  font-size: 12px;
  line-height: 1.7;
  color: var(--text-dim);
  overflow-wrap: anywhere;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 20px;
}
</style>
