<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
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
    <img v-if="cover" class="resume-cover" :src="cover" alt="" />
    <div class="resume-info">
      <p class="resume-label">繼續上次的故事</p>
      <h2>{{ book.title }}</h2>
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
    </div>
    <button class="primary" :disabled="busy" @click="$emit('open', book.id)">
      接著讀
    </button>
  </section>
</template>
<style scoped>
.continue-panel {
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
.resume-cover {
  width: var(--resume-cover-width);
  height: var(--resume-cover-height);
  object-fit: cover;
  border-radius: var(--resume-cover-radius);
  box-shadow: var(--cover-shadow);
  flex: 0 0 auto;
}
.resume-info {
  flex: 1;
  min-width: 0;
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
    margin-left: auto;
  }
}
</style>
