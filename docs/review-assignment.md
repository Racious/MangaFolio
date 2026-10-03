# 本地 Claude／Codex 審查作業交辦單

交辦日期：2026-10-03（Asia/Tokyo）。Repository：`Racious/MangaFolio`。狀態：開發已提交，等待獨立審查；本文件不是審查通過證明。

## 任務與固定範圍

審查功能分支 `feature/library-favorites-resume` 的全部新功能：書庫、收藏、搜尋、最近閱讀、自動續讀、session 隔離、教學，以及第二輪的批次管理、重新指定來源、JSON 備份與合併還原。

| 範圍 | 固定提交 |
| --- | --- |
| 開發前基準 | `d35af0027dd0b08eac84f520ceed797c1c3f2b40` |
| 本次產品程式審查終點 | `df6c1349ffad0aa7f08a4abf64ec07a85352a085` |
| 只看本輪管理功能 | `cb6d79a` → `df6c134` |

最新文件提交會在上述程式終點之後。若遠端後來有新程式提交，先記錄實際 HEAD，再決定另開增量審查；不要把後來提交混入這份固定範圍。

先保存自己的未提交修改，再取得分支：

```bash
git fetch origin
git switch feature/library-favorites-resume
git pull --ff-only origin feature/library-favorites-resume
git status --short
git rev-parse HEAD
git diff --stat d35af0027dd0b08eac84f520ceed797c1c3f2b40 df6c1349ffad0aa7f08a4abf64ec07a85352a085
git diff d35af0027dd0b08eac84f520ceed797c1c3f2b40 df6c1349ffad0aa7f08a4abf64ec07a85352a085
```

沒有本地分支時，用 `git switch --track origin/feature/library-favorites-resume` 取代上面的 switch。

## 可直接貼給審查者的指令

```text
請依 docs/review-assignment.md 對 MangaFolio 做獨立程式碼審查。
先讀 docs/handoff.md、docs/validation.md、docs/library-guide.md。
固定審查範圍為 d35af0027dd0b08eac84f520ceed797c1c3f2b40 到
df6c1349ffad0aa7f08a4abf64ec07a85352a085，記錄本地實際 HEAD。
以資料正確性、回歸風險、Windows 相容性及桌面安全為重點。
先審查與重現，未收到修正指示前不改產品程式、不合併、不發版。
請執行可執行的驗證；無法執行的項目要寫原因，不能當作通過。
使用測試副本與測試書庫，勿拿使用者唯一漫畫或書庫做破壞性測試。
請將結果寫入 docs/review-report.md，列出實際提交、環境、命令與結果。
每項問題附嚴重度、檔案與行號、觸發條件、預期／實際結果、證據、建議。
若沒有發現問題，也要明列檢查範圍、未驗證項目與剩餘風險；
不能只因編譯或測試通過就宣告可合併或可正式發版。
```

## 建置與自動檢查

Node.js 22、Rust 1.95.0 與 Tauri 2 系統依賴。Windows 需 C++ Build Tools 與 WebView2。雲端的 `/workspace/.mangafolio-tools/` 不會隨 clone 帶到本地。

```bash
npm ci
npm test
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
git diff --check
npm run tauri dev
```

目前預期：3 個前端測試與 18 個 Rust library tests；main／doc-tests 的零測試不要另算案例。若失敗，保留輸出並判斷是環境或產品問題，不要先更新依賴、刪 lockfile 或關閉測試。

