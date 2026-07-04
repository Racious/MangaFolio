<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useReaderStore } from "../stores/reader";

const reader = useReaderStore();
const wrap = ref<HTMLElement | null>(null);
const scroller = ref<HTMLElement | null>(null);
let ro: ResizeObserver | null = null;

function measure() {
  if (wrap.value) {
    reader.setViewport(wrap.value.clientWidth, wrap.value.clientHeight);
  }
}

// ── 一般換頁 <Transition>（推移／淡入／無；翻書改走 3D 覆蓋層）──
// ── 3D 翻書 ──
interface FlipState {
  single: boolean;
  side: "left" | "right"; // 雙頁：翻動的是哪一頁
  spin?: string; // 單頁：旋轉類別（書脊固定在裝訂側）
  front: string;
  back: string;
  reveal: string;
  staticUrl?: string; // 雙頁：不動的另一頁
  urls: string[];
}
const flip = ref<FlipState | null>(null);
const flipTurned = ref(false);
let pendingTarget = 0;

// 僅「雙頁＋翻書」有翻頁動畫（3D 覆蓋層）；單頁與「無」皆即時換頁。
const useBookFlip = computed(
  () => reader.transition === "book" && reader.pageMode === "double" && reader.hasBook
);

async function turn(forward: boolean) {
  if (!reader.hasBook || flip.value) return;
  // 僅「雙頁＋翻書」有翻頁動畫；單頁與「無」即時換頁。
  // 捲軸歸零交由 watch(reader.index, flush:'post') 在新頁換好後、繪製前統一處理。
  if (!useBookFlip.value) {
    forward ? reader.next() : reader.prev();
    return;
  }

  const target = forward ? reader.nextStart() : reader.prevStart();
  if (target < 0) {
    // 邊界迴圈：以即時換頁繞回另一端（不跨端做翻書動畫，避免突兀）。
    forward ? reader.next() : reader.prev();
    return;
  }

  const curView = reader.viewIndices;
  const nextView = reader.indicesForStart(target);
  // 僅完整雙頁跨頁才翻書動畫；封面單頁／末尾單頁等退化為即時換頁。
  if (curView.length !== 2 || nextView.length !== 2) {
    forward ? reader.next() : reader.prev();
    return;
  }

  // 右開：下一頁往右翻（左頁繞書脊往右）、上一頁往左翻（右頁往左）。左開相反。
  const rtl = reader.direction === "rtl";
  const side: "left" | "right" = forward === rtl ? "left" : "right";

  let urls: string[];
  try {
    urls = await reader.peekUrls(nextView);
  } catch {
    forward ? reader.next() : reader.prev();
    return;
  }

  const curLeft = reader.slots[0]?.url;
  const curRight = reader.slots[1]?.url ?? curLeft;
  if (!curLeft || !curRight) {
    forward ? reader.next() : reader.prev();
    return;
  }
  // 目標跨頁的顯示左／右頁（依閱讀方向，與 store 排版一致）。
  const targetRight = rtl ? urls[0] : urls[1] ?? urls[0];
  const targetLeft = rtl ? urls[1] ?? urls[0] : urls[0];

  flip.value =
    side === "left"
      ? // 左頁往右翻：正面=當前左頁，背面=目標右頁，露出目標左頁，右側不動
        { single: false, side: "left", front: curLeft, back: targetRight, reveal: targetLeft, staticUrl: curRight, urls }
      : // 右頁往左翻：正面=當前右頁，背面=目標左頁，露出目標右頁，左側不動
        { single: false, side: "right", front: curRight, back: targetLeft, reveal: targetRight, staticUrl: curLeft, urls };

  flipTurned.value = false;
  pendingTarget = target;
  await nextTick();
  requestAnimationFrame(() => requestAnimationFrame(() => (flipTurned.value = true)));
}

function onFlipEnd(e: TransitionEvent) {
  if (e.propertyName !== "transform" || !flip.value) return;
  reader.commitWith(pendingTarget, reader.indicesForStart(pendingTarget), flip.value.urls);
  flip.value = null;
  flipTurned.value = false;
}

function onClick(e: MouseEvent) {
  const el = scroller.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const leftHalf = e.clientX - rect.left < rect.width / 2;
  // 右開：右半=下一頁；左開：左半=下一頁。
  const advance = reader.direction === "rtl" ? !leftHalf : leftHalf;
  turn(advance);
}

