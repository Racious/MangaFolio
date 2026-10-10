<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed } from "vue";
import { PhX, PhPaintBrush, PhFolderNotchOpen, PhTag, PhDatabase, PhQuestion } from "@phosphor-icons/vue";
import AppearanceSettings from "./AppearanceSettings.vue";
import TagManager from "./TagManager.vue";
import BackupPanel from "./BackupPanel.vue";
import LibraryGuide from "./LibraryGuide.vue";
import { listRoots, forgetRoot } from "../api/media";
import { useLibraryStore } from "../stores/library";
import { useReaderStore } from "../stores/reader";
import { useBackupStore } from "../stores/backup";
import { displayPath } from "../lib/library";
const emit = defineEmits<{ close: []; import: [mode: "roots" | "rescan"] }>();
const library = useLibraryStore(), reader = useReaderStore();
const backup = useBackupStore();
const sections = [
  { id: "appearance", title: "外觀與閱讀", icon: PhPaintBrush },
  { id: "sources", title: "書庫來源", icon: PhFolderNotchOpen },
  { id: "tags", title: "類別與標籤", icon: PhTag },
  { id: "backup", title: "資料與備份", icon: PhDatabase },
  { id: "help", title: "使用說明", icon: PhQuestion },
];
const section = ref("appearance"), dialog = ref<HTMLDialogElement>();
const roots = ref<string[]>([]), error = ref(""), loading = ref(false);
const pending = ref(false);
const busy = computed(() => pending.value || backup.pending || library.loading || library.managing || library.importing || reader.loading || reader.favoritePending || library.favoritePending.size > 0);
let previousFocus: HTMLElement | null = null, disposed = false;
async function loadRoots() {
  loading.value = true;
  error.value = "";
  try { const paths = await listRoots(); if (!disposed) roots.value = paths; }
  catch (e) { if (!disposed) error.value = `無法載入主目錄：${e}`; }
  finally { loading.value = false; }
}
async function forget(path: string) {
  if (busy.value) return;
  pending.value = true; error.value = "";
  try { await forgetRoot(path); roots.value = roots.value.filter(root => root !== path); library.notifySuccess("已忘記主目錄，現有作品與原始檔案保留。"); }
  catch (e) { error.value = `無法忘記主目錄：${e}`; }
  finally { pending.value = false; }
}
function select(id: string) { if (busy.value) return; section.value = id; if (id === "sources") void loadRoots(); }
function close() { if (!busy.value) emit("close"); }
function importSources(mode: "roots" | "rescan") { if (!busy.value) emit("import", mode); }
onMounted(() => { previousFocus = document.activeElement as HTMLElement | null; dialog.value?.showModal(); });
onBeforeUnmount(() => {
  disposed = true; dialog.value?.close();
  const target = previousFocus?.isConnected ? previousFocus : document.querySelector<HTMLElement>('[aria-label="開啟設定"]');
  target?.focus({ preventScroll: true });
});
</script>
<template>
  <dialog ref="dialog" class="settings-dialog" aria-labelledby="settings-title" @cancel.prevent="close">
    <header><div><p>MangaFolio</p><h1 id="settings-title">設定</h1></div><button :disabled="busy" aria-label="關閉設定" @click="close"><PhX :size="22" aria-hidden="true" /></button></header>
    <div class="settings-layout">
      <nav aria-label="設定分類"><button v-for="item in sections" :key="item.id" :disabled="busy" :aria-pressed="section === item.id" @click="select(item.id)"><component :is="item.icon" :size="19" aria-hidden="true" />{{ item.title }}</button></nav>
      <div class="settings-content">
        <p v-if="library.error" role="alert">{{ library.error }}</p>
        <p v-if="library.importNotice" role="status">{{ library.importNotice }}</p>
        <AppearanceSettings v-if="section === 'appearance'" />
        <section v-else-if="section === 'sources'">
          <h2>書庫來源</h2><p>加入主目錄後可重新掃描，將新作品加入收藏。媒體檔案保留在原位。</p>
          <div class="source-actions"><button class="primary" :disabled="busy" @click="importSources('roots')">加入主目錄</button><button :disabled="busy || loading || !roots.length" @click="importSources('rescan')">重新掃描主目錄</button></div>
          <p v-if="loading" role="status">正在載入主目錄…</p>
          <p v-else-if="!roots.length && !error">尚未記住主目錄。</p>
          <ul><li v-for="path in roots" :key="path"><span>{{ displayPath(path) }}</span><button :disabled="busy" @click="forget(path)">忘記主目錄</button></li></ul>
          <p>忘記主目錄只停止後續掃描，已加入的作品會保留。</p>
          <p v-if="error" role="alert">{{ error }} <button @click="loadRoots">重試</button></p>
        </section>
        <section v-else-if="section === 'tags'"><h2>類別與標籤</h2><p>先建立共用標籤，再回到書庫的「管理作品」，勾選作品並按「批次加入標籤」。</p><TagManager :expanded="true" /></section>
        <section v-else-if="section === 'backup'"><h2>資料與備份</h2><p>備份包含收藏、閱讀資料與自訂封面；原始媒體檔案另行保管。</p><BackupPanel /></section>
        <section v-else><h2>使用說明</h2><p>點封面閱讀或使用預設播放器開啟影片；點作品名稱查看資訊、編輯及替換封面。</p><LibraryGuide embedded /><button @click="close">回到書庫操作</button><p>本機私人收藏 · 無需帳號</p></section>
      </div>
    </div>
    <p v-if="busy" class="pending" role="status">正在處理資料，完成後即可關閉設定。</p>
  </dialog>