可另跑 mock IPC 的介面 smoke：先在另一個終端啟動 `npm run dev`，再依 [驗證紀錄](validation.md#管理介面-smoke-重跑方式) 安裝 Python Playwright 並執行 `scripts/smoke-library-management.py`。這只驗證前端操作與 mock 行為，不能取代原生檔案視窗或備份 IO 驗收。

## 程式碼審查重點

| 路徑 | 重點 |
| --- | --- |
| `src-tauri/src/library.rs` | SQLite 鎖順序、交易回滾、來源去重、schema 版本、進度驗證、重新指定來源、封面快取與 JSON 匯出還原。 |
| `src-tauri/src/commands.rs`、`cache.rs` | session ID 與快取鍵是否一致，舊算繪／預載是否可能混入新書。 |
| `src/stores/reader.ts`、`src/App.vue` | 延遲保存、序列化 flush、切書與關閉、錯誤重試，管理後 discard 是否阻止舊進度回寫。 |
| `src/components/LibraryManager.vue`、`LibraryView.vue` | 操作互斥、取消／失敗處理、選取範圍、篩選後清除選取、目前閱讀狀態同步。 |
| `src/api/library.ts`、`src/stores/library.ts` | IPC 型別、命令參數、收藏與畫面狀態、錯誤提示。 |
| `src/components/BookCard.vue`、`LibraryGuide.vue` | keyboard／IME、checkbox 與封面可操作性、blob 清理、教學狀態與小視窗。 |
| `src-tauri/tauri.conf.json`、`capabilities/default.json` | CSP、視窗權限、檔案對話框與自訂 IPC 的信任邊界；權限設定是否符合實際需求。 |

備份特別確認：版本與大小限制、非法／重複項目先整批拒絕、既有來源保留、ID 不直接沿用、不存在的來源保留為離線、匯出不覆寫原檔／既有備份。JSON 包含本機來源路徑，不應當作可公開分享的匿名資料。

## 原生桌面驗收清單

請使用至少兩本測試漫畫，一本 ZIP／CBZ、一本圖片資料夾。開始前備份測試書庫與來源。

- [ ] 加入、重複加入、取消檔案選取、收藏、搜尋、最近閱讀與教學正常。
- [ ] 翻到第 2 頁，立即返回書庫／切書／正常關閉，重啟確認兩本書各自的進度與設定。
- [ ] 批次收藏／取消收藏、取消移除、成功移除；原始漫畫仍在，重新加入不套用已移除項目的舊進度。
- [ ] 改搜尋／篩選／排序後選取清除；「選取目前顯示」只選目前已顯示卡片，不暗中選隱藏項目。
- [ ] 移除目前閱讀中的書後，沒有「返回閱讀」或關閉補存失敗；再開其他書正常。
- [ ] 把來源移到測試新路徑，再重新指定 ZIP／CBZ／資料夾；收藏與閱讀設定保留。頁名重排能定位，頁名消失時頁碼不越界。
- [ ] 重新指定到已有書籍的來源會拒絕，兩本原有資料不變；選無效／空來源時不改資料。
- [ ] 重新指定目前閱讀中的書後，重新開啟使用新來源，沒有舊 session 或舊封面。
- [ ] 在有進度的書仍載入時匯出備份，確認 JSON 中有最新位置；選新檔名成功，選既有檔名不覆寫。
- [ ] 在另一個空的測試書庫還原；收藏、進度、閱讀設定保留。原漫畫不在時顯示離線，可重新指定。
- [ ] 在已有同來源的書庫還原，顯示略過數量，現有收藏與進度不被備份覆蓋；重複還原不增加重複項目。
- [ ] 損壞 JSON、較新版本、重複來源、超過 16 MiB、無效設定等皆拒絕，整個書庫不部分寫入。
- [ ] 管理動作進行中嘗試再操作或關閉視窗；等待完成後仍能正常關閉。取消對話框後恢復可操作狀態。
- [ ] 640×480、中文輸入與鍵盤導覽正常；Windows 來源路徑、Unicode、大小寫／連結別名與唯讀目的地合理處理。

雲端已原生確認管理畫面、批次收藏／取消收藏、移除與來源保留。原生匯出選檔卡在 PRoot／GTK 的 `Bad address`；調整 PRoot/GIO 環境後仍失敗，尚未完成原生匯出／還原／重新指定來源的完整流程。資料層 unit tests 及 mock 介面檢查已通過；請優先在本地補驗這個缺口。

## 交付物與判定

請產出 `docs/review-report.md`，至少包含：

1. 審查人／工具、日期、OS、Node／Rust 版本、實際 HEAD 與固定 diff 範圍。
2. 命令結果與人工驗收：通過、失敗、跳過／未執行分開列。
3. 問題清單：P0（資料毀損等重大問題）、P1（阻擋主要操作）、P2（可重現的功能／相容性缺陷）、P3（一般體驗改善）；附證據與建議，不把猜測列成已確認問題。
4. 審查結論：阻擋合併的項目、可後續改善的項目、尚未驗證的風險。

先回報問題，再由使用者決定修正與重驗。Windows 打包、安裝升級、簽章、自動更新與效能驗收仍屬正式發版前的工作；本輪審查不自動授權發版。
