<script setup lang="ts">
import Toolbar from "./components/Toolbar.vue";
import ReaderView from "./components/ReaderView.vue";
import PageScrubber from "./components/PageScrubber.vue";
import { useReaderStore } from "./stores/reader";

// 鍵盤導航與捲動由 ReaderView 處理（其持有捲動容器參照）。
const reader = useReaderStore();
</script>

<template>
  <div class="app">
    <Toolbar />
    <ReaderView />
    <PageScrubber />
    <p v-if="reader.error" class="error-bar">{{ reader.error }}</p>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
}
.error-bar {
  flex: 0 0 auto;
  background: rgba(217, 138, 138, 0.12);
  color: var(--red);
  border-top: 1px solid var(--border);
  padding: 8px 14px;
  font-size: 13px;
  margin: 0;
}
</style>