</template>
<style scoped>
.settings-dialog { margin:auto; width:min(920px,calc(100vw - 32px)); height:min(760px,calc(100dvh - 40px)); max-height:calc(100dvh - 40px); padding:0; background:var(--panel); color:var(--text); border:1px solid var(--line); border-radius:16px; box-shadow:0 24px 80px #0005; }
.settings-dialog[open] { display:flex; flex-direction:column; }
dialog::backdrop { background:#0009; backdrop-filter:blur(4px); }
header { display:flex; justify-content:space-between; align-items:center; padding:22px 28px; border-bottom:1px solid var(--line); }
header p { margin:0 0 4px; font-size:12px; color:var(--text-dim); }
h1 { font-size:25px; } header button { border:0; background:transparent; padding:10px; display:flex; }
.settings-layout { display:grid; grid-template-columns:200px minmax(0,1fr); grid-template-rows:minmax(0,1fr); flex:1; min-height:0; }
nav { background:var(--bg-soft); padding:20px 12px; display:flex; flex-direction:column; gap:6px; }
nav button { display:flex; align-items:center; gap:10px; padding:13px 12px; border-color:transparent; background:transparent; text-align:left; }
nav button[aria-pressed=true] { background:var(--panel); color:var(--accent-soft); font-weight:600; }
.settings-content { padding:28px; overflow:auto; min-width:0; min-height:0; }
h2 { font-size:22px; } p { font-size:14px; color:var(--text-dim); line-height:1.8; margin:12px 0 18px; }
ul { list-style:none; padding:0; } li { display:flex; align-items:center; gap:12px; padding:16px 0; border-bottom:1px solid var(--line); } li span { flex:1; overflow-wrap:anywhere; font-size:14px; }
.source-actions { display:flex; flex-wrap:wrap; gap:10px; margin:24px 0; } .pending { padding:0 24px; }
@media(max-width:700px) { .settings-layout { grid-template-columns:1fr; grid-template-rows:auto minmax(0,1fr); } nav { flex-direction:row; flex-wrap:wrap; padding:10px; } nav button { padding:10px; } .settings-content { padding:20px; } header { padding:16px 20px; } li { flex-wrap:wrap; } }
</style>
