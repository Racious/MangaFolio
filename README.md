# MangaFolio · 漫畫閱讀器

一款跨平台、高效能、忠於傳統閱讀體驗的本地漫畫／圖集閱讀器。對標 MangaMeeyaCE。

技術棧：**Tauri 2 + Rust 後端 + Vue 3 (TypeScript) 前端**。

---

## 開發進度

| 階段 | 內容 | 狀態 |
| --- | --- | --- |
| **P1 — 基礎** | 開啟資料夾／ZIP／CBZ、自然排序、單頁顯示、左右翻頁 | ✅ 完成 |
| **P2 — 閱讀核心** | 雙頁、左／右開、縮放模式、Lanczos3 縮放管線 | ✅ 完成 |
| P3 — 體驗 | 快捷鍵、預載快取、縮圖列、進度記憶 | 規劃中 |
| P4 — 格式擴增 | RAR / CBR、7z | 規劃中 |

### P1 已實作功能

- 開啟**資料夾**、**ZIP / CBZ** 壓縮檔、或**單張圖片**（單張圖片會載入其所在資料夾並定位該圖）
- 頁面清單**自然排序**（1, 2, …, 10，而非 1, 10, 2）
- **單頁顯示**，圖片配合視窗（contain）
- **左右翻頁**：方向鍵、空白鍵、PageUp／Down、Home／End、點擊左右半邊、工具列按鈕
- **左開／右開**切換（影響翻頁與點擊方向；右開為日漫預設）

### P2 已實作功能

- **後端影像管線**：解碼 → Lanczos3 縮放 → PNG 編碼，前端以 1:1 原生像素呈現（不二次縮放）
- **五種縮放模式**：配合視窗／配合寬度／配合高度／原始尺寸／固定倍率（10%–800%）
- **雙頁顯示**與跨頁配對（依左右開決定左右位置；偶數頁對齊步進）
- 容器尺寸變動（縮放視窗）即時重算並重新請求後端縮放
- 快速翻頁以 render token 防止舊請求覆蓋新畫面

> 註：雙頁採「每頁配合半個視窗」策略，對尺寸一致的漫畫頁（常態）效果理想。
> 傳輸格式目前統一 PNG（線稿無損銳利）；照片內容的 WebP/JPEG 啟發式留待後續優化。

---

## 開發

```bash
npm install
npm run tauri dev      # 開發模式（前端 + Rust 熱重載）
npm run tauri build    # 打包正式版
```

### 環境需求

| 工具 | 版本 |
| --- | --- |
| Rust toolchain | 1.95.0 |
| Node.js | v22.x |
| MS C++ Build Tools | 建置 Rust 時需要 |

---

## 架構

前後端透過 Tauri IPC 通訊。**核心原則：原圖絕不直送 WebView** —— 後端負責解碼與縮放（P2 起），前端只負責排版顯示。

```
src/                      # Vue 3 前端
├─ components/
│  ├─ ReaderView.vue      # 閱讀區（單／雙頁、3D 翻書特效、點擊／鍵盤導航）
│  ├─ Toolbar.vue         # 工具列（開檔、縮放、翻頁特效、單雙頁、方向、封面單獨）
│  └─ PageScrubber.vue    # 底部頁碼進度條（可跳頁、顯示檔名）
├─ stores/reader.ts       # Pinia 狀態（書、頁碼、方向、縮放、特效、配對）
├─ api/backend.ts         # 封裝 Tauri invoke（含影像解碼）
└─ App.vue                # 組裝

src-tauri/                # Rust 後端
└─ src/
   ├─ lib.rs              # 應用進入點、指令註冊
   ├─ commands.rs         # IPC 指令（open_path / render_page）＋兩級快取＋預載
   ├─ image_pipeline.rs   # 解碼 → Lanczos3(SIMD) 縮放 → PNG 編碼
   ├─ cache.rs            # 解碼／算繪 LRU 快取
   ├─ book.rs             # 書籍抽象（資料夾 / 壓縮檔）
   ├─ archive/zip.rs      # ZIP / CBZ 讀取
   └─ sorting.rs          # 自然排序
```

---

*由天城（AMAGI）整理*
