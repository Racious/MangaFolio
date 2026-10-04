# 第三階段 T2-UI1 面板錯誤恢復修正交辦

日期：2026-10-04（Asia/Tokyo）。分支：`feature/series-library-reading`。
固定接手基準：`1d62ec7f5203e32a533ff926d89c1cf3fb16820b`（已含最新獨立報告）。
本次產品提交：`T2_UI1_PRODUCT_SHA_PENDING`；後續固定 SHA 的文件提交不另修改產品，以 Git 記錄區分。
前輪產品：`0b883c85e77ea8046fa851464c80a86fbbb7c513`。原報告 `third-phase-t2-r1-review-report.md` 及另外三份第三階段報告完整保留。

## 本次問題与修正

最新獨立報告確認 T2-R1 後端已修復，Windows Rust49／npm21 通過；剩餘 P2 **T2-UI1**：立即備份失敗被 `BackupPanel.run()` 複製到面板本地 error，後續排程雖清空 backup store 錯誤，面板仍優先顯示舊副本。此路徑原已存在，沒有確認資料遺失。

只修改 `src/components/BackupPanel.vue` 的 automatic()：維持共用 busy／flushProgress／管理互斥，呼叫 backup.maintain(true) 後，不把 store 已處理的備份錯誤再次 throw 給面板。本機自動備份錯誤直接使用模板原有的 backup.error／settings.lastError，排程更新後立即反映；兩者皆空才顯示成功通知。預覽／還原／手動匯出及進度保存錯誤仍使用原本本地 error，沒有新增成功排程清除所有錯誤的 watcher。

不修改 backend、store、App 排程、schema／JSON、24h gate、資料來源或其他 UI 功能。這不是新的視覺設計，沒有用示意圖替代功能證據。

## 新增回歸與實際畫面

`scripts/validate-backup-panel-recovery.py` 使用真正 App、BackupPanel、Pinia，加隔離記憶體 Tauri IPC。攔截 App **實際註冊的 900000ms interval callback** 加速觸發，不另實作排程／直接呼叫 store 來替代 App 路徑；每次確認 IPC 呼叫數增加及 pending 結束。前後核對同一個面板 DOM instance，沒有重新掛載掩蓋錯誤。

八項情境：

1. 真實「立即建立本機安全備份」按鈕失敗，警告可見、不顯示成功通知。
2. 排程重試再失敗，警告更新為目前恢復／清理錯誤。
3. 再次排程恢復成功，store error／settings.lastError／面板 alert 全清空，成功時間穩定。
4. 尚未重試的建立／登記失敗即使排程回傳 Ok，settings.lastError 仍有值，警告持續可見。
5. 手動匯出失敗後排程成功，本地匯出錯誤仍保留。
6. 預覽失敗後排程成功，本地預覽錯誤仍保留。
7. 還原失敗後排程成功，本地還原錯誤仍保留。
8. 正常立即備份成功，警告消失並顯示成功通知。

新腳本在修正前以實際按鈕與兩次真實排程 callback 重現：pending=false、store error／lastError 皆空，但面板仍顯示首次清理失敗（exit1，預期重現）；修正後八項通過、pageErrors=[]。

本次 [恢復後實際 browser 截圖](third-phase-t2-ui1-results/browser/recovered-panel.png) 與 [八項狀態證據](third-phase-t2-ui1-results/browser/results.json) 為真正 Vue 畫面／mock IPC，**不是原生 Tauri 或 Windows SQL／I/O 驗收**。書庫整合另跑的截圖在 `third-phase-t2-ui1-results/browser-library/screens/`；第三階段整合截圖在 `/tmp/mangafolio-t2-ui1-third-screens`，屬暫存產物。完整 JSON／raw log 已保存下表，不宣稱暫存畫面是永久交付。

## 本次驗證

完整輸出與退出碼在 `docs/third-phase-t2-ui1-results/`，對應本次產品；詳見 [checks.json](third-phase-t2-ui1-results/checks.json) 及 tested-source-sha256.json。

