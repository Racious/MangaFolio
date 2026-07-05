<script setup lang="ts">
// 更新提示浮層：發現新版時彈出，呈現版本、發行說明、下載進度與操作。
import { computed } from "vue";
import { useUpdateStore } from "../stores/update";

const update = useUpdateStore();

// 有新版可裝、正在安裝、或（手動檢查）出錯時顯示。
const visible = computed(
  () => update.updateAvailable || update.installing || !!update.errorMessage,
);
</script>

<template>
  <div v-if="visible" class="overlay">
    <div class="card">
      <template v-if="update.errorMessage && !update.installing">
        <h3 class="title err">更新失敗</h3>
        <p class="msg">{{ update.errorMessage }}</p>
        <div class="actions">
          <button class="btn" @click="update.openReleasePage()">前往下載頁</button>
          <button class="btn ghost" @click="update.dismiss()">關閉</button>
        </div>
      </template>

      <template v-else>
        <h3 class="title">發現新版本</h3>
        <p class="ver">
          <span class="dim">{{ update.currentVersion }}</span>
          <span class="arrow">→</span>
          <span class="accent">{{ update.latestVersion }}</span>
        </p>

        <div v-if="update.releaseNotes" class="notes">{{ update.releaseNotes }}</div>

        <div v-if="update.installing" class="progress-area">
          <div class="bar">
            <div
              class="fill"
              :style="{ width: (update.downloadProgress ?? 0) + '%' }"
            ></div>
          </div>
          <p class="status">
            {{ update.statusMessage }}
            <span v-if="update.downloadProgress !== null">（{{ update.downloadProgress }}%）</span>
          </p>
        </div>

        <div v-else class="actions">
          <button class="btn primary" @click="update.installUpdate()">立即更新並重啟</button>
          <button class="btn ghost" @click="update.dismiss()">稍後</button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.55);
}
.card {
  width: min(440px, 90vw);
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 22px 24px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.4);
}
.title {
  margin: 0 0 12px;
  font-size: 16px;
  color: var(--accent-soft);
  font-weight: 600;
}
.title.err {
  color: var(--red);
}
.ver {
  margin: 0 0 14px;
  font-size: 15px;
  font-variant-numeric: tabular-nums;
}
.dim {
  color: var(--text-dim);
}
.arrow {
  margin: 0 8px;
  color: var(--text-dim);
}
.accent {
  color: var(--accent-soft);
  font-weight: 600;
}
.notes {
  max-height: 180px;
  overflow-y: auto;
  background: var(--bg-soft);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  margin-bottom: 16px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--text);
  white-space: pre-wrap;
}
.msg {
  font-size: 13px;
  color: var(--text);
  line-height: 1.6;
  margin: 0 0 16px;
  word-break: break-word;
}
.progress-area {
  margin-top: 4px;
}
.bar {
  height: 8px;
  background: var(--bg-soft);
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}
.fill {
  height: 100%;
  background: var(--accent);
  transition: width 0.2s ease;
}
.status {
  margin: 10px 0 0;
  font-size: 13px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}
.actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}
.btn {
  background: var(--panel);
  border: 1px solid var(--border);
  color: var(--text);
  padding: 8px 16px;
  border-radius: 7px;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.btn:hover {
  border-color: var(--accent);
  color: var(--accent-soft);
}
.btn.ghost {
  background: transparent;
}
.btn.primary {
  border-color: var(--accent);
  color: var(--accent-soft);
  background: rgba(200, 169, 106, 0.12);
}
</style>
