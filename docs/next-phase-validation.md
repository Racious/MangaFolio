# 下一階段驗證紀錄

日期：2026-10-03 UTC。固定開發基準 `bac9674f3b0d2a5c2e19774d5294d7cbfd2f86b8`，分支 `feature/library-management-ui`。這是開發者驗證，不替代獨立 Code Review 或 Windows 實機驗收。

## 本次最終驗證

| 實際命令 | 結果 | 完整輸出 |
| --- | --- | --- |
| `npm test` | exit 0，15 passed、0 failed | [npm-test.log](next-phase-results/final/npm-test.log) |
| `npm run build` | exit 0，TypeScript／Vite 通過 | [npm-build.log](next-phase-results/final/npm-build.log) |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | exit 0，34 passed、0 failed；main／doc 零案例 | [cargo-test.log](next-phase-results/final/cargo-test.log) |
| `git diff --check` | exit 0 | [diff-check.log](next-phase-results/final/diff-check.log) |
| `python3 scripts/validate-next-phase.py` | exit 0，無 page error | [完整輸出](next-phase-results/ui/full-output.log)、[結果與對比數值](next-phase-results/ui/results.json) |
| `CHROMIUM_EXECUTABLE=/usr/bin/chromium python3 scripts/smoke-library-management.py` | exit 0 | [management-smoke.log](next-phase-results/ui/management-smoke.log) |

前四項由 `scripts/validate-commands.py` 保存未刪減 stdout／stderr、UTC 執行時間及退出碼：[exit-codes.json](next-phase-results/final/exit-codes.json)。第一階段 13／33 項輸出另保留 `stage1/`，不代替本次 15／34 項驗證。未增加產品依賴、未改 lockfile；僅以臨時 npm cache 執行 Prettier 格式化本輪修改。

最後新增較新備份未知欄位案例時，測試使用 unwrap_err 需要成功型別 Debug，曾編譯 exit101；已改為取 err 後斷言，重新完整驗證。失敗完整輸出保留 `final/cargo-test-attempt1.log` 及 `exit-codes-attempt1.json`，不以該次結果宣告通過。

## 測試覆蓋

- A：壓縮檔 file_stem、資料夾完整名稱、舊壓縮標題重新加入更新而 ID／收藏／時間／進度／偏好保留；全形 ｂ 不因路徑副檔名誤命中。
- R1：選取 ID、雙來源別名明確衝突、不換 ID／不污染既有資料與目前閱讀；重新加入多匹配拒絕、離線還原重新上線、唯一別名重新開書、Windows 比對鍵磁碟／UNC／特殊命名空間。Windows cfg 實體路徑案例本輪 Linux **未執行**。
- R2：真實 Pinia store 接 mock IPC；收藏成功同步仍保留同 ID 閱讀器，失敗及切換書籍不誤更新。既有測試保留。
- schema v1 升級保留資料與失敗回滾、較新 schema 拒絕；實際缺少新欄位 v1 備份還原。
- 自訂資訊重新加入／relink／備份保留；手動狀態與進度分離、恢復自動、批次缺 ID 回滾。
- 標籤空白／大小寫重名／限長、CRUD、批次加入／移除、刪標籤不刪來源；備份未使用標籤、同來源略過不覆寫、新格式無效關聯與 SQL 失敗整批回滾。
- 匯入新增／更新／失敗／衝突、失敗後繼續、取消保留當前成功項及不啟動後續、重試與 busy 互斥。
- 外觀缺失／損壞／保存失敗、舊設定補預設風格；全部 18 風格／明暗／檢視組合保留資料、搜尋、排序與選取，重新載入恢復設定。
- 瀏覽器整合：標籤 CRUD／批次與複合篩選、資訊編輯失敗可重試、來源路徑／relink、備份失敗／合併結果、部分匯入／重試、空庫／無結果、教學開關、長名稱與標籤、640×480 無水平溢出／導航／設定、基本 Tab 與組字事件。
- 10000 本純資料搜尋／排序及隔離 SQLite 還原／來源可用性檢查／匯出；列表初始只渲染 60 本；封面佇列最多兩項並跳過取消工作。這不是 Windows 網路來源效能保證，也未量測實際 10000 本大圖解碼。
- 六套風格／明暗 token，共 36 個文字及控制項邊界色對，文字至少 4.5:1、邊界至少 3:1。這是指定 token 的測量，未宣告整個產品完整 WCAG 合規。

