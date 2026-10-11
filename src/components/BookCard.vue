<script setup lang="ts">
import { PhStar, PhPencilSimple, PhPlay, PhBookOpen } from "@phosphor-icons/vue";
import { onMounted, onUnmounted, ref, watch } from "vue";
import { loadCover } from "../api/covers";
import { progressPercent, statusLabels, videoStatusLabels, displayPath } from "../lib/library";
import type { LibraryBook } from "../api/library";

const props = defineProps<{
  book: LibraryBook;
  opening: boolean;
  favoritePending: boolean;
  selectable?: boolean;
  selected?: boolean;
  focused?: boolean;
  view?: "grid" | "detail" | "compact";
}>();
const emit = defineEmits<{
  open: [id: number];
  favorite: [book: LibraryBook];
  select: [id: number];
  edit: [book: LibraryBook];
}>();
const element = ref<HTMLElement | null>(null);
const titleElement = ref<HTMLElement | null>(null);
const titleMeasure = ref<HTMLElement | null>(null);
const titleWrapMeasure = ref<HTMLElement | null>(null);
const coverFrame = ref<HTMLElement | null>(null);
const longTitle = ref(false);
const expandedTitleStyle = ref<Record<string, string>>({});
let titleObserver: ResizeObserver | null = null;
const cover = ref<string | null>(null);
const coverFailed = ref(false);
let observer: IntersectionObserver | null = null;
let cancelled = false;
function activateCard() {
  if (props.opening) return;
  if (props.selectable) emit("select", props.book.id);
  else emit("edit", props.book);
}
function activateCover() {
  if (props.selectable) activateCard();
  else if (!props.opening && props.book.available) emit("open", props.book.id);
}
onMounted(() => {
  let measurementKey = "";
  const measureTitle = () => {
    if (!titleElement.value || !titleMeasure.value || !titleWrapMeasure.value || !coverFrame.value) return;
    const style = getComputedStyle(titleElement.value);
    const available = titleElement.value.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    longTitle.value = titleMeasure.value.getBoundingClientRect().width > available + 1;
    const baseFont = parseFloat(style.getPropertyValue("--title-base-font")) || parseFloat(style.fontSize);
    const lineRatio = parseFloat(style.getPropertyValue("--title-line-ratio")) || 1.5;
    const frameHeight = coverFrame.value.getBoundingClientRect().height;
    const key = JSON.stringify([props.book.title, available, baseFont, lineRatio, frameHeight]);
    if (key === measurementKey || available <= 0 || frameHeight <= 0) return;
    measurementKey = key;
    if (!longTitle.value) {
      expandedTitleStyle.value = {};
      return;
    }
    const budget = Math.max(baseFont * lineRatio, frameHeight * 0.45);
    const probe = titleWrapMeasure.value;
    probe.style.width = `${available}px`;
    let font = baseFont;
    let rows = 1;
    // Try readable sizes first; exceptionally long names retain a full native tooltip.
    for (const scale of [1, 0.95, 0.9, 0.85]) {
      font = baseFont * scale;
      probe.style.fontSize = `${font}px`;
      probe.style.lineHeight = `${font * lineRatio}px`;
      rows = Math.max(1, Math.round(probe.getBoundingClientRect().height / (font * lineRatio)));
      if (rows * font * lineRatio <= budget) break;
    }
    rows = Math.min(rows, Math.max(1, Math.floor(budget / (font * lineRatio))));
    const height = Math.max(baseFont * lineRatio, rows * font * lineRatio);
    expandedTitleStyle.value = {
      "--title-expanded-font": `${font}px`,
      "--title-expanded-extra": `${height - baseFont * lineRatio}px`,
      "--title-expanded-rows": `${rows}`,
    };
  };
  if (typeof ResizeObserver !== "undefined") {
    titleObserver = new ResizeObserver(measureTitle);
    if (titleElement.value) titleObserver.observe(titleElement.value);
    if (titleMeasure.value) titleObserver.observe(titleMeasure.value);
    if (coverFrame.value) titleObserver.observe(coverFrame.value);
    watch(() => props.book.title, measureTitle, { flush: "post" });
    measureTitle();
  }
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
  titleObserver?.disconnect();
  observer?.disconnect();
  if (cover.value) URL.revokeObjectURL(cover.value);
});
</script>

