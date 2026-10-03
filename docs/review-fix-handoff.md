# A／R1／R2 修正與 R1 補強審查交辦

日期：2026-10-03（Asia/Tokyo）。分支：`feature/library-favorites-resume`。

## 固定版本與交付

- 本輪固定審查基準：`197668666a08507a8af796b50befe61b99b8c33e`。
- 產品修正提交：`878d2bb6918fe1adbf1b0d394dbec6f6f478bdd3`。其後文件提交僅填入本 SHA，不含產品程式差異；審查產品範圍以基準到本提交為準。
- A／R1／R2 前輪修正為原補丁無法取得後重新實作；本輪再補強 R1，不宣稱原補丁曾套用。
- 原工作區 `/workspace/MangaFolio` 保留在 `work`；使用既有乾淨隔離 worktree。遠端起始版本已 fetch 核對等於上述基準。

## 修改檔案及行為

| 檔案 | 問題與修正 |
| --- | --- |
| `src-tauri/src/commands.rs` | `open_library_book(id)` 將選取 ID 傳入 `open_source`／`register_selected`。註冊失敗即返回，尚未 mark_opened 或替換 BookSlot、session、generation、快取。新增開书衝突與唯一／離線還原整合測試。 |
| `src-tauri/src/library.rs` | 掃描所有來源匹配，移除精確字串優先／第一筆優先的選擇。多筆匹配明確回傳「來源衝突」，不刪除／合併資料。選取 ID 在交易內重新確認存在及來源一致；更新與讀回結果同一 IMMEDIATE 交易，失敗回滾。一般新增／重新加入亦拒絕多筆匹配。 |
| `src-tauri/src/library.rs` | Windows 內部鍵僅剝除磁碟絕對路徑的 `\\?\`，以及具有 server、share 的 `\\?\UNC\`；其他 device namespace 保持原樣且不 canonicalize 為磁碟鍵。儲存路徑不遷移。 |
| `tests/library.test.ts` | 全形 `ｂ` 經 NFKC 搜尋只比對標題，不因來源 `.cbz` 副檔名誤命中。 |
| `docs/review-fix-results/r1-followup/` | 本輪新驗證完整輸出與退出碼，JSON 的 `output` 欄保留原始完整 stdout／stderr（含換行）。不是前輪舊紀錄。 |

## 需求對照及回歸證據

| 需求 | 覆蓋 |
| --- | --- |
| A：ZIP／CBZ file_stem；資料夾／單圖父資料夾完整名稱 | `archive_titles_reimport_preserves_state_and_folder_dots`，兩種壓縮格式；直接模擬舊標題含副檔名後重新加入，檢查新標題及 ID、收藏、進度、頁名、最近閱讀時間、偏好。全形 `ｂ` 前端搜尋測試。A 產品程式本輪未變。 |
| R1：舊資料庫兩筆來源別名、選任一本不得換 ID | `duplicate_sources_reject_both_selected_ids_and_import_without_switching_reader`，在臨時 SQLite 注入兩筆不同 ID／收藏／進度／偏好／時間；選兩筆及一般重新加入均回傳衝突。前後完整備份 bytes 與 sqlite_sequence 相同；BookSlot Arc、session、generation、next_session 不變。 |
| R1：選取來源驗證／交易失敗 | `selected_source_mismatch_and_sql_failure_leave_data_unchanged`：不符來源、已不存在 ID、SQLite trigger 注入 UPDATE 失敗；備份前後不變。 |
| R1：唯一別名／離線還原上線後開書 | `unique_alias_and_offline_restore_open_selected_id_with_reading_state` 真實 open_source 流程，ID、收藏、頁碼／頁名、偏好、儲存路徑不變；重新加入最近閱讀時間保留。既有 restore/reopen 資料層測試改走 selected ID 註冊。 |
| R1：Windows 鍵轉換範圍 | `windows_keys_only_convert_absolute_disks_and_complete_unc` 含磁碟、UNC、Volume、GLOBALROOT、device、相對磁碟、不完整 UNC；既有離線 Windows 別名合併／備份重複拒絕測試。 |
| R2：成功後同步仍載入 reader，失敗／切書不污染 | 既有 `tests/favorite-store.test.mjs` 三項真實 Pinia store／mock IPC 測試本輪全部重跑；產品程式未變。 |

註冊／重新加入保留最近閱讀時間；成功「開書」仍依既有 `mark_opened` 更新最近閱讀時間，未新增行為。衝突在 mark_opened 前拒絕。沒有調整 AUTOINCREMENT／既有同字串 INSERT 衝突機制。

## 本輪驗證

Linux，沿用 `.mangafolio-tools/env.sh` 工具鏈與既有 node_modules／Cargo target 編譯快取；未修改 lockfile。測試全部只使用臨時來源及資料庫，未啟動正式應用。

| 命令 | 本輪結果 | 完整輸出 |
| --- | --- | --- |
| `npm test` | exit 0；7 passed，0 failed，含 A 搜尋與 R2 | `docs/review-fix-results/r1-followup/npm-test.json` |
| `npm run build` | exit 0；TypeScript／Vite 通過 | `docs/review-fix-results/r1-followup/npm-build.json` |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | exit 0；26 passed，0 failed；main／doc 零測試 | `docs/review-fix-results/r1-followup/cargo-test.json` |
| `git diff --check` | exit 0 | `docs/review-fix-results/r1-followup/diff-check.json` |

退出碼索引：`docs/review-fix-results/r1-followup/exit-codes.json`。本輪編譯中的首輪輸出另存 `cargo-test-initial.json`；最終判定使用最新 `cargo-test.json`。

## 未執行、實機複驗與限制

- Linux **未執行** `#[cfg(windows)] windows_normal_restore_then_verbatim_open_keeps_id`；一般／verbatim／UNC 真實 Windows IO、大小寫／連結別名、斷線分享需 Windows 隔離書庫複驗。
- **未執行原生介面人工驗收**：兩筆舊重複資料開書錯誤且保留當前閱讀、唯一／離線恢復後續讀、原生 ZIP／CBZ 重新加入、返回書庫收藏再返回閱讀、失敗／等待期間切書。
- 不全面改寫路徑、不自動修復舊重複資料；多筆來源衝突須使用者另行確認處理。離線且無法解析的不同連結目標不能僅靠字串確認，重新上線後解析；檔案系統來源在檢查後被外部替換的競態仍是既有本機 IO 限制。
- 未驗 Windows 打包、安裝、簽章、自動更新、正式發版或大型網路書庫效能。來源比對需解析既有路徑，網路來源延遲仍待實機評估。
- 未碰 `%APPDATA%\com.racious.mangafolio\library.sqlite3`，不 force push、不合併 main、不發布版本。

## 給本地 Claude／Codex 審查者

請同時閱讀 `docs/review-report.md`，審查基準到產品修正 SHA 的實際程式差異與測試，不只閱讀交辦單；核對 A、R1、R2 是否完整修復，特別檢查所有來源匹配、選取 ID 保留、交易失敗與收藏同步。

將結果寫入 `docs/review-fix-report.md`。發現問題請提供位置、觸發條件、影響及建議修法；列出實際驗證與未執行項目。未經另行授權，不自行修改產品程式、提交或推送。此文件是審查交辦，本輪尚未宣稱獨立審查通過。
