---
title: 第三階段 T1／T2／T3 修正複審
type: review
date: 2026-10-04
reviewer: 天城／Codex 桌面互動審查
---

# 第三階段修正複審

## 結論與版本

**T1、T3 的修正通過本輪核對；T2 原登記／成功時間問題已修正，但重试恢復仍有 1 項 P2，尚不宜將三項全部結案。** Windows 原始 Rust 測試本輪 **47／47 通過**，已消除上一輪 v1／v2 升級的兩個失敗。另在隔離副本增加缺陷重現，確認連續清理失敗後，恢復成功仍留下過期錯誤訊息。

- Brief：`docs/third-phase-review-fix-handoff.md`。
- 聚焦比較：`421d2acea22fccb7f2822848c80ac5a2a265270e` → `45568a804b1228c9016f9a7fd2bd876218673b2e`。
- 完整第三階段比較：`e38f7a84c33126e11a141d10c0f38f4defb8985f` → `45568a804b1228c9016f9a7fd2bd876218673b2e`，沿用前兩份報告已核對的契約，聚焦修正影響與回歸。
- 開工 checkout HEAD：`79071d462ebbe17f9b49e102e9e19791d13ac244`，分支 `feature/series-library-reading`；產品 head 後為文件／驗證紀錄，沒有新增產品程式差異。
- 行號對應固定產品 `45568a8`。原工作區已有 `src-tauri/Cargo.toml` 修改，未納入固定產品驗證、未更動。
- 已核對修正 diff、前兩份第三階段報告、原交辦／計畫／驗證及既有修正契約；未覆寫歷史報告或證據。

## 必要修正

### T2-R1 — [P2] 連續清理失敗後，恢復成功仍保留失敗狀態

- **位置**：`src-tauri/src/library_safety.rs:248–254`，相關錯誤覆寫在 `:308–316`；畫面顯示在 `src/components/BackupPanel.vue:139–143`。
- **觸發**：保留數為 1，已有成功備份；立即建立第二份時，刪除舊檔成功，但 manifest DELETE 失敗。首次錯誤以「備份已建立」開頭。下一次排程檢查若仍無法 DELETE，`automatic_files()` 恢復失敗，改寫為「備份狀態恢復／清理失敗」。之後失敗原因消失，再檢查即可清掉殘留登記。
- **根因**：成功時間在第一次建立新備份時已更新，第三次檢查的 `recovered` 為 false；清理成功分支只清除「備份已建立」前綴，沒有清除前次重試寫入的恢復／清理錯誤。它回傳 `Ok(settings)`，但 `settings.last_error` 仍是第二次的失敗訊息。
- **影響**：備份與保留清理已恢復正常，面板仍顯示失敗且表示下次會重試。停用排程後也會執行恢復，但此錯誤可持續保留至另一次完整備份成功，使用者無法從狀態判斷問題已解決。沒有確認此路徑造成備份資料遺失。
- **獨立證據**：僅在固定產品的暫存副本追加 Rust 測試，以 SQLite `BEFORE DELETE ON automatic_backups` trigger 注入失敗，連續呼叫強制建立與非強制重試；移除 trigger、停用排程後再檢查。結果為 manifest **1**、有效備份 **1**、`enabled=false`，呼叫成功但 `last_error` 仍等於第二次錯誤；再檢查一次也未清除。缺陷斷言測試 **1／1 通過**，代表重現成立，並非產品行為正確。
- **建議**：成功完成恢復／清理時，清除對應階段的既存錯誤。宜以明確狀態識別階段，或至少涵蓋目前恢復／清理錯誤分支，避免僅靠單一文案前綴；不要順帶清除尚未恢復的「建立失敗」。補「連續兩次失敗 → 原因移除 → 停用或 24h gate 內恢復」案例，斷言錯誤清空、成功時間穩定、無重複新備份、保留數與未知檔保護成立。

## 修正完成度及回歸

