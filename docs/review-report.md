---
title: MangaFolio 書庫、續讀與管理備份獨立審查
type: review
date: 2026-10-03
reviewer: 天城／Codex 桌面互動審查（GPT-6）
---

# 審查結果

結論：確認 **2 項 P2（中）功能缺陷**，建議修正並重驗後再合併。本次未確認 P0／P1 問題；測試通過不代表可正式發版。未修改產品程式、設定或既有未提交成果，未 commit、push、合併或操作正式書庫。

## 實際版本、範圍與環境

| 項目 | 實際值 |
| --- | --- |
| 交辦單 | `docs/review-assignment.md`（本地 HEAD 文件） |
| 日期 | 2026-10-03，Asia/Tokyo |
| 分支 | `feature/library-favorites-resume` |
| 本地 HEAD | `63b86c8b19734e860d66dddc42466ccee8902bee` |
| 固定 base | `d35af0027dd0b08eac84f520ceed797c1c3f2b40` |
| 固定產品 head | `df6c1349ffad0aa7f08a4abf64ec07a85352a085` |
| 比較語意 | 固定兩提交的 tree diff；34 檔、3490 insertions、84 deletions |
| HEAD 與產品 head 差異 | 9 檔文件／截圖，沒有後續產品程式變更 |
| 本地未提交變更 | `src-tauri/Cargo.toml`；未追蹤 `AGENTS.md`、`CLAUDE.md`、`CLAUDE-CODE-FEEDBACK.md`、Android／iOS icons；均排除且保留 |
| OS | Windows x64，kernel release `10.0.26200`（Node `os.release()`） |
| Node／npm | `v22.22.3`／`10.9.8` |
| Rust／Cargo | `1.95.0 (59807616e 2026-04-14)`／`1.95.0 (f2d3ce0bd 2026-03-21)` |
| 隔離方式 | `git archive df6c134` 匯出獨立暫存副本；資料層測試使用臨時圖片與資料庫 |

Git 遇到 sandbox 身分的 dubious ownership，採單次 `git -c safe.directory=E:/projects/agents/MangaFolio ...` 讀取，沒有修改 global／local Git 設定。未切分支、拉取或 stash；指定提交均已存在，故直接審查固定版本。Vault 活頁仍為 2026-07-05 的既有狀態，本輪以交辦單、`docs/handoff.md`、`docs/validation.md`、`docs/library-guide.md` 及固定程式版本確認需求，未同步或修改 Vault。

## 必要修正

### R1 — P2／中：還原路徑未正規化，開書會另建紀錄並失去還原狀態

- **位置**：`src-tauri/src/library.rs:462`、`:476`；相關開書路徑 `src-tauri/src/commands.rs:115–120`、註冊 `src-tauri/src/library.rs:164–175`。
- **觸發條件**：合法 v1 JSON 備份含可存取、但未 canonical 化的絕對來源路徑。Windows 上一般 `C:\...\pages` 與註冊產生的 `\\?\C:\...\pages` 指向同一來源；外部產生或調整路徑的備份可合法通過目前驗證。這不是宣稱應用原樣匯出的普通 roundtrip 失敗。
- **預期**：還原後點開該書仍使用還原紀錄，保留收藏／設定／進度；同來源略過，整批重複來源拒絕。
- **實際**：`restore_json` 只比較原始字串，直接寫入 path；`open_library_book` 隨後呼叫 `register`，canonical 化後以另一字串插入新書。開啟結果使用新 ID 與預設狀態，原還原紀錄仍留在書庫。同來源的別名字串也能避開合併略過與備份內重複檢查。
- **證據**：在 Windows 暫存副本以真實 PNG 資料夾產生備份，保留 `favorite=true`，將 path 換成同一來源的一般絕對路徑，移除原測試項目後還原，再依開書所用的 `book::open → Library::register` 路徑執行：

  ```text
  CONFIRMED: restored id=2 favorite=true -> opened id=3 favorite=false; rows=2
  test library::independent_review_tests::review_restore_alias_then_open_loses_restored_identity ... ok
  ```

  重現測試的 `ok` 表示斷言成功確認缺陷，並非產品行為正確。原本 18 項測試只涵蓋字串相同的 roundtrip／duplicates，未覆盖此分支。
- **建議**：還原驗證及合併使用與 register／relink 一致的來源識別；可用來源先 canonical 化，備份內與既有書庫均以正規化鍵去重。保留離線路徑的同時，定義重新上線時的身份銜接，避免開書悄悄換 ID。補一般／verbatim Windows 路徑、來源別名、離線重新上線案例，檢查收藏、設定與進度。

### R2 — P2／中：單本卡片收藏與仍載入的閱讀器不同步

- **位置**：`src/stores/library.ts:78–81`；相關 `src/stores/reader.ts:232–240`、`src/components/LibraryView.vue:121–125`。
- **觸發條件**：開啟未收藏的書 → 返回書庫（reader 保留該書）→ 點卡片收藏 → 點「返回閱讀」。反方向取消收藏也同樣受影響。
- **預期**：閱讀工具列立刻反映最新收藏；下一次點擊取消收藏。
- **實際**：library store 只更新卡片資料，未更新相同 bookId 的 `reader.favorite`。返回閱讀不重新載入書；工具列顯示舊值，下一次切換依舊值送出相同的收藏指令，第一次點擊無法達成預期操作。
- **證據**：在暫存副本以真實 Pinia 與 TypeScript 轉譯的兩份受審 store 執行，僅 mock 收藏 IPC；保留 reader.bookId=1，先呼叫卡片 action，再呼叫 reader action：

  ```text
  After card favorite: {"card":true,"reader":false,"persisted":true}
  After attempting reader unfavorite: {"reader":true,"persisted":true,"calls":[{"id":1,"value":true},{"id":1,"value":true}]}
  ```

  批次收藏在 `LibraryManager.vue` 已同步 reader.favorite，但單本卡片 action 沒有這項防護；沒有其他 watcher 替它同步。
