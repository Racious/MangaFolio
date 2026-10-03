<script setup lang="ts">
import { ref, watch, computed } from "vue";
import {
  listBookmarks,
  saveBookmark,
  deleteBookmark,
  type Bookmark,
} from "../api/library";
import { useReaderStore } from "../stores/reader";
import { useLibraryStore } from "../stores/library";
const props = defineProps<{
  bookId: number;
  pages?: string[];
  indices?: number[];
}>();
const emit = defineEmits<{ opened: [] }>();
const reader = useReaderStore(),
  library = useLibraryStore();
const marks = ref<Bookmark[]>([]),
  error = ref(""),
  pending = ref(false),
  name = ref(""),
  note = ref(""),
  editing = ref<Bookmark | null>(null),
  pageIndex = ref(0);
const busy = computed(
  () =>
    pending.value ||
    library.managing ||
    library.importing ||
    library.loading ||
    reader.loading ||
    reader.favoritePending ||
    library.favoritePending.size > 0,
);
let generation = 0;
async function refresh() {
  const token = ++generation;
  try {
    const result = await listBookmarks(props.bookId);
    if (!Array.isArray(result)) throw new Error("書籤資料格式無效，請重試。");
    if (token === generation) marks.value = result;
  } catch (e) {
    if (token === generation) error.value = String(e);
  }
}
watch(
  () => props.bookId,
  () => {
    editing.value = null;
    name.value = "";
    note.value = "";
    void refresh();
  },
  { immediate: true },
);
watch(
  () => props.indices,
  (indices) => {
    if (indices?.length && !indices.includes(pageIndex.value))
      pageIndex.value = indices[0];
  },
  { immediate: true },
);
function edit(mark: Bookmark) {
  editing.value = mark;
  name.value = mark.name;
  note.value = mark.note;
}
function cancel() {
  editing.value = null;
  name.value = "";
  note.value = "";
}
async function save() {
  if (busy.value) return;
  pending.value = true;
  library.managing = true;
  error.value = "";
  try {
    const mark = editing.value ?? {
      id: 0,
      bookId: props.bookId,
      pageIndex: pageIndex.value,
      pageName: props.pages?.[pageIndex.value] ?? "",
      name: "",
      note: "",
    };
    await saveBookmark({ ...mark, name: name.value, note: note.value });
    cancel();
    await refresh();
  } catch (e) {
    error.value = String(e);
  } finally {
    pending.value = false;
    library.managing = false;
  }
}
async function remove(mark: Bookmark) {
  if (busy.value) return;
  pending.value = true;
  library.managing = true;
  error.value = "";
  try {
    await deleteBookmark(mark.id, props.bookId);
    if (editing.value?.id === mark.id) cancel();
    await refresh();
  } catch (e) {
    error.value = String(e);
  } finally {
    pending.value = false;
    library.managing = false;
  }
}
async function jump(mark: Bookmark) {
  if (busy.value) return;
  pending.value = true;
  error.value = "";
  try {
    if (await reader.openBookmark(props.bookId, mark.id)) {
      library.screen = "reader";
      emit("opened");
    } else error.value = reader.error;
  } finally {
    pending.value = false;
  }
}
</script>
<template>
  <section class="bookmarks-panel" aria-label="書籤與頁面筆記">
    <h3>
      書籤與頁面筆記 <small>{{ marks.length }}／200</small>
    </h3>
    <p>頁面依檔名定位；來源缺頁時不使用無關頁碼。</p>
    <p v-if="error" role="alert" class="error">{{ error }}</p>
    <form v-if="editing || pages?.length" @submit.prevent="save">
      <label v-if="!editing"
        >標記頁面<select v-model.number="pageIndex" :disabled="busy">
          <option v-for="index in indices" :key="index" :value="index">
            第 {{ index + 1 }} 頁 · {{ pages?.[index] }}
          </option>
        </select></label
      >
      <p v-else>第 {{ editing.pageIndex + 1 }} 頁 · {{ editing.pageName }}</p>
      <label
        >書籤名稱<input v-model="name" maxlength="80" required :disabled="busy"
      /></label>
      <label
        >簡短筆記<textarea
          v-model="note"
          maxlength="2000"
          rows="3"
          :disabled="busy"
        />
      </label>
      <div class="panel-actions">
        <button class="primary" :disabled="busy">
          {{ editing ? "儲存書籤" : "新增書籤" }}</button
        ><button v-if="editing" type="button" :disabled="busy" @click="cancel">
          取消編輯
        </button>
      </div>
    </form>
    <p v-else>開啟閱讀器，從「書籤」標記目前頁面。雙頁可指定其中一頁。</p>
    <ul>
      <li v-for="mark in marks" :key="mark.id">
        <button class="bookmark-jump" :disabled="busy" @click="jump(mark)">
          {{ mark.name
          }}<small>第 {{ mark.pageIndex + 1 }} 頁 · {{ mark.pageName }}</small>
        </button>
        <p class="note">{{ mark.note }}</p>
        <div class="panel-actions">
          <button :disabled="busy" @click="edit(mark)">編輯名稱／筆記</button
          ><button :disabled="busy" @click="remove(mark)">刪除書籤</button>
        </div>
      </li>
    </ul>
    <p v-if="!marks.length">尚無書籤。</p>
  </section>
</template>
<style scoped>
h3 {
  font-size: 18px;
}
small {
  font-size: 11px;
  color: var(--text-dim);
}
p {
  font-size: 12px;
  color: var(--text-dim);
  line-height: 1.7;
  margin: 10px 0;
  overflow-wrap: anywhere;
}
form,
label {
  display: grid;
  gap: 8px;
}
form {
  margin: 18px 0;
  gap: 14px;
}
label {
  font-size: 12px;
}
ul {
  list-style: none;
}
li {
  padding: 16px 0;
  border-top: 1px solid var(--line);
}
.bookmark-jump {
  display: block;
  text-align: left;
  width: 100%;
}
.bookmark-jump small {
  display: block;
  margin-top: 5px;
  overflow-wrap: anywhere;
}
.note {
  white-space: pre-wrap;
}
.error {
  color: var(--red);
}
.panel-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
</style>
