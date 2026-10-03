# MangaFolio · 漫畫閱讀器

一款跨平台、高效能、忠於傳統閱讀體驗的本地漫畫／圖集閱讀器。對標 MangaMeeyaCE。

技術棧：**Tauri 2 + Rust 後端 + Vue 3 (TypeScript) 前端**。

文件入口：[文件索引](docs/README.md) · [新功能教學](docs/library-guide.md) · [開發交接](docs/handoff.md) · [驗證紀錄](docs/validation.md) · [工作日誌](docs/work-log.md)。

本地獨立審查請使用 [審查作業交辦單](docs/review-assignment.md)，內含可直接交給 Claude／Codex 的指令。

---

## 下載

至 [Releases](https://github.com/Racious/MangaFolio/releases/latest) 取得最新版本：

| 類型 | 說明 |
| --- | --- |
| 安裝版（MSI，推薦） | Windows Installer，自動整合捷徑與解除安裝 |
| 安裝版（NSIS） | 輕量安裝程式 |
| 攜帶版（免安裝） | 單一 `.exe`，無需安裝，直接執行 |

---

## 開發進度

| 階段 | 內容 | 狀態 |
| --- | --- | --- |
| **P1 — 基礎** | 開啟資料夾／ZIP／CBZ、自然排序、單頁顯示、左右翻頁 | ✅ 完成 |
| **P2 — 閱讀核心** | 雙頁、左／右開、縮放模式、Lanczos3 縮放管線 | ✅ 完成 |
| P3 — 體驗 | 快捷鍵、預載快取、書庫、收藏、搜尋、續讀、教學引導 | 功能分支已實作；縮圖列待開發，尚待獨立審查與 Windows 驗收 |
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

## 書庫開發版

此分支新增本機書庫首頁、封面、書名搜尋、整本收藏、最近閱讀與自動續讀。

首頁新增可收合的五步教學，標示對應操作區域，並可隨時重看。完整操作與畫面對照請見 [新功能教學](docs/library-guide.md)。

- **加入書籍**：首頁可多選 ZIP／CBZ，或加入直接包含圖片的資料夾；原始檔案留在原位。重複加入同一來源會更新資料，不會清除收藏與進度。
- **閱讀與收藏**：點封面開始閱讀，點星號收藏；「我的收藏」「最近閱讀」可搭配書名搜尋。搜尋支援大小寫與全形／半形正規化。
- **自動續讀**：每本書保存頁面名稱、頁碼、方向、單雙頁、縮放及翻頁偏好。返回書庫、切書與正常關閉視窗前會補存；儲存失敗時顯示錯誤並保留閱讀視窗供重試。
- **本機資料**：Rust 使用 bundled SQLite，不需另外安裝資料庫。`library.sqlite3` 與 `covers/` 位於 Tauri 的應用資料目錄（identifier 為 `com.racious.mangafolio`）。備份前先關閉程式；程式開啟時 SQLite 可能還有 WAL 資料。
- **管理與備份**：勾選書籍可批次收藏／移除，單本可重新指定來源。支援 JSON 書庫備份與合併還原，既有來源保留不覆寫；備份不含漫畫本體。程式內匯出會先補存進度，不需要關閉程式；手動複製 SQLite 的備份仍需先關閉。
- **目前範圍**：這是手動加入來源的本機書庫，尚未提供遞迴掃描漫畫根目錄、頁面書籤、OCR 或雲端同步。來源暫時不存在時會保留書籍資料、收藏與進度；重新接回原路徑或重新指定來源後可恢復使用。

## 開發

```bash
npm ci
npm test              # 書名搜尋、篩選與排序
npm run build         # TypeScript 與前端建置
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run tauri dev      # 開發模式（前端 + Rust 熱重載）
npm run tauri build    # 打包正式版
```

### 發版（自動 release）

1. 在 `CHANGELOG.md` 頂端新增 `## vX.Y.Z` 區段並寫好更新說明。
2. 同步版本號：`npm run version:set -- X.Y.Z`（更新 package.json / tauri.conf.json / Cargo.toml）。
3. 提交後打 tag 並推送：

   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

4. GitHub Actions（`.github/workflows/release.yml`）自動建置並發佈 Release，附 MSI／NSIS 安裝檔、攜帶版 `.exe`，說明內容取自 `CHANGELOG.md`。

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
│  ├─ LibraryView.vue     # 書庫、搜尋、收藏與續讀入口
│  ├─ LibraryGuide.vue    # 五步教學與操作區域提示
│  ├─ LibraryManager.vue  # 批次管理、重新指定來源、備份／還原
│  ├─ BookCard.vue        # 延遲封面、收藏與進度卡片
│  ├─ Toolbar.vue         # 工具列（開檔、縮放、翻頁特效、單雙頁、方向、封面單獨）
│  └─ PageScrubber.vue    # 底部頁碼進度條（可跳頁、顯示檔名）
├─ stores/reader.ts       # Pinia 狀態（書、頁碼、方向、縮放、特效、配對）
├─ stores/library.ts      # 書庫狀態、加入與收藏
├─ api/backend.ts         # 封裝 Tauri invoke（含影像解碼）
└─ App.vue                # 組裝

src-tauri/                # Rust 後端
└─ src/
   ├─ lib.rs              # 應用進入點、指令註冊
   ├─ library.rs          # SQLite 書庫、收藏、閱讀進度與封面
   ├─ commands.rs         # IPC 指令（open_path / render_page）＋兩級快取＋預載
   ├─ image_pipeline.rs   # 解碼 → Lanczos3(SIMD) 縮放 → PNG 編碼
   ├─ cache.rs            # 解碼／算繪 LRU 快取
   ├─ book.rs             # 書籍抽象（資料夾 / 壓縮檔）
   ├─ archive/zip.rs      # ZIP / CBZ 讀取
   └─ sorting.rs          # 自然排序
```

---

*由天城（AMAGI）整理*
