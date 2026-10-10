<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { PhBookOpen } from "@phosphor-icons/vue";
import { loadCover } from "../api/covers";
import type { LibraryBook } from "../api/library";
import { progressPercent } from "../lib/library";
const props = defineProps<{ book: LibraryBook; busy: boolean }>();
defineEmits<{ open: [id: number] }>();
const cover = ref<string | null>(null);
let cancelled = false;
onMounted(() => {
  loadCover(props.book.id, () => cancelled)
    .then((url) => {
      if (cancelled) {
        if (url) URL.revokeObjectURL(url);
      } else cover.value = url;
    })
    .catch(() => {});
});
onUnmounted(() => {
  cancelled = true;
  if (cover.value) URL.revokeObjectURL(cover.value);
});
</script>
<template>
  <section class="continue-panel" aria-label="繼續閱讀">
    <img v-if="cover" class="resume-background" :src="cover" alt="" />
    <img v-if="cover" class="resume-cover" :src="cover" alt="" />
    <div class="resume-info">
      <p class="resume-label">繼續上次的故事</p>
      <h2>{{ book.title }}</h2>
      <p v-if="book.series || book.volume" class="resume-series">
        {{ book.series }}{{ book.volume ? ` · 第 ${book.volume} 集` : "" }}
      </p>
      <p v-if="book.notes" class="resume-note">{{ book.notes }}</p>
      <p class="resume-page">
        第 {{ book.lastIndex + 1 }}／{{ book.pageCount }} 頁 ·
        {{ progressPercent(book) }}%
      </p>
      <div
        class="resume-progress"
        role="img"
        :aria-label="`閱讀進度 ${progressPercent(book)}%`"
      >
        <span :style="{ width: `${progressPercent(book)}%` }" />
      </div>
      <button class="primary" :disabled="busy" @click="$emit('open', book.id)">
        <PhBookOpen :size="18" aria-hidden="true" /> 接著讀
      </button>
    </div>
  </section>
</template>
<style scoped>
.continue-panel {
  position: relative;
  min-height: var(--hero-min-height);
  display: flex;
  align-items: center;
  gap: var(--resume-gap);
  padding: var(--resume-padding);
  margin-bottom: 28px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--resume-surface);
  overflow: hidden;
}
.resume-background {
  position: absolute;
  inset: 0 0 0 auto;
  width: 70%;
  height: 100%;
  object-fit: cover;
  opacity: 0.13;
  filter: blur(2px);
  pointer-events: none;
}
.continue-panel > :not(.resume-background) {
  position: relative;
  z-index: 1;
}
.resume-series {
  color: var(--text-dim);
  font-size: 13px;
  margin-bottom: 8px;
}
.resume-note {
  color: var(--text-dim);
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  max-width: 420px;
  margin-bottom: 12px;
}
.resume-cover {
  width: var(--hero-cover-width);
  height: var(--hero-cover-height);
  object-fit: contain;
  border-radius: var(--resume-cover-radius);
  box-shadow: var(--cover-shadow);
  flex: 0 0 auto;
}
.resume-info {
  flex: 0 1 420px;
  min-width: 0;
  background: var(--resume-surface);
  padding: 12px;
  border-radius: var(--radius);
}
.resume-label {
  font-size: 12px;
  color: var(--accent-soft);
  margin-bottom: 8px;
  letter-spacing: 0.08em;
}
h2 {
  font-family: var(--heading-font);
  font-size: var(--resume-title-size);
  margin: 0 0 8px;
  overflow-wrap: anywhere;
}
.resume-page {
  font-size: 12px;
  color: var(--text-dim);
}
.resume-progress {
  height: 4px;
  max-width: 280px;
  margin-top: 12px;
  background: var(--line);
  border-radius: 3px;
  overflow: hidden;
}
.resume-progress span {
  display: block;
  height: 100%;
  background: var(--accent);
}
button {
  flex: 0 0 auto;
  margin-top: 16px;
  padding: 12px 22px;
}
@media (max-width: 700px) {
  .continue-panel {
    padding: 16px;
    gap: 12px;
    flex-wrap: wrap;
  }
  .resume-cover {
    width: 54px;
    height: 72px;
    border-radius: 4px;
  }
  h2 {
    font-size: 20px;
  }
  button {
    margin-top: 12px;
  }
}
</style>
