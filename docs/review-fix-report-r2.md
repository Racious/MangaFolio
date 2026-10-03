---
title: MangaFolio A／R1／R2 修正獨立複審（第二輪）
type: review
date: 2026-10-03
reviewer: 天城／Codex 桌面互動審查（GPT-6，獨立於本批修正實作）
---

# 結論

**本輪固定範圍內無需先處理的重大發現。A、R1、R2 的修正符合交辦要求，前輪兩項 P2 可在本輪已覆蓋的情境下結案；未確認阻擋這批修正合併的新缺陷。** Windows 資料層測試補驗通過，仍不代表原生介面或正式發版驗收通過。

本輪只審查，未修改產品程式、既有設定或測試；未 commit、push、合併、發版或使用正式書庫。原 `docs/review-fix-report.md` 是實作者自審紀錄，保留原檔，本次獨立結論另存此第二輪報告。

## 實際版本與範圍

| 項目 | 值 |
| --- | --- |
| 日期 | 2026-10-03，Asia/Tokyo |
| 交辦／前輪文件 | `docs/review-fix-handoff.md`、`docs/review-fix-report.md`、`docs/review-report.md` |
| 本地實際 HEAD | `32ca3024bffd2ca08a4ca423eba32de951e9070e` |
| 分支 | `feature/library-favorites-resume` |
| 完整修正比較 | `7f095125c5fa9fff1cc74446458d0ac4dc6fc229` → `878d2bb6918fe1adbf1b0d394dbec6f6f478bdd3` |
| R1 補強比較 | `197668666a08507a8af796b50befe61b99b8c33e` → `878d2bb6918fe1adbf1b0d394dbec6f6f478bdd3` |
| 產品 tree 核對 | `git diff --quiet 878d2bb HEAD -- src src-tauri package.json package-lock.json tests`，exit 0；HEAD 後續差異僅文件與驗證輸出 |
| 測試環境 | Windows x64；本機 Node v22.22.3／npm 10.9.8、Rust／Cargo 1.95.0（沿用本輪對話已實查工具鏈） |
| 測試來源 | `git archive 878d2bb` 匯出固定提交到獨立暫存副本，未包含本地 Cargo.toml 修改或未追蹤檔案 |

完整比較涉及 19 檔；產品／測試修改為 `package.json`、`book.rs`、`commands.rs`、`library.rs`、library store 與兩份前端測試，其餘為文件與歷史驗證輸出。沒有將 Linux 歷史輸出當成本輪執行證據。

## 逐項複審

| 項目 | 判定 | 實際程式與本輪證據 |
| --- | --- | --- |
| A：ZIP／CBZ 書名去副檔名 | 修正成立 | `src-tauri/src/book.rs:112` 採 file_stem，資料夾與單圖父目錄仍用完整 dir_title。`archive_titles_reimport_preserves_state_and_folder_dots` 在 Windows 建立真實 ZIP／CBZ，注入舊標題後重新加入，驗證新標題與 ID／收藏／進度／偏好／最近時間保留。 |
| A：全形搜尋 | 修正／防回歸證據成立 | 前端仍僅比對 title；`full-width b searches titles without matching archive path extensions` 通過。此測試與 Rust 標題產生測試分別驗證各自職責。 |
| R1：來源鍵與合併還原 | 修正成立 | `library.rs:108` 的 source_key 在可解析時 canonicalize；Windows 一般／verbatim 磁碟、完整 UNC 使用一致鍵。restore 在完整驗證後進行交易，來源別名重複拒絕、既有來源略過、儲存路徑不全面改寫。別名合併、重複拒絕、離線重新上線測試通過。 |
| R1：保留選取 ID | 修正成立 | `commands.rs:103` 傳入 Some(id)，`library.rs:233` register_selected 在 IMMEDIATE transaction 中重查選取 ID、來源一致性並掃描全部來源匹配。多筆匹配在 metadata 更新、mark_opened 與 session／BookSlot 切換之前拒絕，不默默取另一筆。 |
| R1：衝突與 SQL 失敗保護 | 修正成立 | 選取兩個舊 alias ID、一般重新加入、來源不一致、不存在 ID 與 trigger 注入 UPDATE 失敗均通過；斷言涵蓋備份不變、sqlite_sequence、BookSlot Arc、session、generation 與 next_session。 |
| R1：原始 Windows 觸發條件 | 本輪補驗通過 | **`windows_normal_restore_then_verbatim_open_keeps_id` 在真實 Windows IO 執行通過**，驗證還原後 ID、收藏、進度保留且書庫只有一筆。Linux 報告原列未執行的這項缺口已補上。 |
| R2：卡片收藏同步閱讀器 | 修正成立 | `src/stores/library.ts:83` 在 IPC 成功後只更新相同 bookId 的 reader。真實 Pinia store／mock IPC 三項測試通過：收藏／取消與下一次 reader.toggleFavorite、失敗不污染狀態、等待期間切書不污染新 reader。 |

