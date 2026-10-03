<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { loadCover } from "../api/covers";
import type { LibraryBook } from "../api/library";

const props = defineProps<{
  book: LibraryBook;
  opening: boolean;
  favoritePending: boolean;
  selectable?: boolean;
  selected?: boolean;
}>();
const emit = defineEmits<{
  open: [id: number];
  favorite: [book: LibraryBook];
  select: [id: number];
}>();
const element = ref<HTMLElement | null>(null);
const cover = ref<string | null>(null);
const coverFailed = ref(false);
let observer: IntersectionObserver | null = null;
let cancelled = false;
onMounted(() => {
  if (!props.book.available) return;
  observer = new IntersectionObserver(
    (entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      observer?.disconnect();
      loadCover(props.book.id, () => cancelled)
        .then((url) => {
          if (cancelled) {
            if (url) URL.revokeObjectURL(url);
            return;
          }
          cover.value = url;
        })
        .catch(() => {
          if (!cancelled) coverFailed.value = true;
        });
    },
    { rootMargin: "150px" },
  );
  if (element.value) observer.observe(element.value);
});
onUnmounted(() => {
  cancelled = true;
  observer?.disconnect();
  if (cover.value) URL.revokeObjectURL(cover.value);
});
</script>

<template>
  <article
    ref="element"
    class="book-card"
    :class="{ offline: !book.available }"
  >
    <label v-if="selectable" class="selection">
      <input
        type="checkbox"
        :checked="selected"
        :disabled="opening"
        :aria-label="`選取：${book.title}`"
        @change="emit('select', book.id)"
      />
      選取
    </label>
    <button
      class="cover"
      :disabled="opening || !book.available"
      :aria-label="`${book.lastReadAt ? '繼續閱讀' : '開始閱讀'}：${book.title}`"
      @click="emit('open', book.id)"
    >
      <img v-if="cover" :src="cover" alt="" />
      <div v-else class="placeholder">
        <span aria-hidden="true">漫</span
        ><small>{{
          !book.available
            ? "來源離線"
            : coverFailed
              ? "無法載入封面"
              : "封面載入中"
        }}</small>
      </div>
      <span class="format">{{
        book.format === "folder" ? "資料夾" : "CBZ / ZIP"
      }}</span>
    </button>
    <div class="details">
      <h2 :title="book.title">{{ book.title }}</h2>
      <button
        class="favorite"
        :class="{ selected: book.favorite }"
        :aria-label="`${book.favorite ? '取消收藏' : '收藏'}：${book.title}`"
        :aria-pressed="book.favorite"
        :disabled="favoritePending"
        @click="emit('favorite', book)"
      >
        {{ book.favorite ? "★" : "☆" }}
      </button>
      <p v-if="!book.available" class="offline-hint">
        找不到原始檔案，收藏與進度已保留
      </p>
      <p v-else>
        {{
          book.lastReadAt
            ? `${book.lastIndex + 1} / ${book.pageCount} 頁`
            : `${book.pageCount} 頁 · 尚未閱讀`
        }}
      </p>
      <div
        v-if="book.lastReadAt"
        class="progress"
        role="img"
        :aria-label="`閱讀進度 ${book.lastIndex + 1} / ${book.pageCount} 頁`"
      >
        <span
          :style="{
            width: `${Math.min(100, ((book.lastIndex + 1) / book.pageCount) * 100)}%`,
          }"
        ></span>
      </div>
    </div>
  </article>
</template>

<style scoped>
.selection {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px;
  font-size: 12px;
}
.selection input {
  accent-color: var(--accent);
  width: 16px;
  height: 16px;
}
.book-card {
  min-width: 0;
  background: var(--bg-soft);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}
.cover {
  display: block;
  position: relative;
  width: 100%;
  aspect-ratio: 3 / 4;
  border: 0;
  padding: 0;
  background: #101318;
  cursor: pointer;
}
.cover:disabled {
  cursor: default;
}
.cover img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}
.placeholder {
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 12px;
  color: var(--text-dim);
}
.placeholder span {
  font-size: 64px;
  font-weight: bold;
  color: var(--border);
}
.placeholder small {
  font-size: 12px;
}
.format {
  position: absolute;
  left: 10px;
  bottom: 10px;
  border-radius: 4px;
  background: #14171ce6;
  color: var(--text);
  padding: 4px 6px;
  font-size: 10px;
}
.details {
  padding: 14px;
  position: relative;
}
h2 {
  font-size: 14px;
  font-weight: 600;
  padding-right: 28px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
p {
  color: var(--text-dim);
  font-size: 12px;
  margin-top: 8px;
}
.favorite {
  position: absolute;
  top: 7px;
  right: 7px;
  width: 36px;
  height: 36px;
  background: transparent;
  border: 0;
  color: var(--text-dim);
  font-size: 23px;
  cursor: pointer;
}
.favorite.selected {
  color: var(--accent-soft);
}
.progress {
  height: 3px;
  border-radius: 2px;
  background: var(--border);
  margin-top: 12px;
  overflow: hidden;
}
.progress span {
  display: block;
  height: 100%;
  background: var(--accent);
}
.offline .cover {
  opacity: 0.65;
}
.offline-hint {
  line-height: 1.6;
}
</style>
