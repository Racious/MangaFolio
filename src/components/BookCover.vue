<script setup lang="ts">
import { onUnmounted, ref, watch } from "vue";
import { loadCover } from "../api/covers";
const props = defineProps<{ id: number; available: boolean; title: string }>();
const cover = ref<string | null>(null);
let disposed = false,
  generation = 0;
watch(
  () => [props.id, props.available] as const,
  () => {
    const token = ++generation;
    if (cover.value) URL.revokeObjectURL(cover.value);
    cover.value = null;
    if (!props.available) return;
    loadCover(props.id, () => disposed || token !== generation)
      .then((url) => {
        if (disposed || token !== generation) {
          if (url) URL.revokeObjectURL(url);
        } else cover.value = url;
      })
      .catch(() => {});
  },
  { immediate: true },
);
onUnmounted(() => {
  disposed = true;
  generation++;
  if (cover.value) URL.revokeObjectURL(cover.value);
});
</script>
<template>
  <span class="book-cover"
    ><img v-if="cover" :src="cover" alt="" /><span v-else>{{
      available ? title : "來源離線"
    }}</span></span
  >
</template>
<style scoped>
.book-cover {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--cover-bg);
  aspect-ratio: 3/4;
  border-radius: var(--radius);
  overflow: hidden;
  min-width: 0;
}
img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}
span span {
  padding: 16px;
  color: var(--text-dim);
  font-family: var(--heading-font);
  text-align: center;
  overflow-wrap: anywhere;
}
</style>
