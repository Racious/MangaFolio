# 第三階段 T1／T2／T3 修正複審交辦

最新面板修正 **T2-UI1**：產品 `b1a02dd65d80cb85c6b633ce65d197ac8ca4fb3f`，詳見 [本次交辦／完整驗證](third-phase-t2-ui1-handoff.md)。本次 Linux npm21／Rust48、build／diff、八項真實元件情境與兩支整合 script 通過；最新 Windows／原生流程未執行。前輪 0b883c8 的独立 Windows49 通過只涵蓋前輪；以下保留歷史交辦與證據。

最新 T2-R1 補強產品：`0b883c85e77ea8046fa851464c80a86fbbb7c513`，交辦與本次 Linux npm21／Rust48、build／diff 输出見 [T2-R1 修正交辦](third-phase-t2-r1-handoff.md)。最新 Windows 尚未執行；45568a8 的獨立 Windows47 通過不涵蓋本次。下方保留前輪歷史範圍與證據。

日期：2026-10-04（Asia/Tokyo）。分支：`feature/series-library-reading`。

固定接手基準：`421d2acea22fccb7f2822848c80ac5a2a265270e`（已含兩份獨立審查報告）。
修正後產品提交：`45568a804b1228c9016f9a7fd2bd876218673b2e`。後續固定 SHA 的文件提交不另修改產品；文件提交以 Git 記錄辨識。
原第三階段產品：`ad0e366cb0d7c4b583968243f8d864d58fce0d3e`；完整開發基準：`e38f7a84c33126e11a141d10c0f38f4defb8985f`。

## 範圍與行為

本次只修正獨立報告確認的 T1、T2、T3，維持 schema／JSON v3，沒有全庫遷移、新增功能或主線合併。保留 `third-phase-review-report.md`、`third-phase-review-report-r2.md` 原文。

| 問題 | 修改與失敗契約 | 回歸證據 |
| --- | --- | --- |
| T1 Windows 升級同步 | `src-tauri/src/library_safety.rs` 升級前 SQLite online backup 後，以具讀／寫權限的 handle 呼叫 sync_all。真實快照或同步失敗仍阻止 migration，涵蓋 WAL，不複製單一 SQLite 主檔替代一致性備份。 | v1、v2／WAL、保留原資料與快照；DDL 回滾測試新增錯誤內容斷言，必須抵達 backup_settings 已存在的 DDL，不能由同步提早失敗而假通過。Linux 本次通過，Windows 尚未執行。 |
| T2 登記／狀態失敗 | `library_safety.rs` 先 create_new 保留新空檔，再 INSERT 本程式擁有權，登記成功才寫入及同步完整 JSON，最後更新成功時間／清理。INSERT 失敗僅移除本次空檔，不更新成功時間，也不碰既有成功備份。完整 JSON 已建立而狀態／清理失敗時保留檔案並回報部分成功。 | INSERT trigger 失敗連續強制／排程重試；已有成功備份位元組與時間不變；成功時間 UPDATE 失敗後重啟，即使停用排程仍先恢復時間，無重複檔案；清理 DELETE 失敗後在 24h gate 前重試。 |
| T2 恢復限制 | 恢復／清理先於 enabled／24h gate，僅認 manifest 已登記、嚴格名稱、有效 JSON regular file；不收編未知／未登記／symlink。先前清理已刪檔但 SQL 失敗，可重試移除殘留登記。`library.rs` 將成功狀態更新移出低階 write_export，手動 JSON 匯出仍記錄最近成功。 | 未知有效同格式 JSON、未知文件、升級快照保留；目錄 I/O 失敗；原還原／預覽交易測試保留。 |
| T3 返回位置 | `stores/library.ts` 保留一般書庫、系列詳情各自的展開數及系列入口展開數；保存可見書籍／系列 ID 錨點與偏移。`LibraryView.vue` 依 700px 斷點追蹤內／外真正 scroll owner，等待 refresh 完成及 DOM layout，再恢復；尺寸切換亦依錨點恢復。搜尋／排序／篩選改變才重設對應展開範圍，進出系列及 reader 不重設。 | 真實 Vue DOM：120 本開第 80 本、120 冊系列內开書、120 個系列第 80 個詳情／reader／返回；640×480 外容器；窄窗轉寬窗錨點；所有返回展開數及 scroll 差 <3px，無 pageerror。 |