確認不是僅以測試數量判定：已閱讀來源鍵轉換、所有匹配檢查、交易提交／錯誤返回、開書狀態替換順序與前端成功／失敗分支。R1 不自動合併或刪除舊重複資料，符合本輪交辦約束。

## 本次執行結果

| 命令 | 結果 |
| --- | --- |
| `git diff --check 7f095125 878d2bb` | exit 0，無輸出 |
| `npm ci --ignore-scripts --no-audit --no-fund --cache ../npm-cache` | exit 0，added 83 packages；固定 lockfile，安裝 lifecycle scripts 未執行 |
| `npm test` | exit 0，**7 passed、0 failed** |
| `npm run build` | exit 0，TypeScript／Vite 6.4.3 通過；62 modules |
| `cargo test --locked --offline --manifest-path src-tauri/Cargo.toml` | exit 0，**27 passed、0 failed**；main／doc-tests 零案例不另計 |

Rust 與 Linux 自審 26 項的差異，是本機實際執行了 Windows 專屬測試；沒有把歷史結果累加成新案例。Rust 使用先前暫存 target 快取，但編譯來源為本次 `878d2bb` 副本；cargo 輸出確認重新編譯本次副本，未沿用先前追加缺陷重現測試的 source。

關鍵輸出：

```text
running 27 tests
test library::tests::windows_normal_restore_then_verbatim_open_keeps_id ... ok
test commands::tests::duplicate_sources_reject_both_selected_ids_and_import_without_switching_reader ... ok
test commands::tests::unique_alias_and_offline_restore_open_selected_id_with_reading_state ... ok
test library::tests::selected_source_mismatch_and_sql_failure_leave_data_unchanged ... ok
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

本機隔離副本：`C:/Users/Racious/AppData/Local/Temp/mangafolio-review-fix-878d2bb-20261003/source/`。依賴安裝與前端建置在工具核准後於此副本執行，避免先前已確認的沙箱父目錄／網路限制；沒有寫回原產品工作區或正式應用資料目錄。

## 未驗證與剩餘限制

- 未執行原生介面人工驗收，包括收藏返回閱讀、原生檔案視窗、衝突錯誤提示與目前畫面保留。Rust BookSlot 與 Pinia mock 證據不代替 UI 實測。
- 本輪 Windows 真實 IO 覆蓋本機一般／verbatim 路徑與臨時圖片來源；沒有真實 UNC 分享、網路斷線、大小寫／連結別名完整矩陣或大型網路書庫效能證據。
- 外部來源被替換的檔案系統競態、離線且尚無法解析的不同連結目標，仍維持既有披露限制。來源匹配須解析書庫路徑，網路 canonicalize 延遲仍待實機評估。
- register transaction 不包含後續 mark_opened；不能將衝突回滾測試推論成所有開書 SQL 失敗都不改 metadata。本輪來源衝突會在註冊寫入前拒絕，沒有確認違反此次修正要求。
- 未驗打包、安裝升級、簽章、自動更新或正式發版。

## 接手

主導方可依據本報告處理 A／R1／R2 的完成判定與後續 Git 預覽；不需要為本輪新增程式修正。原生實機驗收與正式發版工作仍須另行完成。本報告不代填主導方採納／裁決，也不自動授權 commit、push、merge 或 release。