- **建議**：單本收藏成功後同步目前相同 bookId 的 reader，或改成共用收藏狀態。補「保留 reader 返回書庫後單本收藏／取消，再返回閱讀並切換」的 store／介面回歸驗證。

## 驗證命令與結果

以下產品測試均針對固定 `df6c134` 副本，不納入原工作區修改。

| 命令／檢查 | 結果 | 證據／限制 |
| --- | --- | --- |
| `git diff --stat d35af002 df6c134`、`git diff df6c134 HEAD --stat` | 通過 | 固定範圍可讀；後續提交僅文件 |
| `git diff --check d35af002 df6c134` | 通過 | 無 whitespace error，exit 0 |
| `npm ci --ignore-scripts --no-audit --no-fund --cache ../npm-cache` | 通過 | 暫存副本成功 added 83 packages，exit 0；忽略非必要安裝 lifecycle scripts |
| `npm test` | 通過 | 3 passed、0 failed |
| `node node_modules/vue-tsc/bin/vue-tsc.js --noEmit` | 通過 | exit 0 |
| `npm run build`（lockfile 安裝後） | 通過 | TypeScript＋Vite 6.4.3，62 modules，exit 0 |
| `cargo test --locked --offline --manifest-path src-tauri/Cargo.toml` | 通過 | 原始版本 library tests 18 passed、0 failed；main／doc 零測試不計案例 |
| `node repro-favorite.cjs` | 缺陷重現成功 | 真實 store、mock IPC，確認 R2；不等同原生 UI 驗收 |
| `cargo test --locked --offline --manifest-path src-tauri/Cargo.toml independent_review_tests -- --nocapture` | 缺陷重現成功 | 僅暫存副本新增重現測試，1 passed，18 filtered out；確認 R1 |

環境失敗與最終結果分開記錄：初次 npm 安裝受原快取 `EPERM` 阻擋；改獨立快取仍有 registry `EACCES`，npm 回報 `Exit handler never called`。其後經工具核准在沙箱外、仍於同一暫存副本安裝，成功。Vite 初次受父目錄讀取 `Access is denied` 阻擋；同樣於核准後在副本建置成功。曾先複製既有 node_modules 驗證；最終已以 lockfile 重新安裝並重跑 npm test／build，不以先前副本建置代替最終證據。

## 檢查範圍與完成度稽核

- 書庫資料層：schema 版本拒絕、WAL、進度範圍與偏好驗證、交易回滾、移除來源保留、封面鎖順序、relink、備份大小／數量／版本限制、create_new 防覆寫、合併還原與來源去重。
- 閱讀流程：400ms 保存、序列化 flush、切書／返回／關閉、管理先 flush 後 discard、session-aware decode／render key、舊 session 拒絕。
- 前端：單本與批次收藏、選取目前顯示、篩選／排序清選取、忙碌互斥、取消與錯誤路徑、API 型別、封面排隊與 blob 清理、鍵盤／IME 防護、教學 localStorage 容錯與窄視窗樣式。
- 桌面安全靜態檢查：自訂 IPC 的本機讀寫邊界、export 不覆寫、restore 限量讀取、主視窗 destroy 權限；新增介面未使用 v-html／innerHTML 執行備份文字。CSP=null 為既有配置，未確認可達注入鏈，不列成已證實漏洞。本輪不是完整安全掃描。
- 功能實作入口及既有 3＋18 測試數量與交辦宣稱一致；上述 R1／R2 表明來源去重與收藏同步仍未完全達成。不能用全綠取代需求驗收。

## 未執行與剩餘風險

- **未執行 `npm run tauri dev` 與原生桌面人工清單**：本輪工具沒有可用的 native UI 控制面；直接啟動正式 identifier 的應用可能使用老爺現有書庫，未進行這種操作。未將 Rust 資料層結果計作原生選檔成功。
- **未重跑 Python Playwright management smoke**：該項為選配 mock UI 驗證；本輪以目標 store 重現與 Rust 真實 IO 驗證確認發現，沒有 browser smoke 通過宣稱。
- 原生 ZIP／CBZ＋圖片資料夾的加入／取消、立即切書／關閉／重啟、管理中關閉、檔案視窗、JSON 匯出還原／relink 全流程、640×480／中文 IME／鍵盤操作仍須在隔離原生書庫驗收。
- Windows 大小寫、連結別名、離線重新上線、唯讀目的地尚未完成完整案例矩陣；R1 已重現一般／verbatim 路徑差異，其餘不當作通過。
- Windows 打包、安裝升級、簽章、自動更新、效能與大型書庫驗收未執行，仍屬正式發版前工作。

## 接手

主導方應逐項評估 R1／R2，修正後補聚焦回歸證據；有異議或實質方案取捨時由老爺裁決。交辦單未指定 resolution 檔案，審查方不代填採納／裁決，也不宣稱修復或合併通過。

本機重現材料留於 `C:/Users/Racious/AppData/Local/Temp/mangafolio-review-df6c134-20261003/`：`source.zip` 為原始固定版本，`source/` 為隔離驗證副本，`repro-favorite.cjs` 為 R2 重現，R1 測試只追加於副本 `source/src-tauri/src/library.rs`。這是本輪本機證據位置，並非跨機相依設定。
