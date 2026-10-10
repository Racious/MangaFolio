<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { cancelScan, forgetRoot, listRoots, pickFolders, pickMediaFiles, rememberRoots, scanSources, type ScanResult } from "../api/media";
import { displayPath } from "../lib/library";
import { useLibraryStore } from "../stores/library";
const props = defineProps<{ mode: "folders" | "roots" | "files" | "rescan" }>();
const emit = defineEmits<{ close: [] }>();
const library = useLibraryStore();
const dialog = ref<HTMLDialogElement>();
const paths = ref<string[]>([]), roots = ref<string[]>([]);
const result = ref<ScanResult | null>(null);
const scanning = ref(false), selecting = ref(false), applying = ref(false), stopping = ref(false), remember = ref(true);
const error = ref("");
const selected = computed(() => result.value?.items.filter(i => i.selected) ?? []);
const busy = computed(() => scanning.value || selecting.value || applying.value);
const types = { folder: "漫畫／圖集", cbz: "ZIP／CBZ", video: "影片" };
async function scan() {
  if (busy.value || !paths.value.length) return;
  error.value = ""; result.value = null; scanning.value = true; stopping.value = false;
  library.managing = true;
  try { result.value = await scanSources(paths.value, props.mode === "roots" || props.mode === "rescan"); }
  catch (e) { error.value = String(e); }
  finally { scanning.value = false; library.managing = false; }
}
async function choose() {
  if (busy.value) return;
  selecting.value = true; library.managing = true;
  try {
    const chosen = props.mode === "files" ? await pickMediaFiles() : await pickFolders();
    if (!chosen?.length) { emit("close"); return; }
    paths.value = chosen;
  } catch (e) { error.value = String(e); }
  finally { selecting.value = false; library.managing = false; }
  await scan();
}
async function stop() {
  stopping.value = true;
  try { await cancelScan(); } catch (e) { error.value = String(e); stopping.value = false; }
}
async function removeRoot(path: string) {
  if (busy.value) return;
  try {
    await forgetRoot(path); roots.value = roots.value.filter(p => p !== path); paths.value = [...roots.value]; result.value = null;
  } catch (e) { error.value = String(e); }
}
async function apply() {
  if (busy.value || !selected.value.length) return;
  applying.value = true; error.value = "";
  try {
    await library.add(selected.value.map(i => i.path));
    if (library.error) { error.value = library.error; return; }
    if (props.mode === "roots" && remember.value) {
      try { await rememberRoots(paths.value); }
      catch (e) { library.importNotice = `作品已匯入，但無法記住主目錄：${e}。可稍後重新加入主目錄。`; }
    }
    emit("close");
  } catch (e) { error.value = String(e); }
  finally { applying.value = false; }
}
onMounted(async () => {
  dialog.value?.showModal();
  if (props.mode === "rescan") {
    try { roots.value = await listRoots(); paths.value = [...roots.value]; await scan(); }
    catch (e) { error.value = String(e); }
  } else await choose();
});
</script>
<template>
  <dialog ref="dialog" class="import-panel" aria-labelledby="import-title" @cancel="busy ? $event.preventDefault() : emit('close')">
    <h2 id="import-title">{{ mode === 'rescan' ? '重新掃描主目錄' : '匯入預覽' }}</h2>
    <p>漫畫以直接包含圖片的資料夾或壓縮檔為一筆，影片以每個檔案為一筆。已加入的來源不預選，勾選可更新資訊。</p>
    <ul v-if="mode === 'rescan'" class="roots">
      <li v-for="path in roots" :key="path">{{ displayPath(path) }} <button :disabled="busy" @click="removeRoot(path)">忘記此主目錄</button></li>
    </ul>
    <p v-if="mode === 'rescan' && !paths.length">尚未記住主目錄。請使用「加入主目錄」並勾選記住路徑。</p>
    <p v-if="busy" role="status">{{ selecting ? '請在系統視窗選擇來源…' : scanning ? (stopping ? '正在停止掃描…' : '正在掃描來源…') : `正在匯入 ${library.importProgress.completed}／${library.importProgress.total}…` }}</p>
    <button v-if="scanning" :disabled="stopping" @click="stop">取消掃描</button>
    <button v-if="applying" :disabled="library.importCanceled" @click="library.cancelImport">取消剩餘匯入</button>
    <template v-if="result">
      <p role="status">找到 {{ result.items.length }} 筆作品，已選 {{ selected.length }} 筆。{{ result.canceled ? '掃描已取消，下列僅為已找到的部分。' : '' }}</p>
      <p v-if="result.truncated" class="warning">掃描達數量／深度限制或問題過多；結果可能不完整，請縮小主目錄後重試。</p>
      <div class="actions"><button :disabled="busy" @click="result.items.forEach(i => i.selected = !i.existing && !i.warning)">選取可辨識的新作品</button><button :disabled="busy" @click="result.items.forEach(i => i.selected = false)">清除選取</button></div>
      <ul class="candidates">
        <li v-for="item in result.items" :key="item.path"><label><input v-model="item.selected" type="checkbox" :disabled="busy" /><span><strong>{{ item.title }}</strong> · {{ types[item.format] }} {{ item.existing ? ' · 已加入' : '' }}<small>{{ displayPath(item.path) }}</small><small v-if="item.warning" class="warning">{{ item.warning }}</small></span></label></li>
      </ul>
      <details v-if="result.issues.length"><summary>掃描問題（{{ result.issues.length }}）</summary><ul><li v-for="issue in result.issues" :key="issue">{{ displayPath(issue) }}</li></ul></details>
    </template>
    <label v-if="mode === 'roots'"><input v-model="remember" type="checkbox" :disabled="busy" />記住主目錄，之後可重新掃描新增作品</label>
    <p v-if="error" role="alert" class="warning">{{ error }}</p>
    <div class="actions"><button :disabled="busy" @click="emit('close')">關閉</button><button v-if="mode === 'rescan'" :disabled="busy || !paths.length" @click="scan">重新掃描</button><button v-else :disabled="busy" @click="choose">重新選擇來源</button><button class="primary" :disabled="busy || !selected.length" @click="apply">匯入所選 {{ selected.length }} 筆</button></div>
  </dialog>
</template>
<style scoped>
.import-panel { margin: auto; width: min(850px, calc(100vw - 32px)); max-height: calc(100vh - 40px); padding: 24px; overflow: auto; background: var(--panel); color: var(--text); border: 1px solid var(--border); border-radius: var(--radius); }
dialog::backdrop { background: var(--modal-backdrop); }
p, small { color: var(--text-dim); line-height: 1.7; font-size: 13px; margin: 12px 0; }
ul { list-style: none; padding: 0; overflow-wrap: anywhere; }
.candidates { max-height: 42vh; overflow: auto; margin: 12px 0; }
.candidates li { border-bottom: 1px solid var(--border); padding: 10px 0; }
label { display: flex; gap: 10px; align-items: flex-start; }
small { display: block; margin: 4px 0; }
input[type=checkbox] { width: auto; flex: 0 0 auto; margin-top: 5px; }
.warning { color: var(--danger, #e5a36f); }
.actions { display: flex; flex-wrap: wrap; gap: 8px; margin: 14px 0; }
.roots li { margin: 8px 0; }
</style>
