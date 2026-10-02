<script setup lang="ts">
import { computed, ref, watch } from "vue";

const emit = defineEmits<{ highlight: [target: string] }>();
const steps = [
  {
    target: "import",
    title: "加入第一本漫畫",
    text: "點「＋ 加入 ZIP／CBZ」選取一本或多本漫畫；也可以加入直接包含圖片的資料夾。原始檔案會留在原位。",
    check: "加入後，下方會出現封面卡片。",
  },
  {
    target: "books",
    title: "從封面開始閱讀",
    text: "點封面開啟漫畫，使用工具列或方向鍵翻頁。點閱讀工具列左上角的「書庫」即可回到這裡。",
    check: "回到書庫後，卡片會顯示頁碼與閱讀進度。",
  },
  {
    target: "favorites",
    title: "收藏喜歡的作品",
    text: "點書籍卡片的 ☆，變成 ★ 就代表已收藏。再點「我的收藏」查看；閱讀工具列也可以切換收藏。",
    check: "再次點星號可取消收藏，收藏數量會同步更新。",
  },
  {
    target: "search",
    title: "搜尋與排序書籍",
    text: "在「搜尋書名…」輸入部分書名，再選「書名排序」或「最近閱讀／加入」。搜尋也能搭配收藏與最近閱讀篩選。",
    check: "支援大小寫與全形／半形；清除搜尋可恢復顯示。",
  },
  {
    target: "recent",
    title: "接著上次的進度讀",
    text: "先開一本書、翻到其他頁，再返回書庫。首頁的「繼續閱讀」會帶你回到上次位置，「最近閱讀」則列出讀過的書。",
    check: "正常關閉再開啟，收藏、進度與閱讀設定仍會保留在這台電腦。",
  },
];
function readSetting(key: string) {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}
function saveSetting(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* The guide remains usable without storage. */
  }
}
const visible = ref(
  readSetting("mangafolio.library-guide.v1.hidden") !== "true",
);
const savedIndex = Number(readSetting("mangafolio.library-guide.v1.step"));
const index = ref(
  Number.isInteger(savedIndex) && savedIndex >= 0 && savedIndex < steps.length
    ? savedIndex
    : 0,
);
const current = computed(() => steps[index.value]!);
watch(
  [visible, index],
  () => {
    emit("highlight", visible.value ? current.value.target : "");
    saveSetting("mangafolio.library-guide.v1.step", String(index.value));
  },
  { immediate: true },
);
function hide() {
  visible.value = false;
  saveSetting("mangafolio.library-guide.v1.hidden", "true");
}
function reopen() {
  index.value = 0;
  visible.value = true;
  saveSetting("mangafolio.library-guide.v1.hidden", "false");
}
</script>

<template>
  <section class="guide" aria-labelledby="guide-title">
    <div class="guide-header">
      <h2 id="guide-title">新功能教學</h2>
      <button v-if="visible" @click="hide">收合教學</button>
      <button v-else @click="reopen">開始教學 →</button>
    </div>
    <template v-if="visible">
      <nav class="steps" aria-label="教學步驟">
        <button
          v-for="(step, position) in steps"
          :key="step.target"
          :aria-current="position === index ? 'step' : undefined"
          :class="{ selected: position === index }"
          @click="index = position"
        >
          {{ position + 1 }}. {{ step.title }}
        </button>
      </nav>
      <div class="guide-content" aria-live="polite" aria-atomic="true">
        <span class="count"
          >第 {{ index + 1 }} 步／共 {{ steps.length }} 步 ·
          金色框線標示對應區域</span
        >
        <h3>{{ current.title }}</h3>
        <p>{{ current.text }}</p>
        <p class="check">確認效果：{{ current.check }}</p>
      </div>
      <div class="guide-actions">
        <button :disabled="index === 0" @click="index--">上一步</button>
        <button
          v-if="index < steps.length - 1"
          class="primary"
          @click="index++"
        >
          下一步 →
        </button>
        <button v-else class="primary" @click="hide">完成教學</button>
      </div>
    </template>
    <p v-else class="collapsed">
      加入、閱讀、收藏、搜尋、續讀，五步認識你的書庫。
    </p>
  </section>
</template>

<style scoped>
.guide {
  padding: 18px 20px;
  margin-bottom: 24px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-soft);
}
.guide-header,
.guide-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  justify-content: space-between;
}
h2 {
  font-size: 16px;
}
button {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
.steps {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin: 16px 0;
}
.steps .selected {
  color: var(--accent-soft);
  border-color: var(--accent);
}
.count {
  color: var(--text-dim);
  font-size: 11px;
}
h3 {
  margin: 10px 0 8px;
  font-size: 18px;
}
p {
  font-size: 13px;
  color: var(--text);
  line-height: 1.8;
}
.check {
  margin-top: 8px;
  color: var(--accent-soft);
}
.guide-actions {
  justify-content: flex-end;
  margin-top: 16px;
}
.primary {
  background: var(--accent);
  color: #17191d;
  border-color: var(--accent);
}
.collapsed {
  color: var(--text-dim);
  margin-top: 8px;
}
@media (max-width: 600px) {
  .guide {
    padding: 14px;
  }
  .steps button {
    flex: 1 1 150px;
    text-align: left;
  }
}
</style>
