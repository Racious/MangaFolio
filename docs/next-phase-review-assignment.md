# 下一階段獨立審查與實機驗收交辦

**最新 N1／N2 修正產品：`10af40e99c703c31332e00f5ab5bcd3c7ebe2ccc`。** 本次固定範圍與新驗證見 [修正複審交辦](next-phase-review-fix-handoff.md)。原審查報告保留，修正仍待獨立複審與 Windows 原生驗收。

Repository：Racious/MangaFolio。分支：`feature/library-management-ui`。
固定開發基準：`bac9674f3b0d2a5c2e19774d5294d7cbfd2f86b8`。
首輪產品提交：`08ccc3e447372ade1834e8d3d63cabc8df23c5b0`。
固定首輪範圍：`bac9674f3b0d2a5c2e19774d5294d7cbfd2f86b8` → `08ccc3e447372ade1834e8d3d63cabc8df23c5b0`。
本文件固定 SHA 與原始測試 .log 的保存由後續單獨文件提交補齊，不變更產品程式。文件提交 SHA 可用 `git log -1 -- docs/next-phase-review-assignment.md` 核對，不能代替上述產品範圍。
產品包含原 A／R1／R2 修正 `878d2bb6918fe1adbf1b0d394dbec6f6f478bdd3`；原完整修正範圍 `7f095125c5fa9fff1cc74446458d0ac4dc6fc229` → `878d2bb6918fe1adbf1b0d394dbec6f6f478bdd3`。

## 任務與授權邊界

同時閱讀 `review-report.md`、`review-fix-handoff.md`、`review-fix-report-r2.md`、`next-phase-plan.md` 及 `next-phase-validation.md`；審查實際差異、測試及產品流程，不只讀交辦單宣告通過。核對既有 A／R1／R2 與本輪 F1–F6／U1–U8 是否完整達成。

獨立結果寫入 `docs/next-phase-review-report.md`。每個問題提供位置、觸發條件、影響、嚴重度及建議修法；列出實際執行與未執行項目。未經另行授權，不自行修改產品、提交或推送。不得操作使用者實際 `%APPDATA%\com.racious.mangafolio\library.sqlite3`；用測試 Windows 帳戶／VM 的隔離資料目錄及臨時來源。不要 force push、合併 main 或發版。

## 新功能與修改入口

