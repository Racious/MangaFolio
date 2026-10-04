<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";

import { useAppearanceStore } from "../stores/appearance";
const appearance = useAppearanceStore();
const pinned = ref(appearance.settings.readerPinned);
const REVEAL_DISTANCE = 80;
const HIDE_DELAY = 400;
type Edge = "top" | "bottom";
const revealed = ref({ top: false, bottom: false });
const hovered = { top: false, bottom: false };
const hideTimers: Partial<Record<Edge, ReturnType<typeof setTimeout>>> = {};
function cancelHide(edge: Edge) {
  clearTimeout(hideTimers[edge]);
  delete hideTimers[edge];
}
function reveal(edge: Edge) {
  cancelHide(edge);
  revealed.value[edge] = true;
}
function deferHide(edge: Edge) {
  if (hovered[edge] || !revealed.value[edge] || hideTimers[edge] !== undefined) return;
  hideTimers[edge] = setTimeout(() => {
    revealed.value[edge] = false;
    delete hideTimers[edge];
  }, HIDE_DELAY);
}
function onPointerMove(event: PointerEvent) {
  if (event.pointerType === "touch") return;
  const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
  for (const edge of ["top", "bottom"] as const) {
    const distance = edge === "top" ? event.clientY - bounds.top : bounds.bottom - event.clientY;
    if (distance >= 0 && distance <= REVEAL_DISTANCE) reveal(edge);
    else deferHide(edge);
  }
}
function chromeEnter(edge: Edge) {
  hovered[edge] = true;
  reveal(edge);
}
function chromeLeave(edge: Edge) {
  hovered[edge] = false;
  deferHide(edge);
}
function readerLeave() {
  chromeLeave("top");
  chromeLeave("bottom");
}
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
onUnmounted(() => {
  window.removeEventListener("keydown", onKey);
  cancelHide("top");
  cancelHide("bottom");
});
</script>

<template>
  <section class="immersive-reader" :class="{ pinned }" aria-label="閱讀區域"
    @pointermove="onPointerMove" @pointerleave="readerLeave">
    <slot />
    <div class="reader-edge top" :class="{ revealed: revealed.top }" aria-label="上方閱讀工具列">
      <div class="reader-chrome" @pointerenter="chromeEnter('top')" @pointerleave="chromeLeave('top')"><slot name="top" /></div>
    </div>
    <div class="reader-edge bottom" :class="{ revealed: revealed.bottom }" aria-label="下方閱讀工具列">
      <div class="reader-chrome" @pointerenter="chromeEnter('bottom')" @pointerleave="chromeLeave('bottom')"><slot name="bottom" /></div>
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
  pointer-events: none;
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
.reader-edge.revealed .reader-chrome,
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
