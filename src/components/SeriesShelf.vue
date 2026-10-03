<script setup lang="ts">
import BookCover from "./BookCover.vue";
import type { SeriesGroup } from "../lib/series";
defineProps<{ groups: SeriesGroup[]; busy: boolean }>();
defineEmits<{ open: [key: string] }>();
</script>
<template>
  <section class="series-shelf" aria-label="系列書架">
    <button
      v-for="group in groups"
      :key="group.key"
      class="series-card"
      :disabled="busy"
      :aria-label="`開啟系列：${group.name}`"
      @click="$emit('open', group.key)"
    >
      <BookCover
        :key="group.cover.id"
        :id="group.cover.id"
        :available="group.cover.available"
        :title="group.name"
      />
      <h2>{{ group.name }}</h2>
      <p>
        {{ group.books.length }} 冊 · 已讀 {{ group.read }}／{{
          group.books.length
        }}
        冊
      </p>
      <div class="series-progress">
        <span :style="{ width: `${group.percent}%` }" />
      </div>
      <p>已讀冊數 {{ group.percent }}% · 頁面平均 {{ group.pagePercent }}%</p>
      <p v-if="group.resume">續讀：{{ group.resume.title }}</p>
      <p v-if="group.missing" class="missing">{{ group.missing }} 冊來源失效</p>
    </button>
    <p v-if="!groups.length" class="empty">
      目前沒有符合條件的系列。可在管理模式批次指定系列，或編輯單本的系列資訊。未分系列書籍仍在書庫。
    </p>
  </section>
</template>
<style scoped>
.series-shelf {
  display: grid;
  grid-template-columns: repeat(
    auto-fill,
    minmax(min(var(--cover-min), 100%), 1fr)
  );
  gap: var(--space);
}
.series-card {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  justify-content: flex-start;
  text-align: left;
  padding: var(--grid-inset);
  background: var(--grid-surface);
  border-color: var(--grid-outline);
  min-width: 0;
}
.series-card :deep(.book-cover) {
  box-shadow: var(--grid-cover-shadow);
  width: 100%;
}
h2 {
  font-family: var(--heading-font);
  font-size: 22px;
  margin-top: 16px;
  overflow-wrap: anywhere;
}
p {
  font-size: 12px;
  color: var(--text-dim);
  line-height: 1.7;
  margin-top: 8px;
  overflow-wrap: anywhere;
}
.series-progress {
  height: 5px;
  border-radius: 3px;
  background: var(--line);
  margin-top: 12px;
  overflow: hidden;
}
.series-progress span {
  display: block;
  height: 100%;
  background: var(--accent);
}
.missing {
  color: var(--red);
}
.empty {
  grid-column: 1/-1;
  padding: 32px;
  border: 1px dashed var(--border);
}
</style>