function onKey(e: KeyboardEvent) {
  if (!reader.hasBook) return;
  const el = scroller.value;
  // 單次捲動約視窗高 0.7（幅度大但靠 smooth 平滑滑行，非瞬間跳動）。
  const stepPx = el ? Math.max(160, el.clientHeight * 0.7) : 160;
  switch (e.key) {
    case "ArrowLeft":
      // 右開：左=上一頁；左開：左=下一頁。
      reader.direction === "rtl" ? turn(false) : turn(true);
      break;
    case "ArrowRight":
      // 右開：右=下一頁；左開：右=上一頁。
      reader.direction === "rtl" ? turn(true) : turn(false);
      break;
    case "ArrowUp":
      el?.scrollBy({ top: -stepPx, behavior: "smooth" });
      break;
    case "ArrowDown":
      el?.scrollBy({ top: stepPx, behavior: "smooth" });
      break;
    case "PageUp":
      turn(false);
      break;
    case "PageDown":
    case " ":
      turn(true);
      break;
    case "Home":
      reader.goto(0);
      break;
    case "End":
      reader.goto(reader.pageCount - 1);
      break;
    default:
      return;
  }
  e.preventDefault();
}

// 捲軸歸零：監聽 viewSeq（新頁 slots 換好後才遞增），而非 index（頁碼一改即觸發、但新圖尚未抓回）。
// flush:'post' → 在新頁 DOM 更新後、瀏覽器繪製前歸零捲軸；配合 viewSeq，舊頁在抓圖期間維持不動，
// 新圖就位後於同一幀歸零並繪出，消除「舊高圖先急速上滑再換頁」的兩段式抖動。
// 惟 viewSeq 亦會因縮放（setZoom）或視窗尺寸變動（ResizeObserver→setViewport→render）而遞增，
// 故以 lastScrollIndex 比對頁碼：僅「真正換頁」（index 改變）才歸頂；resize／縮放重繪一律保留當前閱讀位置。
let lastScrollIndex = -1;
watch(
  () => reader.viewSeq,
  () => {
    if (reader.index === lastScrollIndex) return; // resize／縮放重繪：非換頁，保留位置
    lastScrollIndex = reader.index;
    if (scroller.value) scroller.value.scrollTop = 0;
  },
  { flush: "post" }
);

onMounted(() => {
  ro = new ResizeObserver(measure);
  if (wrap.value) ro.observe(wrap.value);
  measure();
  window.addEventListener("keydown", onKey);
});
onUnmounted(() => {
  ro?.disconnect();
  window.removeEventListener("keydown", onKey);
});
</script>

<template>
  <div class="reader-wrap" ref="wrap">
    <div v-if="!reader.hasBook" class="placeholder">
      <div class="placeholder-art">漫</div>
      <p class="placeholder-title">尚未開啟書籍</p>
      <p class="placeholder-hint">由上方工具列開啟資料夾或壓縮檔（ZIP / CBZ）</p>
    </div>

    <div v-else class="scroll" :class="{ fit: reader.zoom === 'window' }" ref="scroller" @click="onClick">
      <!-- 版面直接就位；翻書動畫由 3D 覆蓋層負責，其餘即時換頁 -->
      <div class="stage">
        <img v-for="slot in reader.slots" :key="slot.index" :src="slot.url" class="page" draggable="false" />
      </div>
    </div>

    <!-- 3D 翻書覆蓋層 -->
    <div v-if="flip" class="flip-overlay">
      <div class="flip-stage" :class="{ turned: flipTurned }">
        <!-- 單頁：朝裝訂側基準線收攏（弱透視，偏收縮非擺盪） -->
        <template v-if="flip.single">
          <div class="half flip-half single-half">
            <img class="sizer" :src="flip.front" draggable="false" />
            <img class="reveal" :src="flip.reveal" draggable="false" />
            <div class="sheet" :class="flip.spin" @transitionend="onFlipEnd">
              <img class="face front" :src="flip.front" draggable="false" />
              <img class="face back" :src="flip.back" draggable="false" />
            </div>
          </div>
        </template>

        <!-- 雙頁：左右兩半，翻動頁繞中央書脊翻轉 -->
        <template v-else>
          <div class="half" :class="{ 'flip-half': flip.side === 'left' }">
            <template v-if="flip.side === 'left'">
              <img class="sizer" :src="flip.front" draggable="false" />
              <img class="reveal" :src="flip.reveal" draggable="false" />
              <div class="sheet sheet-left" @transitionend="onFlipEnd">
                <img class="face front" :src="flip.front" draggable="false" />
                <img class="face back" :src="flip.back" draggable="false" />
              </div>
            </template>
            <img v-else class="static-page" :src="flip.staticUrl" draggable="false" />
          </div>
          <div class="half" :class="{ 'flip-half': flip.side === 'right' }">
            <template v-if="flip.side === 'right'">
              <img class="sizer" :src="flip.front" draggable="false" />
              <img class="reveal" :src="flip.reveal" draggable="false" />
              <div class="sheet sheet-right" @transitionend="onFlipEnd">
                <img class="face front" :src="flip.front" draggable="false" />
                <img class="face back" :src="flip.back" draggable="false" />
              </div>
            </template>
            <img v-else class="static-page" :src="flip.staticUrl" draggable="false" />
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.reader-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  background: var(--bg);
  overflow: hidden;
  user-select: none;
  display: flex;
  align-items: center;
  justify-content: center;
}
.scroll {
  position: absolute;
  inset: 0;
  overflow: auto;
  display: flex;
  cursor: pointer;
  /* 關閉捲動錨定：換頁時新圖高度不同，避免瀏覽器為「維持視覺位置」而跳動造成兩段式 */
  overflow-anchor: none;
}
/* 配合視窗模式內容必填滿、不需捲動：關閉捲軸，避免捲軸寬度造成置中位移與翻頁閃爍 */
.scroll.fit {
  overflow: hidden;
}
.stage {
  margin: auto;
  display: flex;
  align-items: center;
}
.page {
  display: block;
}