| 驗證 | 本次結果 | 完整輸出 |
| --- | --- | --- |
| npm test | 21／21 通過，exit0 | [npm-test.log](third-phase-t2-ui1-results/npm-test.log) |
| npm run build | 通過，exit0 | [npm-build.log](third-phase-t2-ui1-results/npm-build.log) |
| cargo test --locked --manifest-path src-tauri/Cargo.toml | Linux48／48 通過，exit0 | [cargo-test.log](third-phase-t2-ui1-results/cargo-test.log) |
| git diff --check／cached --check | 通過，exit0 | [diff-check.log](third-phase-t2-ui1-results/diff-check.log) |
| validate-backup-panel-recovery.py | 八項通過，exit0 | [browser-backup.log](third-phase-t2-ui1-results/browser-backup.log) |
| validate-next-phase.py | 通過，exit0 | [browser-library.log](third-phase-t2-ui1-results/browser-library.log) |
| validate-third-phase.py | 通過，exit0 | [browser-third.log](third-phase-t2-ui1-results/browser-third.log) |
| 修正前 UI 重現 | 失敗，exit1（預期） | [before-fix.log](third-phase-t2-ui1-results/before-fix.log)、[狀態](third-phase-t2-ui1-results/before-fix/results.json)、[退出碼](third-phase-t2-ui1-results/before-fix.json) |

修正前另一次較早斷言捕捉「第二次錯誤被第一個本地副本遮蔽」，保留 before-fix-first-retry.log；也為 exit1 的缺陷重現，不計為測試通過。既有書庫 script 首次因指定的 /tmp 截圖目錄不能 relative_to(repo) 而在結果寫出時失敗（exit1），所有操作已完成；改用支援的 repo 內獨立輸出目錄重跑成功。失敗 raw log 與退出碼保留 attempts/，沒有更改產品或放寬斷言。必要命令為本次重新執行；既有 A／R1／R2、N1／N2、T1／T2／T2-R1、系列、書籤及交易測試仍通過，沒有擴大产品範圍。

## 獨立複審與 Windows 待驗

- 聚焦 UI1：`1d62ec7f5203e32a533ff926d89c1cf3fb16820b` → `T2_UI1_PRODUCT_SHA_PENDING`。
- 累積修正：`421d2acea22fccb7f2822848c80ac5a2a265270e` → `T2_UI1_PRODUCT_SHA_PENDING`。
- 整體第三階段：`e38f7a84c33126e11a141d10c0f38f4defb8985f` → `T2_UI1_PRODUCT_SHA_PENDING`。

先閱讀最新原報告、前輪 T2-R1 交辦、另三份第三階段報告與原 review-report。審查實際程式差異、測試與操作，不只依本文件宣告通過。先判 T2-UI1，再核對既有錯誤、忙碌互斥、進度保存、其他面板操作及 A／R1／R2、N1／N2、S1–S6／F1–F6 未退化。新結果寫入 `docs/third-phase-t2-ui1-review-report.md`，原報告保留；問題附位置、條件、影響、證據與建議。未另授權，不自行修改、提交、推送、合併或發布。

Windows 使用 VM／專用測試帳戶或 app identifier，先核對隔離 app_data_dir，不得接觸 `%APPDATA%\com.racious.mangafolio\library.sqlite3`。重跑 npm／build／cargo locked（含 Windows cfg）；在隔離 DB 注入連續清理失敗，經真實立即按鈕與排程恢復後確認警告消失；保持同一面板，尚未恢復的建立、預覽／還原／匯出錯誤仍可見，恢復期間控制項禁用／完成後可用。

本次未執行 Windows Rust 或 Windows／Linux 原生 Tauri；前輪 Windows49 對應 **0b883c8**，不能涵蓋最新 UI。另未重跑 immersive-reader 與 T3 五組專用 script（未改其路徑，沿用前輪界線）；原生 UNC／ACL／ENOSPC、選檔、關閉重啟、IME／觸控、高 DPI、輔助科技、全部三檢視矩陣及正確 PNG 歸檔仍待驗。只用臨時 Rust Fixture 與 mock IPC，未操作使用者實際資料庫。無主線合併、force push 或發布。