## 實際畫面與原生邊界

- 基準畫面：[current](images/next-phase/current/) 為 Vue／Chromium mock IPC；[native-current](images/next-phase/native-current/) 為 Linux Tauri／WebKit 舊版面及當時資料層，使用 v1 臨時庫升級，不是舊截圖替代。
- [directions](images/next-phase/directions/) 是設計示意；三種均由使用者選定實作。
- [implemented](images/next-phase/implemented/) 是本次實作畫面，隔離 in-memory IPC；同四本樣本，多套風格／檢視、窄視窗、空庫、無結果、匯入部分失敗。測試圖不是使用者漫畫。
- [native-implemented](images/next-phase/native-implemented/) 是本次 Linux 真實 Tauri 視窗：三種風格／三種檢視、管理面板、資訊編輯、續讀及返回。實際編輯自訂名稱、系列、集數、備註，唯讀核對臨時 SQLite；來源名稱、ID、收藏及第43頁保留。再開書顯示自訂名稱，翻到第44頁返回後進度保存。最終程式重新 build／啟動，夜讀／深色／緊湊設定及第44頁仍保留，畫面 10；原生 build 完整輸出 ui/native-build.log（exit0）。[隔離原生核對](next-phase-results/ui/native-checks.json)。
- 原生環境 DISPLAY=:98；XDG_DATA_HOME／CONFIG_HOME／CACHE_HOME 全部在 `/tmp/mangafolio-next-phase-native/`。沒有操作 `%APPDATA%\com.racious.mangafolio\library.sqlite3` 或原工作區測試庫。

## 未執行與待實機複驗

Linux GTK 選檔本次已開啟並輸入隔離目錄，但清單未能列出／Open 無法完成，因此未宣告原生匯入成功；畫面 `native-implemented/08-picker-uncompleted.png`，仍交辦實機複驗。

Windows native／WebView2、cfg(windows) 測試、磁碟／verbatim／UNC 上下線、原生多檔選擇及匯入取消重試、備份選檔完整流程、原生中文 IME、螢幕閱讀器與高 DPI、Windows 大型書庫／網路來源耗時、系統明暗切換均待 Windows 矩陣實驗。Browser 組字事件不是實際輸入法驗收。Linux 的原生資訊編輯與閱讀證據不替代這些流程。

未打包、簽章、發布或合併 main。schema v2 無降級遷移，回退需使用升級前備份；備份不含漫畫及外觀偏好。詳細驗收步驟見 [審查交辦](next-phase-review-assignment.md)。

完整原始 .log 保留測試工具輸出的尾端空行；本結果目錄 .gitattributes 僅允許 .log 尾端空行，不調整程式碼 whitespace 檢查。固定開發基準到最終文件提交的 diff --check 另行核對。

## 審查後 N1／N2 修正（2026-10-03，Asia/Tokyo）

產品提交 `10af40e99c703c31332e00f5ab5bcd3c7ebe2ccc`，基準 `1394670f3b4ac63ab69fcedcabb650a2b0e97879`。本次 npm16／Rust37、build、diff-check 全部exit0。完整輸出與退出碼獨立保存於 `next-phase-results/review-fix-n1-n2/`，不以首輪結果代替。雙頁末組、自動／手動、來源擴縮頁／頁名定位與 SQL 回滾已補測。Windows cfg／native 本輪未執行。詳見 [修正複審交辦](next-phase-review-fix-handoff.md)。
