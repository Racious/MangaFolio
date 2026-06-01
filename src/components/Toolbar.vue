<script setup lang="ts">
import { useReaderStore, type Transition } from "../stores/reader";
import { pickFolder, pickFile, type FitMode } from "../api/backend";

const reader = useReaderStore();

const ZOOMS: { value: FitMode; label: string }[] = [
  { value: "window", label: "配合視窗" },
  { value: "width", label: "配合寬度" },
  { value: "height", label: "配合高度" },
  { value: "original", label: "原始尺寸" },
  { value: "fixed", label: "固定倍率" },
];

const TRANSITIONS: { value: Transition; label: string }[] = [
  { value: "book", label: "翻頁：翻書" },
  { value: "slide", label: "翻頁：推移" },
  { value: "fade", label: "翻頁：淡入" },
  { value: "none", label: "翻頁：無" },
];

async function openFolder() {
  const path = await pickFolder();
  if (path) await reader.open(path);
}

async function openFile() {
  const path = await pickFile();
  if (path) await reader.open(path);
}

function onZoom(e: Event) {
  reader.setZoom((e.target as HTMLSelectElement).value as FitMode);
}

function onTransition(e: Event) {
  reader.setTransition((e.target as HTMLSelectElement).value as Transition);
}

function bumpScale(delta: number) {
  reader.setFixedScale(Math.round((reader.fixedScale + delta) * 100) / 100);
}
</script>

<template>
  <header class="toolbar">
    <div class="group">
      <button class="btn" @click="openFolder">開啟資料夾</button>
      <button class="btn" @click="openFile">開啟檔案</button>
    </div>

    <div class="title" :title="reader.title">{{ reader.title || "MangaFolio" }}</div>

    <div class="group right">
      <!-- 縮放模式 -->
      <select class="select" :value="reader.zoom" :disabled="!reader.hasBook" @change="onZoom">
        <option v-for="z in ZOOMS" :key="z.value" :value="z.value">{{ z.label }}</option>
      </select>
      <div v-if="reader.zoom === 'fixed'" class="scale">
        <button class="btn ghost sq" :disabled="!reader.hasBook" @click="bumpScale(-0.1)">−</button>
        <span class="counter sm">{{ Math.round(reader.fixedScale * 100) }}%</span>
        <button class="btn ghost sq" :disabled="!reader.hasBook" @click="bumpScale(0.1)">＋</button>
      </div>

      <!-- 翻頁特效 -->
      <select class="select" :value="reader.transition" @change="onTransition">
        <option v-for="t in TRANSITIONS" :key="t.value" :value="t.value">{{ t.label }}</option>
      </select>

      <span class="sep"></span>

      <!-- 單／雙頁 -->
      <button
        class="btn ghost"
        :disabled="!reader.hasBook"
        @click="reader.togglePageMode()"
        :title="reader.pageMode === 'single' ? '切換為雙頁' : '切換為單頁'"
      >
        {{ reader.pageMode === "single" ? "單頁" : "雙頁" }}
      </button>

      <!-- 封面單獨（雙頁時校正跨頁配對） -->
      <button
        v-if="reader.isDouble"
        class="btn ghost"
        :class="{ on: reader.doubleCover }"
        :disabled="!reader.hasBook"
        @click="reader.toggleDoubleCover()"
        title="封面單獨成頁，校正後續跨頁配對（如 1 單頁，2-3、4-5 配對）"
      >
        封面單獨
      </button>

      <!-- 左開／右開 -->
      <button
        class="btn ghost"
        :disabled="!reader.hasBook"
        @click="reader.toggleDirection()"
        :title="reader.direction === 'rtl' ? '右開（右到左，右頁數字小）' : '左開（左到右，左頁數字小）'"
      >
        {{ reader.direction === "rtl" ? "右開 →" : "← 左開" }}
      </button>

      <span class="sep"></span>

      <div class="pager" v-if="reader.hasBook">
        <button class="btn ghost sq" :disabled="reader.atFirst" @click="reader.prev()">‹</button>
        <span class="counter">{{ reader.index + 1 }} / {{ reader.pageCount }}</span>
        <button class="btn ghost sq" :disabled="reader.atLast" @click="reader.next()">›</button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 14px;
  background: var(--bg-soft);
  border-bottom: 1px solid var(--border);
  flex: 0 0 auto;
}
.group {
  display: flex;
  align-items: center;
  gap: 8px;
}
.group.right {
  margin-left: auto;
}
.title {
  flex: 1;
  text-align: center;
  color: var(--accent-soft);
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.5px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.btn {
  background: var(--panel);
  border: 1px solid var(--border);
  color: var(--text);
  padding: 6px 14px;
  border-radius: 7px;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.btn:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent-soft);
}
.btn:disabled {
  opacity: 0.4;
  cursor: default;
}
.btn.ghost {
  background: transparent;
}
.btn.on {
  border-color: var(--accent);
  color: var(--accent-soft);
  background: rgba(200, 169, 106, 0.12);
}
.btn.sq {
  padding: 6px 10px;
}
.select {
  background: var(--panel);
  border: 1px solid var(--border);
  color: var(--text);
  padding: 6px 10px;
  border-radius: 7px;
  font-size: 13px;
  cursor: pointer;
}
.select:disabled {
  opacity: 0.4;
}
.scale {
  display: flex;
  align-items: center;
  gap: 4px;
}
.sep {
  width: 1px;
  height: 20px;
  background: var(--border);
}
.pager {
  display: flex;
  align-items: center;
  gap: 6px;
}
.counter {
  font-size: 13px;
  color: var(--text-dim);
  min-width: 72px;
  text-align: center;
  font-variant-numeric: tabular-nums;
}
.counter.sm {
  min-width: 48px;
}
</style>
