<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue";
import { PhX } from "@phosphor-icons/vue";
import LibraryGuide from "./LibraryGuide.vue";
const emit = defineEmits<{ close: [] }>();
const dialog = ref<HTMLDialogElement>();
let previousFocus: HTMLElement | null = null;
onMounted(() => {
  previousFocus = document.activeElement as HTMLElement | null;
  dialog.value?.showModal();
});
onBeforeUnmount(() => {
  dialog.value?.close();
  const target = previousFocus?.isConnected ? previousFocus : document.querySelector<HTMLElement>('[aria-label="開啟教學"]');
  target?.focus({ preventScroll: true });
});
</script>
<template>
  <dialog ref="dialog" class="tutorial-dialog" aria-labelledby="tutorial-title" @cancel.prevent="emit('close')">
    <header><div><p>MangaFolio</p><h1 id="tutorial-title">使用教學</h1></div><button autofocus aria-label="關閉教學" @click="emit('close')"><PhX :size="22" aria-hidden="true" /></button></header>
    <div class="tutorial-content"><LibraryGuide embedded standalone @complete="emit('close')" /></div>
    <footer><span>教學可隨時從書庫上方重新開啟。</span><button @click="emit('close')">回到書庫</button></footer>
  </dialog>
</template>
<style scoped>
.tutorial-dialog { margin: auto; width: min(880px, calc(100vw - 32px)); max-height: calc(100dvh - 40px); padding: 0; border: 1px solid var(--line); border-radius: 16px; background: var(--panel); color: var(--text); box-shadow: 0 24px 80px #0005; }
.tutorial-dialog[open] { display: flex; flex-direction: column; }
dialog::backdrop { background: #0009; backdrop-filter: blur(4px); }
header, footer { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 20px 28px; flex-shrink: 0; }
header { border-bottom: 1px solid var(--line); } header p { font-size: 12px; color: var(--text-dim); margin: 0 0 4px; } h1 { font-size: 25px; }
header button { display: flex; padding: 10px; background: transparent; border: 0; }
.tutorial-content { padding: 24px; overflow: auto; min-height: 0; }
footer { border-top: 1px solid var(--line); font-size: 12px; color: var(--text-dim); }
@media (max-width: 600px) { header, footer { padding: 16px; } footer { flex-wrap: wrap; } .tutorial-content { padding: 12px; } }
</style>
