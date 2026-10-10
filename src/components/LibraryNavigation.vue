<script setup lang="ts">
import { computed } from "vue";
import { PhBooks, PhStar, PhClockCounterClockwise, PhFolderNotchOpen, PhGearSix, PhImage, PhFilmStrip, PhStack } from "@phosphor-icons/vue";
import { useLibraryStore, type LibraryFilter } from "../stores/library";
import { groupSeries } from "../lib/series";
import type { MediaFilter } from "../lib/library";
defineProps<{ mode: "workbench" | "gallery" | "studio"; busy: boolean }>();
defineEmits<{ settings: [] }>();
const library = useLibraryStore();
const groups = computed(() => groupSeries(library.books));
const filters = [
  { id:"all", label:"書庫", icon:PhBooks },
  { id:"favorites", label:"收藏", icon:PhStar },
  { id:"recent", label:"最近", icon:PhClockCounterClockwise },
  { id:"missing", label:"失效來源", icon:PhFolderNotchOpen },
] as const;
const media = [{id:"all",label:"全部作品",icon:PhBooks},{id:"comic",label:"漫畫／圖集",icon:PhImage},{id:"video",label:"影片",icon:PhFilmStrip}] as const;
function filter(id: LibraryFilter) { library.section="books"; library.activeSeries=null; library.setFilter(id); }
function selectMedia(id: MediaFilter) { library.setMediaFilter(id); library.section="books"; library.activeSeries=null; }
function count(id: LibraryFilter) {
  return library.books.filter(book => (library.mediaFilter === "all" || (library.mediaFilter === "video" ? book.format === "video" : book.format !== "video")) && (id === "favorites" ? book.favorite : id === "recent" ? book.lastReadAt !== null : id === "missing" ? !book.available : true)).length;
}
</script>
<template>
  <div class="navigation" :class="mode">
    <div class="brand"><PhBooks :size="25" aria-hidden="true" /><span>MangaFolio</span></div>
    <nav aria-label="書庫導覽">
      <div v-if="mode === 'workbench'" class="media-nav"><button v-for="item in media" :key="item.id" :disabled="busy" :aria-pressed="library.mediaFilter === item.id" @click="selectMedia(item.id)"><component :is="item.icon" :size="20" aria-hidden="true" /><span>{{ item.label }}</span><small>{{ library.books.filter(b => item.id === 'all' || (item.id === 'video' ? b.format === 'video' : b.format !== 'video')).length }}</small></button></div>
      <button v-for="item in filters" :key="item.id" :disabled="busy" :aria-pressed="library.filter === item.id && library.section === 'books' && !library.activeSeries" @click="filter(item.id)"><component :is="item.icon" :size="21" aria-hidden="true" /><span>{{ item.label }}</span><small v-if="mode === 'workbench'">{{ count(item.id) }}</small></button>
      <button :disabled="busy || library.mediaFilter === 'video'" :aria-pressed="library.section === 'series'" @click="library.section = 'series'; library.activeSeries = null"><PhStack :size="21" aria-hidden="true" /><span>系列</span><small v-if="mode === 'workbench'">{{ groups.length }}</small></button>
    </nav>
    <button class="settings" aria-label="開啟設定" @click="$emit('settings')"><PhGearSix :size="22" aria-hidden="true" /><span>設定</span></button>
  </div>
</template>
<style scoped>
.navigation { display:flex; height:100%; flex-direction:column; gap:28px; padding:28px 18px; background:var(--panel); }
.brand { display:flex; align-items:center; gap:10px; font-weight:700; font-size:20px; white-space:nowrap; color:var(--text); }
.brand svg { color:var(--accent-soft); flex-shrink:0; }
nav { display:grid; gap:6px; }
button { display:flex; gap:11px; align-items:center; padding:12px; border-color:transparent; background:transparent; text-align:left; font-size:14px; }
button span { flex:1; } small { font-size:12px; color:var(--text-dim); }
button[aria-pressed=true] { background:var(--bg-soft); color:var(--accent-soft); font-weight:600; }
.media-nav { display:grid; gap:6px; border-bottom:1px solid var(--line); padding-bottom:20px; margin-bottom:14px; }
.settings { margin-top:auto; }
.gallery { flex-direction:row; align-items:center; height:64px; gap:42px; padding:0 52px; background:transparent; border-bottom:1px solid var(--line); }
.gallery nav { display:flex; justify-content:center; align-items:center; gap:8px; flex:1; }
.gallery nav button { border-radius:0; padding:20px 14px; font-size:14px; }
.gallery nav svg { display:none; }
.gallery button[aria-pressed=true] { background:transparent; border-bottom:2px solid var(--accent); }
.gallery .settings { margin:0 0 0 auto; }
.studio { padding:26px 8px 16px; gap:40px; align-items:center; }
.studio .brand span { display:none; }
.studio nav { width:100%; gap:12px; }
.studio nav button, .studio .settings { flex-direction:column; gap:6px; justify-content:center; padding:12px 3px; font-size:12px; width:100%; }
.studio button[aria-pressed=true] { box-shadow:inset 3px 0 var(--accent); border-radius:6px; }
@media(max-width:900px) { .gallery { padding:0 20px; gap:16px; } .gallery .brand span { display:none; } .gallery nav { gap:0; } .gallery nav button { padding:25px 10px; } }
@media(max-width:700px) { .navigation { padding:12px; gap:12px; } .brand { font-size:16px; } nav { display:flex; flex-wrap:wrap; gap:3px; } .media-nav { display:none; } .settings { margin:0; } .gallery { height:auto; flex-wrap:wrap; padding:12px; } .gallery nav { order:3; width:100%; flex-basis:100%; flex-wrap:wrap; } .gallery nav button { padding:10px; } .studio { flex-direction:row; align-items:center; } .studio nav { flex:1; width:auto; gap:2px; } .studio nav button { width:auto; flex:1; min-width:50px; padding:8px 4px; } .studio .settings { width:55px; padding:8px 4px; } .studio .brand { display:none; } }
@media(max-width:700px) { .workbench { display:grid; grid-template-columns:1fr auto; } .workbench nav { grid-column:1 / -1; } .workbench .settings { grid-column:2; grid-row:1; } .workbench .brand { grid-column:1; grid-row:1; } }
</style>
