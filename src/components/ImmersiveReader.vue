<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";

import { useAppearanceStore } from "../stores/appearance";
const appearance = useAppearanceStore();
const pinned = ref(appearance.settings.readerPinned);
watch(
  () => appearance.settings.readerPinned,
  (value) => {
    pinned.value = value;
  },
);
function onKey(event: KeyboardEvent) {
  if (
    event.key !== "Escape" ||
    event.isComposing ||
    event.ctrlKey ||
    event.metaKey ||
    event.altKey
  )
    return;
  if (
    document.querySelector("dialog[open], [role='dialog']:not(dialog)") ||
    (event.target instanceof Element &&
      event.target.closest("input, textarea, select, [contenteditable]"))
  )
    return;
  pinned.value = !pinned.value;
}
onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <section class="immersive-reader" :class="{ pinned }" aria-label="閱讀區域">
    <slot />
    <div class="reader-edge top" aria-label="上方閱讀工具列">
      <div class="reader-chrome"><slot name="top" /></div>
    </div>
    <div class="reader-edge bottom" aria-label="下方閱讀工具列">
      <div class="reader-chrome"><slot name="bottom" /></div>
    </div>
    <button
      class="touch-controls"
      :aria-pressed="pinned"
      @click="pinned = !pinned"
      aria-label="顯示或隱藏閱讀工具列"
    >
      工具列
    </button>
    <div class="reader-alerts"><slot name="alerts" /></div>
  </section>
</template>

<style scoped>
.immersive-reader {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}
.reader-edge {
  position: absolute;
  left: 0;
  right: 0;
  z-index: 40;
  height: 24px;
}
.top {
  top: 0;
}
.bottom {
  bottom: 0;
}
.reader-chrome {
  position: absolute;
  left: 0;
  right: 0;
  opacity: 0;
  pointer-events: none;
  transition:
    opacity 0.16s ease,
    transform 0.16s ease;
}
.top .reader-chrome {
  top: 0;
  transform: translateY(-100%);
}
.bottom .reader-chrome {
  bottom: 0;
  transform: translateY(100%);
}
.reader-edge:hover .reader-chrome,
.reader-edge:focus-within .reader-chrome,
.pinned .reader-chrome {
  position: absolute;
  left: 0;
  right: 0;
  opacity: 1;
  transform: translateY(0);
  pointer-events: auto;
}
.reader-alerts {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 50;
}
.reader-alerts:empty {
  display: none;
}
.touch-controls {
  display: none;
  position: absolute;
  right: 12px;
  top: 12px;
  z-index: 45;
}
@media (hover: none), (pointer: coarse) {
  .touch-controls {
    display: block;
  }
  .pinned .touch-controls {
    top: auto;
    bottom: 64px;
  }
}
</style>