/* ── 3D 翻書覆蓋層 ── */
.flip-overlay {
  position: absolute;
  inset: 0;
  z-index: 20;
  background: var(--bg);
  display: flex;
}
.flip-stage {
  margin: auto;
  display: flex;
  align-items: center;
}
/* perspective 必須在翻動頁的「直接父層」，3D 與背面才正確 */
.flip-stage .half {
  position: relative;
  perspective: 2200px;
}
/* 單頁：弱透視，使翻頁偏「朝基準線等比收攏」而非門板擺盪 */
.flip-stage .half.single-half {
  perspective: 5200px;
}
/* 含翻動頁的那一半需疊在另一半之上，否則翻動頁會被另一半的靜止頁遮住
   （perspective 使每個 .half 成為獨立堆疊脈絡，故須顯式拉高 z-index）。 */
.flip-stage .half.flip-half {
  z-index: 5;
}
.static-page,
.sizer {
  display: block;
}
/* sizer = 已載入的當前頁，僅用來把該半邊撐成正確尺寸（避免新圖未載入時塌陷閃爍） */
.sizer {
  position: relative;
  z-index: 0;
}
/* reveal = 下一頁，絕對定位不影響版面；翻動開始前隱藏 */
.reveal {
  position: absolute;
  inset: 0;
  z-index: 1;
  visibility: hidden;
}
.flip-stage.turned .reveal {
  visibility: visible;
}
.sheet {
  position: absolute;
  inset: 0;
  z-index: 10;
  transform-style: preserve-3d;
  transition: transform 0.5s ease-in-out, filter 0.5s ease-in-out;
}
.sheet-right {
  transform-origin: left center;
}
.sheet-left {
  transform-origin: right center;
}
/* 單頁：書脊固定於右緣（sr）或左緣（sl）；翻動方向決定正負旋轉。 */
.sr-pos,
.sr-neg {
  transform-origin: right center;
}
.sl-pos,
.sl-neg {
  transform-origin: left center;
}
/* 注意：切勿在此加 filter/opacity——會使 preserve-3d 扁平化，背面隱藏失效。 */
.flip-stage.turned .sheet-right {
  transform: rotateY(-180deg);
}
.flip-stage.turned .sheet-left {
  transform: rotateY(180deg);
}
/* 單頁只翻到直角（90°）即露出下一頁，無需翻滿 180°。 */
.flip-stage.turned .sr-pos,
.flip-stage.turned .sl-pos {
  transform: rotateY(90deg);
}
.flip-stage.turned .sr-neg,
.flip-stage.turned .sl-neg {
  transform: rotateY(-90deg);
}
.face {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  display: block;
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
}
.face.back {
  transform: rotateY(180deg);
}

.placeholder {
  text-align: center;
  color: var(--text-dim);
}
.placeholder-art {
  font-size: 88px;
  color: var(--border);
  font-weight: 700;
  line-height: 1;
  margin-bottom: 18px;
}
.placeholder-title {
  font-size: 18px;
  color: var(--text);
  margin-bottom: 6px;
}
.placeholder-hint {
  font-size: 13px;
}
</style>