其餘修改：`SeriesShelf.vue`、BookCard 傳入的穩定 data attributes 供錨點與 DOM 回歸使用；`scripts/validate-third-phase-review-fixes.py` 新增 T3 回歸；原第三階段 script 可指定輸出目錄，避免覆寫歷史證據。資料層測試在 `library_reading.rs`，僅隔離臨時來源／DB。

## 本次驗證與證據界線

本次 Linux npm **21／21**、Rust **46／46**、build、git diff --check 通過；四支 browser script 通過。完整輸出、退出碼、實際執行截圖與失敗嘗試說明見 [修正驗證](third-phase-review-fix-validation.md)。原 Linux 44／Windows 43 通過 2 失敗是歷史審查結果，不作為本次通過證據。

本次未執行 Windows Rust cfg／Windows 原生驗收，也未重新執行 Linux 原生 Tauri 流程。browser 使用真正 Vue 元件加隔離 mock IPC，不能證明 Windows 檔案 handle、原生選檔／ACL／ENOSPC／IME／觸控或全部可及性。

## 複審範圍與結果

1. 聚焦修正：`421d2acea22fccb7f2822848c80ac5a2a265270e` → `45568a804b1228c9016f9a7fd2bd876218673b2e`。
2. 完整第三階段：`e38f7a84c33126e11a141d10c0f38f4defb8985f` → `45568a804b1228c9016f9a7fd2bd876218673b2e`。

同時閱讀兩份原第三階段報告、原 review-report、A／R1／R2 及 N1／N2 交辦。檢查實際差異、測試與流程，先判 T1／T2／T3 是否修復，再確認 S1–S6、既有 F1–F6、A／R1／R2、N1／N2 沒有退化。特別審查同步 handle、DDL 失敗入口、備份建立／登記順序、成功时间與 retry gate、未知檔保護及 scroll owner／refresh 時序。新結果寫入 `docs/third-phase-review-fix-report.md`，不覆寫原報告。問題需附位置、條件、影響、證據與建議；未另授權，不自行修改、提交、推送或合併。

## Windows 隔離複驗

使用 VM 或專用測試使用者／識別碼，確認 app_data_dir 隔離，禁止接觸 `%APPDATA%\com.racious.mangafolio\library.sqlite3`。

- 執行完整 npm／build／cargo --locked，包含 Windows cfg；v1、v2 含 WAL 開啟可升級，快照包含全部原資料。注入快照／sync 真實失敗須中止；DDL collision 須斷言實際 DDL 错誤、schema 與原資料不變。
- 在臨時 DB 注入 automatic_backups INSERT 失敗：不得留下完整未登記 JSON／更新成功時間；已有成功檔保留；排程下一次可以重試。注入 backup_settings UPDATE 失敗：完整已登記檔保留，重啟在 24h 前恢復，不重複建立。DELETE 失敗：重試清理殘留登記；未知文件／有效但未登記 JSON／symlink／升級快照不碰。補真正 ACL／ENOSPC。
- >60 本、>60 系列、系列內第 80 冊，grid／detail／compact，寬窗及 640×480，開書返回、詳情返回、窄寬切換；核對展開數、搜尋／篩選／排序、有效選取及錨點。確認資料刷新慢／來源失效仍可操作，不切換錯誤書籍。
- 依原第三階段清單回歸收藏同步、來源別名衝突不換 ID、手動／自動狀態、下一集保存失敗、書籤定位、還原交易、三風格與工具列覆蓋。

## 已知限制

Windows 最新修正尚待獨立複審。旧 bug 已留下的完整但未登記 JSON 不自動收編或刪除，使用者可手動檢視／還原；部分空檔／無效檔也不會被當成成功備份。備份發生 I/O 錯誤時移除本次不完整檔為 best effort，錯誤保留。錨點只保存本次執行期間的返回位置，不新增跨重啟位置功能；篩選後錨點不再存在時採原捲動數值並由容器自然限制。參考原 PNG 歸檔及完整實機視覺／可及性驗收仍沿用原交辦的待辦。
