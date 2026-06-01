<script setup lang="ts">
import { computed } from "vue";
import { useReaderStore } from "../stores/reader";

const reader = useReaderStore();

/** 取檔名（去除壓縮檔內的路徑）。 */
function baseName(name: string): string {
  return name.split(/[\\/]/).pop() ?? name;
}

/** 當前顯示頁的檔名（依視覺左右順序；雙頁標示左／右）。 */
const namesText = computed(() => {
  const names = reader.slots.map((s) => baseName(reader.pages[s.index] ?? ""));
  if (names.length === 0) return "";
  if (names.length === 1) return names[0];
  return `左 ${names[0]} ｜ 右 ${names[names.length - 1]}`;
});

function onInput(e: Event) {
  reader.goto(Number((e.target as HTMLInputElement).value));
}
</script>

<template>
  <div v-if="reader.hasBook" class="scrubber">
    <span class="names" :title="namesText">{{ namesText }}</span>
    <input
      class="range"
      type="range"
      min="0"
      :max="Math.max(0, reader.pageCount - 1)"
      :value="reader.index"
      :step="1"
      @input="onInput"
    />
    <span class="num">{{ reader.index + 1 }} / {{ reader.pageCount }}</span>
  </div>
</template>

<style scoped>
.scrubber {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 7px 16px;
  background: var(--bg-soft);
  border-top: 1px solid var(--border);
}
.names {
  font-size: 12px;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 320px;
  min-width: 120px;
  font-variant-numeric: tabular-nums;
}
.num {
  font-size: 12px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
  min-width: 72px;
  text-align: right;
}
.range {
  flex: 1;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--border);
  border-radius: 3px;
  outline: none;
  cursor: pointer;
}
.range::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--accent);
  border: 2px solid var(--bg-soft);
  cursor: pointer;
  transition: transform 0.12s ease;
}
.range::-webkit-slider-thumb:hover {
  transform: scale(1.2);
}
</style>
