<script setup lang="ts">
import { computed, ref, watch } from "vue";
const props = defineProps<{ embedded?: boolean; standalone?: boolean }>();

const emit = defineEmits<{ highlight: [target: string]; complete: [] }>();
const guide = ref<HTMLElement>();
interface GuideTask { title: string; instructions: string[]; image?: string; caption?: string }
const steps: { target: string; title: string; tasks: GuideTask[]; check: string }[] = [
  { target: 'import', title: '加入漫畫與影片', tasks: [{ title: '加入一筆作品', instructions: ['回到書庫，點「加入作品」。', '選「漫畫／影片檔案」加入 ZIP、CBZ 或影片；圖片資料夾則選「批次加入資料夾」。', '選取來源；若出現匯入預覽，確認勾選項目後按「匯入所選」。'] }], check: '書庫出現作品封面；原始檔案仍留在原位。' },
  { target: 'books', title: '從封面開始閱讀', tasks: [{ title: '開啟與返回', instructions: ['點漫畫封面進入閱讀器；點影片封面會開啟系統預設播放器。', '漫畫用方向鍵翻頁。將滑鼠移到畫面上緣或下緣，或按 Esc 顯示工具列。', '點工具列的「書庫」返回；影片則關閉播放器，回到 MangaFolio。'] }], check: '漫畫卡片顯示閱讀進度；影片不會自動取得播放器的觀看進度。' },
  { target: 'favorites', title: '收藏喜歡的作品', tasks: [{ title: '加入收藏', instructions: ['找到想收藏的作品，點卡片上的星號。', '在書庫導覽選「我的收藏」（部分介面顯示「收藏」）。', '想取消收藏時，再點一次該作品的星號。'] }], check: '星號亮起代表已收藏；收藏清單會同步更新。' },
  { target: 'search', title: '搜尋與排序書籍', tasks: [{ title: '找出需要的作品', instructions: ['在書庫搜尋欄輸入部分作品名稱、系列、類別或備註。', '在排序選單選「書名排序」、「最近開啟／加入」或「系列／集數排序」。', '要看全部作品，清空搜尋文字，並清除已套用的篩選。'] }], check: '只顯示符合條件的作品；排序改變不會移除書庫紀錄。' },
  { target: 'recent', title: '接著上次的進度讀', tasks: [{ title: '續讀漫畫', instructions: ['先開一本漫畫，翻到想停下來的頁面，再返回書庫。', '在「繼續閱讀」點該作品；也可從「最近開啟」（部分介面顯示「最近」）找到它。', '讀完後再返回書庫，保存新的進度。'] }], check: '下次開啟同一本漫畫會回到上次位置；影片由外部播放器處理續播。' },
  { target: 'books', title: '系列書架與下一集', tasks: [{ title: '整理同一系列', instructions: ['點「管理作品」，勾選同一系列的漫畫。', '在「批次系列」輸入系列名稱，按「指定系列」；再逐本「編輯資訊」填寫集數。', '回到書庫，在系列書架點系列封面查看各集；作品資訊的「下一集」可開啟下一本。'] }], check: '漫畫依系列與集數整理。缺值、重複或跳號時，請自行選擇下一本。' },
  { target: 'status', title: '閱讀狀態與標籤', tasks: [
    { title: '先建立標籤', instructions: ['關閉教學，點「設定」→「類別與標籤」。', '在「標籤名稱」輸入一個名稱，例如「科幻」（1–64 字）。', '按「建立標籤」，確認「現有標籤」選單已出現它，再關閉設定。'], image: '/guide/create-tag.jpg', caption: '建立標籤：設定 → 類別與標籤 → 輸入名稱 → 建立標籤' },
    { title: '再把標籤加到作品', instructions: ['回到書庫，點「管理作品」，勾選一本或多本作品。', '在「選擇標籤」選單選剛建立的「科幻」，按「批次加入標籤」。', '結束管理後，從「類別／標籤」篩選該標籤，確認這些作品出現在清單。'], image: '/guide/assign-tag.jpg', caption: '套用標籤：管理作品 → 勾選作品 → 選擇標籤 → 批次加入標籤' },
    { title: '手動調整閱讀／觀看狀態', instructions: ['在「管理作品」勾選要調整的作品。', '點「標記已讀」或「標記未讀」；選到影片時，按鈕會顯示「已讀／已看」或「未讀／未看」。', '漫畫要恢復自動狀態，按「依進度判定」；影片請手動標記。'] },
  ], check: '建立標籤後仍需套用到作品；多個標籤逐一加入即可。移除用「批次移除標籤」，手動改狀態不清除續讀位置。' },
  { target: 'books', title: '編輯書籍資訊', tasks: [{ title: '修改顯示資訊', instructions: ['點作品名稱，開啟作品資訊。', '點「編輯資訊」，填入自訂書名、系列、集數或備註。', '按「保存資訊」，確認書庫顯示更新後的內容。'] }], check: '只變更書庫顯示資訊，來源檔案不會更名。標籤請使用「管理作品」設定。' },
  { target: 'books', title: '找到原始檔案與安全移除', tasks: [
    { title: '開啟來源目錄', instructions: ['點作品名稱，開啟作品資訊；資訊書架可先展開「更多作品資訊」。', '找到「來源」，點路徑旁的資料夾圖示。', '檔案總管開啟圖片資料夾，或選取 ZIP、CBZ、影片檔案。'], image: '/guide/source-location.jpg', caption: '來源目錄：作品資訊 → 來源路徑旁的資料夾按鈕' },
    { title: '從書庫移除', instructions: ['點「管理作品」，勾選要移除的作品。', '點「移除所選」，閱讀確認視窗列出的移除範圍。', '確認後移除書庫紀錄；若要保留進度等資訊，請先匯出備份。'] },
  ], check: '移除所選不會刪除原始檔案，但會移除該作品的收藏、進度、標籤關聯、書籤及自訂資訊。' },
  { target: 'management', title: '批次匯入與備份', tasks: [{ title: '一次加入多筆作品', instructions: ['點「加入作品」→「批次加入資料夾」；要遞迴掃描整個資料夾樹，選「加入主目錄」。', '在預覽清單確認路徑與警告，勾選要加入的作品，再按「匯入所選」。', '主目錄後續新增作品時，點「設定」→「書庫來源」，重新掃描已加入的主目錄。'] }], check: '匯入完成後，書庫會出現新作品；來源失效時選取一本，使用「重新指定來源」或「重新指定影片」。備份操作見下一個主題。' },
  { target: 'management', title: '安全備份與書籤', tasks: [
    { title: '匯出與還原備份', instructions: ['點「設定」→「資料與備份」，展開「備份與還原」。', '點「匯出書庫備份」，選儲存位置；原始漫畫與影片請另外保管。', '需要還原時，點「還原書庫備份」，選備份檔，檢查預覽後再確認。'] },
    { title: '在漫畫加書籤', instructions: ['開啟漫畫，翻到想記下來的頁面。', '顯示閱讀工具列，點「書籤／筆記」，輸入名稱與筆記；雙頁時先選要標記的頁面。', '按「新增書籤」；之後從書籤清單點名稱即可回到該頁。'] },
  ], check: '書庫備份包含收藏、進度、標籤、書籤與人工封面；不含原始媒體、自動封面、外觀及本機主目錄。' },
  { target: 'appearance', title: '選擇自己的介面', tasks: [{ title: '調整外觀', instructions: ['點「設定」→「外觀與閱讀」。', '選喜歡的介面、配色與明暗；再調整密度、封面尺寸。', '關閉設定，回到書庫查看效果；不喜歡時再回來更換。'] }], check: '外觀選擇保存在這台電腦，切換介面會保留搜尋、篩選、排序與有效選取。' },
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
  props.standalone || readSetting("mangafolio.library-guide.v1.hidden") === "false",
);
const savedIndex = Number(readSetting("mangafolio.library-guide.v1.step"));
const index = ref(
  !props.standalone && Number.isInteger(savedIndex) && savedIndex >= 0 && savedIndex < steps.length
    ? savedIndex
    : 0,
);
const current = computed(() => steps[index.value]!);
watch(index, () => {
  if (props.standalone && guide.value?.parentElement) guide.value.parentElement.scrollTop = 0;
}, { flush: 'post' });
watch(
  [visible, index],
  () => {
    if (!props.embedded) emit("highlight", visible.value ? current.value.target : "");
    if (!props.standalone) saveSetting("mangafolio.library-guide.v1.step", String(index.value));
  },
  { immediate: true },
);
function hide() {
  if (props.standalone) { emit("complete"); return; }
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
  <section
    ref="guide"
    class="guide"
    :class="{ collapsed: !visible }"
    aria-labelledby="guide-title"
  >
    <div class="guide-header">
      <h2 id="guide-title">{{ standalone ? '從加入作品到日常使用' : '使用教學' }}</h2>
      <button v-if="visible && !standalone" @click="hide">收合教學</button>
      <button v-else-if="!visible" @click="reopen">開始教學 →</button>
    </div>
    <template v-if="visible">
      <nav class="steps" aria-label="教學主題">
        <button
          v-for="(step, position) in steps"
          :key="step.title"
          :aria-current="position === index ? 'step' : undefined"
          :class="{ selected: position === index }"
          @click="index = position"
        >
          {{ position + 1 }}. {{ step.title }}
        </button>
      </nav>
      <div class="guide-content" aria-live="polite" aria-atomic="true">
        <span class="count"
          >主題 {{ index + 1 }}／共 {{ steps.length }} 個 ·
          {{ standalone ? '可選主題，也可依序閱讀；關閉後回到書庫操作' : embedded ? '關閉設定後，依下方說明操作書庫' : '重點框線標示對應區域' }}</span
        >
        <h3>{{ current.title }}</h3>
        <section v-for="task in current.tasks" :key="task.title" class="task">
          <h4>{{ task.title }}</h4>
          <ol class="instructions"><li v-for="instruction in task.instructions" :key="instruction">{{ instruction }}</li></ol>
          <figure v-if="task.image">
            <img :src="task.image" :alt="task.caption" loading="lazy" />
            <figcaption>{{ task.caption }}（示範資料；介面配色可能不同）</figcaption>
          </figure>
        </section>
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
  </section>
</template>

<style scoped>
.guide {
  padding: 12px 16px;
  margin-bottom: var(--space);
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
h4 { margin: 16px 0 8px; font-size: 14px; }
.instructions { padding-left: 26px; margin: 0; font-size: 14px; line-height: 1.8; }
.instructions li { padding: 4px 0 4px 4px; }
.instructions li::marker { color: var(--accent-soft); font-weight: 700; }
figure { margin: 12px 0 20px; }
figure img { display: block; width: 100%; height: auto; border: 1px solid var(--border); border-radius: 8px; }
figcaption { margin-top: 6px; font-size: 12px; color: var(--text-dim); line-height: 1.6; }
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
  color: var(--accent-ink);
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
  .steps { max-height: 140px; overflow-y: auto; }
}
.guide.collapsed {
  padding: 8px 12px;
  background: transparent;
  border: 0;
}
.guide.collapsed h2 {
  font-size: 12px;
  color: var(--text-dim);
}
</style>
