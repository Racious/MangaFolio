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
const transName = computed(() => {
  if (reader.transition === "none" || reader.transition === "book") return "";
  if (reader.transition === "fade") return "pg-fade";
  const dirSign = (reader.direction === "rtl" ? -1 : 1) * reader.flow;
  return dirSign < 0 ? "pg-push-right" : "pg-push-left";
});
const transMode = computed<"out-in" | undefined>(() => {
  const t = transName.value;
  return t === "pg-push-right" || t === "pg-push-left" ? undefined : "out-in";
});

// ── 3D 翻書 ──
interface FlipState {
  single: boolean;
  side: "left" | "right"; // 翻動頁繞哪一側書脊（單／雙頁皆用）
  front: string;
  back: string;
  reveal: string;
  staticUrl?: string; // 雙頁：不動的另一頁
  urls: string[];
}
const flip = ref<FlipState | null>(null);
const flipTurned = ref(false);
let pendingTarget = 0;

const useBookFlip = computed(() => reader.transition === "book" && reader.hasBook);
// 翻書（動畫由 3D 覆蓋層負責）與「無」都直接就位、不經 <Transition>，避免閃爍。
const instantStage = computed(() => useBookFlip.value || reader.transition === "none");

async function turn(forward: boolean) {
  if (!reader.hasBook || flip.value) return;
  if (reader.transition !== "book") {
    forward ? reader.next() : reader.prev();
    return;
  }

  const target = forward ? reader.nextStart() : reader.prevStart();
  if (target < 0) return; // 已到邊界

  const curView = reader.viewIndices;
  const nextView = reader.indicesForStart(target);
  // 跨頁頁數不同（如封面單頁↔雙頁）時，翻書動畫不適用，退化為即時換頁。
  if (curView.length !== nextView.length) {
    forward ? reader.next() : reader.prev();
    return;
  }

  // 右開：下一頁往右翻（左頁繞書脊往右）、上一頁往左翻（右頁往左）。左開相反。
  // side='left' 表「左頁往右翻」；side='right' 表「右頁往左翻」。
  const rtl = reader.direction === "rtl";
  const flipsRight = forward === rtl;
  const side: "left" | "right" = flipsRight ? "left" : "right";

  let urls: string[];
  try {
    urls = await reader.peekUrls(nextView);
  } catch {
    forward ? reader.next() : reader.prev();
    return;
  }

  // 目標跨頁的顯示左／右頁（依閱讀方向，與 store 排版一致）。
  const targetRight = rtl ? urls[0] : urls[1] ?? urls[0];
  const targetLeft = rtl ? urls[1] ?? urls[0] : urls[0];

  if (curView.length === 2) {
    const curLeft = reader.slots[0]?.url;
    const curRight = reader.slots[1]?.url ?? curLeft;
    if (!curLeft || !curRight) {
      forward ? reader.next() : reader.prev();
      return;
    }
    flip.value =
      side === "left"
        ? // 左頁往右翻：正面=當前左頁，背面=目標右頁，露出目標左頁，右側不動
          { single: false, side: "left", front: curLeft, back: targetRight, reveal: targetLeft, staticUrl: curRight, urls }
        : // 右頁往左翻：正面=當前右頁，背面=目標左頁，露出目標右頁，左側不動
          { single: false, side: "right", front: curRight, back: targetLeft, reveal: targetRight, staticUrl: curLeft, urls };
  } else {
    const curPage = reader.slots[0]?.url;
    if (!curPage) {
      forward ? reader.next() : reader.prev();
      return;
    }
    flip.value = { single: true, side, front: curPage, back: urls[0], reveal: urls[0], urls };
  }

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
  const advance = reader.direction === "rtl" ? leftHalf : !leftHalf;
  turn(advance);
}

function onKey(e: KeyboardEvent) {
  if (!reader.hasBook) return;
  const el = scroller.value;
  const stepPx = el ? Math.max(80, el.clientHeight * 0.85) : 80;
  switch (e.key) {
    case "ArrowLeft":
      reader.direction === "rtl" ? turn(true) : turn(false);
      break;
    case "ArrowRight":
      reader.direction === "rtl" ? turn(false) : turn(true);
      break;
    case "ArrowUp":
      el?.scrollBy({ top: -stepPx });
      break;
    case "ArrowDown":
      el?.scrollBy({ top: stepPx });
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

watch(
  () => reader.index,
  () => {
    if (scroller.value) scroller.value.scrollTop = 0;
  }
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
      <!-- 翻書／無：版面直接就位（翻書動畫由 3D 覆蓋層負責），不經 <Transition> -->
      <div v-if="instantStage" class="stage">
        <img v-for="slot in reader.slots" :key="slot.index" :src="slot.url" class="page" draggable="false" />
      </div>
      <!-- 其餘特效 -->
      <Transition v-else :name="transName" :mode="transMode">
        <div class="stage" :key="reader.viewSeq">
          <img v-for="slot in reader.slots" :key="slot.index" :src="slot.url" class="page" draggable="false" />
        </div>
      </Transition>
    </div>

    <!-- 3D 翻書覆蓋層 -->
    <div v-if="flip" class="flip-overlay">
      <div class="flip-stage" :class="{ turned: flipTurned }">
        <!-- 單頁：繞中央軸翻轉 -->
        <template v-if="flip.single">
          <div class="half flip-half">
            <img class="reveal" :src="flip.reveal" draggable="false" />
            <div
              class="sheet"
              :class="flip.side === 'right' ? 'sheet-right' : 'sheet-left'"
              @transitionend="onFlipEnd"
            >
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

/* ── 整頁推移（推移特效）── */
.pg-push-right-leave-active,
.pg-push-left-leave-active {
  position: absolute;
  inset: 0;
  margin: auto;
  z-index: 3;
  transition: transform 0.2s ease-out;
}
.pg-push-right-leave-to {
  transform: translateX(100%);
}
.pg-push-left-leave-to {
  transform: translateX(-100%);
}
.pg-push-right-enter-active,
.pg-push-left-enter-active {
  transition: none;
}
.pg-fade-enter-active,
.pg-fade-leave-active {
  transition: opacity 0.15s ease;
}
.pg-fade-enter-from,
.pg-fade-leave-to {
  opacity: 0;
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
/* 注意：切勿在此加 filter/opacity——會使 preserve-3d 扁平化，背面隱藏失效。 */
.flip-stage.turned .sheet-right {
  transform: rotateY(-180deg);
}
.flip-stage.turned .sheet-left {
  transform: rotateY(180deg);
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