| 範圍 | 本輪判定 |
| --- | --- |
| T1 Windows 升級 | 可寫 handle 同步；Windows v1／v2 含未 checkpoint WAL 測試通過。DDL collision 斷言確實抵達 DDL 錯誤，原 schema／資料回滾測試通過。原 T1 可結案；真實 ACL／ENOSPC 仍未驗 |
| T2 登記順序 | create_new 空檔 → manifest INSERT → 完整 JSON 寫入／同步 → 成功狀態；INSERT trigger 失敗不留下完整未登記 JSON、不推進成功時間，已有成功檔保留。原 T2 核心缺陷已修正 |
| T2 重試／清理 | 恢復先於 enabled／24h gate；成功時間 UPDATE 失敗後重啟、一次 DELETE 失敗恢復等既有測試通過。連續失敗的錯誤狀態受 T2-R1 阻擋 |
| T3 返回位置 | 展開範圍存入 store、依實際 scroll owner 保存 ID 錨點，刷新與 layout 後恢復；5 組真實 Vue／mock IPC 回歸通過，原 T3 可結案。完整原生三檢視矩陣仍待驗 |
| S1／S2／S3 | 系列批次與聚合、相鄰唯一集數、save／load 失敗保留 reader、詳情及收藏入口的相關測試／browser 回歸通過；沒有確認新增退化 |
| S4 | 書籤 CRUD／限制、頁名定位、缺頁不换 session、純文字及還原 ID 映射／交易回滾相關測試通過 |
| S5／F6 | Windows 舊 DB 升級及 JSON 相容、預覽唯讀、還原回滾、未知檔保護相關測試通過；備份狀態完整完成宣稱仍受 T2-R1 限制 |
| S6 | 三風格、閱讀器工具列／dialog／錯誤／browser IME guard 等 script 本輪重跑通過；不等同原生 IME、觸控及完整可及性驗收 |
| A／R1／R2、N1／N2、F1–F5 | 本輪既有前端／Rust／browser 回歸通過，沒有確認新增退化；不重新要求既有 v2 自動狀態全庫回填 |

T3 五組結果：120 本寬窗第 80 本返回 `15945 → 15945`；640×480 外容器 `24109 → 24109`；窄轉寬保留書籍錨點與偏移；系列內第 80 冊 `15570 → 15570`；120 個系列進第 80 個詳情／reader 後返回 `13487 → 13487`。各組 `pageErrors=[]`。原始 script 的檢視設定不代表每組均測遍 grid／detail／compact。

## 本輪執行證據

驗證使用固定 `45568a8` 的 Git archive 副本，系統暫存目錄 `mangafolio-third-fix-45568a8-20261004/source`；測試產物亦在暫存範圍。沒有啟動正式 Tauri app identifier 或操作使用者 APPDATA。

| 項目 | 結果 |
| --- | --- |
| npm ci | offline／ignore-scripts，84 packages，成功 |
| npm test | **21／21 通過**，exit 0 |
| npm run build | vue-tsc／Vite、1613 modules，**通過**，exit 0 |
| cargo test --locked --offline --manifest-path src-tauri/Cargo.toml | Windows 原始 **47／47 通過**，exit 0；在追加獨立重現前執行 |
| validate-next-phase.py | **通過**，exit 0 |
| validate-third-phase.py | **通過**，exit 0 |
| validate-immersive-reader.py | **通過**，exit 0 |
| validate-third-phase-review-fixes.py | **5 組通過**，exit 0 |
| 聚焦固定範圍 git diff --check | 從固定副本執行，使用該副本 attributes，**通過**，exit 0 |
| 追加 independent_retry_review | **1／1 缺陷重現成功**，exit 0；不計入原始 47 項 |

四支 browser 的退出碼、輸出與 T3 數值在暫存副本 `independent-review/checks.json`、各 script `.log`、`browser-scroll/results.json`。独立重現測試僅追加在暫存副本 `src-tauri/src/library_safety.rs`，名稱為 `consecutive_cleanup_failures_leave_stale_error_after_recovery`；原程式未追加測試或修正。隔離 Vite 伺服器已停止。

## 驗證界線與接手

- 本輪補上 Windows Rust／cfg 證據，**沒有完成 Windows 原生 Tauri 實機驗收**；browser 為真實 Vue 加 mock IPC，不證明原生選檔、UNC／ACL／ENOSPC、關閉重啟、實體 IME／觸控、高 DPI、螢幕閱讀器或完整三檢視返回矩陣。
- 本輪未重新執行 Linux Rust／原生流程；交辦的 Linux 46／46 為主導方證據，與本輪 Windows 47／47 分開。沒有將 screenshot 檔案存在視為完整視覺驗收；正確參考 PNG 歸檔與原生視覺待辦沿用原交辦。
- 原本完整但未登記 JSON 不收編／刪除，以及本次執行期間的錨點保存，符合已知限制，未另提需求。
- 主導方應逐項評估本報告：確認 T1／T3 結案，處理 T2-R1 並補連續失敗回歸；審查方不代填裁決。此次僅新增指定報告，沒有修改產品、commit／push／合併或發布。