| 範圍 | 實作與重點 |
| --- | --- |
| F1 狀態 | library_metadata／library：statusManual 與頁面進度分離、手動／自動、批次交易；store／filter／BookCard 展示及複合篩選 |
| F2 標籤 | tags／book_tags FK、名稱及限量驗證、CRUD／批次交易；TagManager／LibraryManager／標籤篩選 |
| F3 資訊 | sourceTitle／customTitle、series／volume／notes；BookEditor、reader title、搜尋與自然排序；register／relink 不覆寫 |
| F4 匯入 | library_metadata import_book_result、lib/import、store 順序控制／取消／重試／busy；LibraryView 逐筆結果 |
| F5 失效來源 | missing filter、來源路徑說明、原 relink 整合；衝突及 ID 保存沿用 R1 |
| F6 相容性 | schema／備份 v2、v1 升級／還原、未來版本拒絕、快照與還原交易、既有來源 skip、create_new |
| U3–U5 | appearance registry/store/settings、themes.css 語意 token；風格／檢視／明暗／密度／封面尺寸獨立，BookCard／LibraryView 共用邏輯 |
| U6–U8 | 可收合窄窗導覽、焦點／控制名稱、純文字狀態、錯誤／空庫／無結果／進度、更新且自願的 LibraryGuide |
| 測試與紀錄 | tests/*.test.ts／mjs、library.rs Rust 測試；scripts/validate-next-phase.py、validate-commands.py、既有 management smoke 更新 |

其餘修改包括 API／IPC handler 註冊、App 外觀及匯入中關閉防護、全域樣式、package test 納入全部案例、教學／交接／索引／工作日誌。精確檔案清單請用固定提交的 `git diff --name-status`，不要以本表代替差異。

## 審查優先點

1. A：壓縮書名更新、自訂名保留、全形搜尋及原 ID／收藏／進度／閱讀時間／偏好。
2. R1：來源鍵只轉磁碟絕對／完整 UNC，保留特殊命名空間；開書選取 ID、別名重複衝突、一般重新加入多匹配拒絕；交易失敗不能換閱讀中的書籍。
3. R2：收藏成功只更新仍保留同 ID 的 reader；失敗／切書不誤更新；批次管理同樣核對 ID。
4. migration／restore：v1 真實舊格式、未知版本拒絕、全量驗證、SQL 中途失敗全回滾、未指派標籤、同名映射與來源 skip 不覆寫資訊。匯出不能覆寫來源／備份。
5. 狀態手動標記不清續讀，收藏／重新加入／relink／備份不遺失新欄位；異步失敗／切書不污染 state。
6. 匯入取消保留當前完成者、後續未開始；單筆失敗繼續、重試只指定失敗項、忙碌互斥與正常關閉。
7. 三種風格 × 三檢視 × 明暗都可操作，沒有三套業務邏輯；搜尋／篩選／排序／有效選取跨切換保存，保存失敗可用。

## 測試與實機清單

完整輸出及退出碼見 `next-phase-results/final/` 與 `next-phase-results/ui/`；本次 npm 15／Rust34、build、diff-check 通過。Browser mock 和 Linux 原生證據的界線見驗證文件。Windows cfg 測試及 Windows native 矩陣尚未執行。

在隔離 Windows VM／帳戶逐步驗收：

1. 保留 v1 測試庫與 v1 JSON 備份，啟動新版確認升至 v2、ID／收藏／頁名／偏好不變；拒絕較新 schema／JSON 而不部分寫入。
2. 加入 ZIP、CBZ、含點資料夾、單張圖片父資料夾；多選混合成功／破損／來源衝突，查看結果、取消／重試及 busy 控制。重新加入舊 `.cbz` 標題更新且手動 metadata 保留。
3. 建兩筆相同來源的普通／verbatim 別名舊紀錄。選任一本、一般重新加入均明確衝突，不換 ID、不刪／合併、当前 reader 不替換；失敗前後資料相同。唯一別名／離線備份恢復來源後續讀保留。
4. 閱讀翻頁、返回、收藏再回讀，確認 R2；強制操作失敗及延遲切書，不更新其他書籍。最後頁自動已讀，手動已讀／未讀不改頁碼；恢復自動。
5. 建立／改名／刪除標籤，空白／重名／超長拒絕；批次加減、組合篩選；刪標籤保留書籍。資訊編輯、重加／relink／再開書保留自訂名／系列／集數／備註。
6. 來源失效／UNC 暫離顯示路徑，不自動刪書；重新指定保留全部 metadata，指向他書來源拒絕。
7. 匯出新檔；嘗試覆寫來源／既有備份拒絕。v2 跨庫還原含未使用標籤與手動狀態；既有來源不覆寫，破損及失敗交易不部分加入。
8. 設定中輪流切換三風格、三檢視、明亮／深色／跟隨系統、密度、封面尺寸，保留搜尋、狀態／收藏／標籤篩選、排序及選取；重啟恢復。損壞 localStorage 或禁止保存仍可操作。
9. 640×480／高 DPI、長書名及標籤、大量資料；Tab／Shift-Tab、編輯 dialog 焦點與 Escape、螢幕閱讀器；中文 IME 組字時方向鍵不翻頁。明暗對比、失敗文字、無結果／空庫下一步。
10. 教學不強制彈出，開啟、跳步、關閉、重開。跨 reader／書庫、正常關閉立即重啟保留進度；管理／匯入中不能丟失正在操作。
11. 10000 本真實來源、封面解碼與 UNC 可用性檢查，記錄搜尋、首屏、滾動／載入更多耗時；現有同步來源檢查可能慢，不能把 Linux 離線測試當網路效能保證。

## 已知限制與風險

schema v2 沒有降級遷移；舊程式回退應使用升級前備份。最多 16 MiB／10000 本備份、1000 標籤／每本100；tag 重名採 Unicode lowercase，不做寬度合併。還原可新增未使用標籤，既有來源仍略過不覆寫。閱讀「已讀」可以手動與進度百分比分離，這是契約。取消不強制中斷正在解析來源。

外觀／教學 localStorage 不隨書庫備份；列表初始 60 並手動載入更多，不是完整虛擬化。來源可用性同步檢查沿用既有架構，UNC 大庫延遲待量測。實機選檔／原生 IME／Windows 未完成項目如驗證文件；沒有將 mock 當原生驗收。此輪不含發版／簽章或安裝升級驗收。
