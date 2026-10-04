# 第三階段 T2-R1 修正與複審交辦

最新面板修正 **T2-UI1**：產品 `b1a02dd65d80cb85c6b633ce65d197ac8ca4fb3f`，詳見 [本次交辦／完整驗證](third-phase-t2-ui1-handoff.md)。本次 Linux npm21／Rust48、build／diff、八項真實元件情境與兩支整合 script 通過；最新 Windows／原生流程未執行。前輪 0b883c8 的独立 Windows49 通過只涵蓋前輪；以下保留歷史交辦與證據。

日期：2026-10-04（Asia/Tokyo）。分支：`feature/series-library-reading`。
固定接手基準：`52e30f5865a0596514f5fa9e03187c3f1e28a3cf`，已包含上一輪複審報告。
本次產品提交：`0b883c85e77ea8046fa851464c80a86fbbb7c513`；後續固定 SHA／完整輸出的文件提交不另修改產品，以 Git 記錄辨識。
前輪產品：`45568a804b1228c9016f9a7fd2bd876218673b2e`。

## 問題與修正

依 `third-phase-review-fix-report.md`：T1、T3 已通過独立核對；T2 登記／成功時間問題已修正，剩餘 T2-R1（P2）為連續清理失敗後恢復成功仍留下過期錯誤。原三份第三階段審查報告完整保留。本次只改此狀態清除條件與必要回歸，不變更 schema、備份版本、24h 間隔、保留政策或來源資料。

`src-tauri/src/library_safety.rs`：使用共用 `RECOVERY_FAILURE_PREFIX` 識別現有「備份狀態恢復／清理失敗」階段。只有 `automatic_files()` 恢復與 `clean_automatic_files()` 清理均成功後，才清除此階段舊錯誤；保留原「備份已建立」及成功時間恢復的清除條件。建立／擁有權登記／寫入失敗尚未重試，不因清理成功就清空訊息。錯誤文案與對外型別維持原樣；沒有新增全庫遷移或新的備份狀態欄位。

`src-tauri/src/library_reading.rs` 新增兩個隔離 Rust 回歸：

- `consecutive_cleanup_failures_clear_error_after_recovery_without_new_backup`：保留 1 份，SQLite DELETE trigger 注入兩次連續失敗，移除 trigger，分別驗證 enabled=true 的 24h gate 內及 enabled=false 恢復。斷言錯誤清空、成功時間不變、manifest 1／有效已登記 JSON 1、原檔名與位元組不變、不另建備份；未知有效同格式 JSON、未知文件與升級快照均保留，下一次檢查仍正常。
- `cleanup_success_does_not_clear_unrecovered_backup_creation_error`：已成功備份後注入 INSERT 失敗；停用排程或 24h gate 內檢查可完成清理，但未重試的建立錯誤仍保留，成功時間與檔案數不變。

第一個回歸在未改產品前實際失敗，訊息與獨立報告吻合；修正後完整測試通過。所有資料庫／來源在測試 Fixture 臨時目錄，不操作使用者 `%APPDATA%\com.racious.mangafolio\library.sqlite3`。

## 本次驗證

對應上述產品，完整原始輸出在 `docs/third-phase-t2-r1-results/`，退出碼見 [checks.json](third-phase-t2-r1-results/checks.json)。

| 驗證 | 本次結果 | 完整輸出 |
| --- | --- | --- |
| npm test | 21／21 通過，exit 0 | [npm-test.log](third-phase-t2-r1-results/npm-test.log) |
| npm run build | 通過，exit 0 | [npm-build.log](third-phase-t2-r1-results/npm-build.log) |
| cargo test --locked --manifest-path src-tauri/Cargo.toml | Linux 48／48 通過，exit 0 | [cargo-test.log](third-phase-t2-r1-results/cargo-test.log) |
| git diff --check／cached --check | 通過，exit 0 | [diff-check.log](third-phase-t2-r1-results/diff-check.log) |
| 修正前聚焦重現 | 1 失敗，exit 101（預期，證明缺陷重現） | [before-fix-reproduction.log](third-phase-t2-r1-results/before-fix-reproduction.log)、[退出碼](third-phase-t2-r1-results/before-fix.json) |

原 A／R1／R2、N1／N2、T1、T2、系列／書籤、WAL 與交易回滾測試持續通過。此輪未改前端，不重跑 browser 或提供新 UI 截圖；本輪未執行 Linux 原生 Tauri 或 Windows Rust cfg／原生驗收。

前輪獨立 Windows npm21／Rust47、build 與四支 browser 通過，對應的是 **45568a8**，詳見原複審報告；不能當成本次產品已通過 Windows。最新 T2-R1 仍待獨立複審及 Windows 隔離複驗。

## 獨立複審範圍

- 聚焦本次：`52e30f5865a0596514f5fa9e03187c3f1e28a3cf` → `0b883c85e77ea8046fa851464c80a86fbbb7c513`。
- 累積修正：`421d2acea22fccb7f2822848c80ac5a2a265270e` → `0b883c85e77ea8046fa851464c80a86fbbb7c513`。
- 整體第三階段：`e38f7a84c33126e11a141d10c0f38f4defb8985f` → `0b883c85e77ea8046fa851464c80a86fbbb7c513`。

同時讀 `third-phase-review-fix-report.md`、另兩份第三階段原報告、前輪交辦與原 review-report；檢查真實程式差異及測試，先判定 T2-R1，再核對 T1／T3、A／R1／R2、N1／N2、S1–S6／F1–F6 是否退化。特別檢查只清除已恢復的階段、不誤清建立失敗、失敗仍回傳錯誤、成功時間與檔案數穩定、未知檔保護。新結果寫入 `docs/third-phase-t2-r1-review-report.md`，不覆寫既有報告。問題提供位置、觸發條件、影響、證據與建議；未另授權，不自行修改、提交、推送、合併或發布。

## 隔離 Windows 待複驗與限制

使用 VM／專用測試使用者並先確認 app_data_dir 隔離。執行 npm test／build／cargo --locked（含 Windows cfg）。在临時 DB 重現上述連續 DELETE 失敗，分別在停用及 24h gate 內移除故障，面板錯誤應消失、最近成功時間不變、已登記備份維持 1 份；再注入建立失敗，僅清理成功時錯誤不可消失。

原生 ACL／ENOSPC、UNC、選檔／關閉重啟、IME／觸控、高 DPI、輔助科技、完整三檢視返回矩陣與正確原 PNG 歸檔仍沿用原交辦待辦。原完整未登記 JSON 不自動收編／刪除。狀態識別沿用現有文字前綴，生成與恢復共用常數；未新增持久化階段列舉，未來若改錯誤分類須同步調整此契約與測試。沒有用目前的測試綠燈宣告整體實機驗收完成。