<template>
  <article
    ref="element"
    class="book-card"
    :title="book.title"
    :style="expandedTitleStyle"
    @click="activateCard"
    :class="[
      view ?? 'grid',
      { offline: !book.available, picked: selectable && selected, focused, 'long-title': longTitle },
    ]"
  >
    <button
      class="card-action"
      :disabled="opening"
      :aria-label="selectable ? `${selected ? '取消選取' : '選取'}：${book.title}` : `查看資訊：${book.title}`"
      :aria-pressed="selectable ? !!selected : undefined"
      @click.stop="activateCard"
    ></button>
    <label v-if="selectable" class="selection" @click.stop @keydown.stop>
      <input
        type="checkbox"
        :checked="selected"
        :disabled="opening"
        :aria-label="`${selected ? '取消選取' : '選取'}：${book.title}`"
        @change="emit('select', book.id)"
      />
      選取
    </label>
    <div ref="coverFrame" class="cover-frame">
      <button
        class="cover"
        :disabled="opening || (!selectable && !book.available)"
        :aria-label="selectable ? `${selected ? '取消選取' : '選取'}：${book.title}` : book.format === 'video' ? `使用預設播放器開啟：${book.title}` : `${book.lastReadAt ? '繼續閱讀' : '開始閱讀'}：${book.title}`"
        :aria-pressed="selectable ? !!selected : undefined"
        @click.stop="activateCover"
      >
        <img v-if="cover" :src="cover" alt="" />
        <div v-else class="placeholder">
          <small>{{
            !book.available
              ? "來源離線"
              : coverFailed
                ? "尚無可用封面"
                : "封面載入中"
          }}</small>
        </div>
      <span class="format" :class="{ video: book.format === 'video' }">{{
        book.format === "video" ? "影片" : book.format === "folder" ? "資料夾" : "CBZ / ZIP"
      }}</span>
      <span v-if="!selectable && book.available" class="cover-action"><component :is="book.format === 'video' ? PhPlay : PhBookOpen" :size="19" aria-hidden="true" />{{ book.format === 'video' ? '預設播放器' : book.lastReadAt ? '繼續閱讀' : '開始閱讀' }}</span>
    </button>
    </div>
    <div class="details">
      <h2 ref="titleElement" :title="book.title"><span>{{ book.title }}</span><span ref="titleMeasure" class="title-measure" aria-hidden="true">{{ book.title }}</span><span ref="titleWrapMeasure" class="title-wrap-measure" aria-hidden="true">{{ book.title }}</span></h2>
      <button
        class="favorite"
        :class="{ selected: book.favorite }"
        :aria-label="`${book.favorite ? '取消收藏' : '收藏'}：${book.title}`"
        :aria-pressed="book.favorite"
        :disabled="favoritePending"
        @click.stop="emit('favorite', book)"
      >
        <PhStar
          :weight="book.favorite ? 'fill' : 'regular'"
          :size="20"
          aria-hidden="true"
        />
      </button>
      <p v-if="book.series || book.volume" class="metadata">
        {{ book.series
        }}<span v-if="book.volume"> · 第 {{ book.volume }} 集</span>
      </p>
      <p v-if="book.tags.length" class="tags">
        <span v-for="tag in book.tags" :key="tag.id">{{ tag.name }}</span>
      </p>
      <p class="reading-state">
        {{ (book.format === 'video' ? videoStatusLabels : statusLabels)[book.readingStatus]
        }}{{ book.statusManual ? " · 手動標記" : "" }} ·
        {{ book.format === 'video' ? '外部播放器' : `${progressPercent(book)}%` }}
      </p>
      <p v-if="!book.available" class="offline-hint">
        來源無法存取 · 收藏與進度已保留
        <span class="source-path">{{ displayPath(book.path) }}</span>
      </p>
      <p v-else-if="book.format === 'video'" class="page-count">{{ book.lastReadAt ? '曾開啟 · 點封面播放' : '點封面使用預設播放器' }}</p>
      <p v-else class="page-count">
        {{
          book.lastReadAt
            ? `${book.lastIndex + 1} / ${book.pageCount} 頁`
            : `${book.pageCount} 頁 · 尚未閱讀`
        }}
      </p>
      <div
        v-if="book.lastReadAt && book.format !== 'video'"
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
      <button
        class="edit"
        :disabled="opening"
        :aria-label="`查看資訊：${book.title}`"
        @click.stop="emit('edit', book)"
      >
        <PhPencilSimple :size="14" aria-hidden="true" /> 查看資訊
      </button>
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
  position: relative;
  min-width: 0;
  background: var(--bg-soft);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
  cursor: pointer;
}
.card-action {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  background: transparent;
  border: 0;
  border-radius: inherit;
  cursor: pointer;
  z-index: 1;
}
.card-action:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.cover, .selection, .favorite, .edit {
  z-index: 2;
}
.selection, .edit {
  position: relative;
}
.cover {
  display: block;
  position: relative;
  width: 100%;
  aspect-ratio: 3 / 4;
  border: 0;
  padding: 0;
  background: var(--cover-bg);
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
  background: var(--panel);
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
  padding-right: 70px;
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
  width: 44px;
  height: 44px;
  background: transparent;
  border: 0;
  color: var(--text-dim);
  font-size: 12px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
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
.offline .cover img {
  opacity: 0.65;
}
.offline-hint {
  line-height: 1.6;
}

.detail .details {
  display: grid;
  grid-template-columns: minmax(160px, 2fr) minmax(140px, 1fr) minmax(
      140px,
      1fr
    );
  gap: 8px;
  align-items: start;
}
.detail h2 {
  padding-right: 0;
}
.detail .favorite {
  position: static;
  justify-self: end;
  grid-column: 3;
  grid-row: 1;
}
.detail .tags {
  grid-column: 2;
}
.detail .reading-state {
  grid-column: 3;
}
.detail .offline-hint {
  grid-column: 1 / -1;
}
.detail .edit {
  justify-self: start;
}
.detail p {
  margin-top: 0;
}
.focused { background:var(--bg-soft);box-shadow:inset 3px 0 var(--accent); }
.picked {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}
.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.tags span {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 2px 5px;
  max-width: 100%;
  overflow-wrap: anywhere;
}
.source-path {
  display: block;
  overflow-wrap: anywhere;
  font-size: 11px;
}
.edit {
  margin-top: 10px;
  padding: 5px 8px;
  white-space: nowrap;
  font-size: 11px;
}
.reading-state {
  color: var(--accent-soft);
}
.detail,
.compact {
  display: flex;
  align-items: center;
  gap: var(--space);
  padding: 12px;
}
.detail .cover {
  width: 76px;
  flex: 0 0 76px;
}
.compact .cover {
  width: 42px;
  flex: 0 0 42px;
}
.detail .details,
.compact .details {
  flex: 1;
  min-width: 0;
  padding: 4px;
}
.detail .format,
.compact .format {
  display: none;
}
.compact .details {
  display: grid;
  grid-template-columns:
    minmax(120px, 2fr) minmax(100px, 1fr) minmax(110px, 1fr)
    70px;
  gap: 8px;
  align-items: center;
}
.compact h2 {
  padding-right: 0;
  grid-column: 1;
  grid-row: 1;
}
.compact .metadata {
  grid-column: 2;
  grid-row: 1;
}
.compact .reading-state {
  grid-column: 3;
  grid-row: 1;
}
.compact .tags {
  grid-column: 1 / 4;
  grid-row: 2;
}
.compact .favorite {
  position: static;
  grid-column: 4;
  grid-row: 1;
}
.compact .edit {
  margin: 0;
  grid-column: 4;
  grid-row: 2;
}
.compact .metadata,
.compact .tags,
.compact .reading-state {
  margin: 0;
}
.compact .progress,
.compact
  .details
  > p:not(.metadata):not(.tags):not(.reading-state):not(.offline-hint) {
  display: none;
}
.compact .offline-hint {
  grid-column: 1 / -1;
}
.compact .selection {
  padding: 0;
}
@media (max-width: 1000px) {
  .detail .details {
    display: block;
  }
  .detail h2 {
    padding-right: 70px;
  }
  .detail .favorite {
    position: absolute;
  }
  .detail p {
    margin-top: 6px;
  }
}
@media (max-width: 700px) {
  .compact .details {
    display: block;
  }
  .compact .favorite {
    position: absolute;
  }
  .compact h2 {
    padding-right: 70px;
  }
  .compact .metadata,
  .compact .tags,
  .compact .reading-state {
    margin-top: 6px;
  }
}

/* Decorative surfaces use --line; interactive boundaries retain --border. */
.book-card {
  border-color: var(--line);
  background: var(--panel);
}
.grid {
  --title-base-font: 20px;
  --title-line-ratio: 1.5;
  align-self: start;
  background: var(--grid-surface);
  border-color: var(--grid-outline);
  padding: var(--grid-inset);
  overflow: visible;
}
.grid .cover {
  border-radius: calc(var(--radius) - 2px);
  overflow: hidden;
  box-shadow: var(--grid-cover-shadow);
}
.cover-frame {
  display: contents;
}
.grid {
  --title-line: 30px;
  --title-extra: 0px;
  --cover-zoom: 1;
}
.grid .cover-frame {
  display: block;
  position: relative;
  aspect-ratio: 3 / 4;
}
.grid .cover {
  position: absolute;
  inset: 0 0 auto;
  height: calc(100% - var(--title-extra));
  aspect-ratio: auto;
  transition: height .2s ease, box-shadow .2s ease;
}
.grid .cover img {
  position: absolute;
  top: 50%;
  height: calc(100% + var(--title-extra));
  transform: translateY(-50%) scale(var(--cover-zoom));
  transition: transform .2s ease;
}
.grid .details {
  margin-top: calc(-1 * var(--title-extra));
  transition: margin-top .2s ease, min-height .2s ease;
}
.title-measure, .title-wrap-measure {
  position: absolute;
  visibility: hidden;
  white-space: nowrap;
  width: max-content;
  pointer-events: none;
  font-size: var(--title-base-font, inherit);
  line-height: var(--title-line-ratio, inherit);
}
.title-wrap-measure {
  white-space: normal;
  overflow-wrap: anywhere;
}
h2 {
  position: relative;
}
.grid h2 {
  height: calc(var(--title-line) + var(--title-extra));
  transition: height .2s ease;
}
.grid h2 > span:first-child {
  display: block;
  line-height: var(--title-line-ratio);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.grid.long-title:focus-within {
  --title-extra: var(--title-expanded-extra, var(--title-line));
  --cover-zoom: 1.025;
}
.grid.long-title:focus-within h2 > span:first-child {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: var(--title-expanded-rows, 2);
  font-size: var(--title-expanded-font, var(--title-base-font));
  white-space: normal;
  overflow-wrap: anywhere;
}
@media (hover: hover) and (pointer: fine) {
  .grid.long-title:hover {
    --title-extra: var(--title-expanded-extra, var(--title-line));
    --cover-zoom: 1.025;
  }
  .grid.long-title:hover h2 > span:first-child {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: var(--title-expanded-rows, 2);
    font-size: var(--title-expanded-font, var(--title-base-font));
    white-space: normal;
    overflow-wrap: anywhere;
  }
}
@media (prefers-reduced-motion: reduce) {
  .grid .cover, .grid .cover img, .grid .details, .grid h2 {
    transition: none;
  }
}
.grid .details {
  padding: var(--grid-details-space);
}
.grid h2 {
  font-family: var(--heading-font);
  font-size: 20px;
  line-height: 1.5;
  font-weight: 600;
  padding-right: 44px;
}
.grid p {
  margin-top: 10px;
  line-height: 1.6;
}
.grid .favorite {
  top: 18px;
  right: 8px;
}
.grid .format {
  display: none;
}
.grid .format.video { display: block; }
.grid .edit {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border-color: transparent;
  background: transparent;
  color: var(--text-dim);
  padding-left: 0;
}
.progress {
  background: var(--line);
  height: 4px;
}
.tags span {
  border: 0;
  background: var(--bg-soft);
  border-radius: 5px;
  padding: 3px 7px;
}
.detail,
.compact {
  border: 0;
  border-bottom: 1px solid var(--line);
  border-radius: 0;
  background: transparent;
  padding: 18px 12px;
}
.detail .details {
  grid-template-columns: minmax(130px, 1.4fr) minmax(90px, 1fr) minmax(
      120px,
      1fr
    ) 68px;
  gap: 8px 20px;
  align-items: center;
}
.detail h2 {
  grid-column: 1;
  grid-row: 1;
  font-size: 16px;
}
.detail .metadata {
  grid-column: 2;
  grid-row: 1;
}
.detail .tags {
  grid-column: 2;
  grid-row: 2;
}
.detail .reading-state {
  grid-column: 3;
  grid-row: 1;
}
.detail .page-count {
  grid-column: 3;
  grid-row: 2;
}
.detail .progress {
  grid-column: 3;
  grid-row: 3;
  margin-top: 0;
}
.detail .favorite {
  grid-column: 4;
  grid-row: 1;
}
.detail .edit {
  grid-column: 4;
  grid-row: 2;
  margin: 0;
  border-color: transparent;
  background: transparent;
  padding: 5px 0;
}
.detail .offline-hint {
  grid-column: 1 / 4;
  grid-row: 3;
}
.compact {
  padding: 10px 12px;
}
.picked {
  background: var(--bg-soft);
  box-shadow: inset 3px 0 var(--accent);
}
@media (max-width: 1100px) {
  .detail .details {
    display: block;
    position: relative;
  }
  .detail h2 {
    padding-right: 70px;
  }
  .detail .favorite {
    position: absolute;
    top: 0;
    right: 0;
  }
  .detail .edit {
    margin-top: 8px;
  }
  .detail .progress {
    margin-top: 10px;
  }
}
</style>
