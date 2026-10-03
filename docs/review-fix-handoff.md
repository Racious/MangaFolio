# A／R1／R2 修正交辦

日期：2026-10-03（Asia/Tokyo）。原補丁無法取得，本次從遠端
`feature/library-favorites-resume` 的 `7f095125c5fa9fff1cc74446458d0ac4dc6fc229`
重新實作，並非套用或宣稱尋回舊補丁。原工作區保留，在獨立 worktree 修改。

## 修正範圍

- A：ZIP／CBZ 以 `file_stem()` 顯示書名，保留書名中的點；資料夾及單張圖片所屬資料夾保留完整名稱。重新加入只更新來源標題、格式與頁數，保留 ID、收藏、進度及偏好。
- R1：依 `docs/review-report.md` 修正還原來源比對。可存取來源透過 canonicalize 比對，離線 Windows 一般／verbatim（含 UNC）使用去除命名空間前綴的識別鍵。備份內同來源重複整批拒絕；與既有來源合併略過並保留既有資料；重新開書或重新加入沿用既有 ID／閱讀狀態。relink 衝突檢查使用同一識別鍵。
- R2：單本收藏 IPC 成功後同步仍持有相同 bookId 的 reader；失敗或目前 reader 已切書時不修改 reader 收藏。
- B：只新增內部比對鍵，沒有統一或改寫備份／既有資料儲存路徑格式。C：沒有調整 AUTOINCREMENT 或相同字串原有 INSERT 衝突流程，序號跳號維持現況。
- 不新增功能，不使用正式資料庫、不啟動正式應用、不合併 main、不 force push。

## 回歸驗證

測試僅在隔離 worktree 執行。Rust 測試使用唯一臨時目錄、圖片、壓縮檔及 SQLite，結束清除。
前端測試執行真實 Pinia stores，只替換桌面 IPC：收藏、取消收藏與返回閱讀後再切換、失敗、等待期間切書。
資料層覆蓋 ZIP／CBZ 書名與重新加入狀態、含點資料夾／圖片、來源別名的合併、重複備份整批拒絕、還原後重新開書、離線還原後重新上線、Windows 命名空間識別鍵。

| 命令 | Linux 結果 |
| --- | --- |
| `npm test` | 6 passed，0 failed |
| `npm run build` | TypeScript 與 Vite 建置通過 |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | 22 passed，0 failed；main／doc 零案例 |
| `git diff --check` | 通過 |

環境沿用 `/workspace/.mangafolio-tools/env.sh` 的工具鏈與系統相依，node_modules 指向既有安裝；未改動 lockfile。Cargo target 共用編譯快取，資料庫仍只由測試 fixture 建立。

完整命令輸出保存於 `docs/review-fix-results/`。Windows 專屬真實路徑測試以 `#[cfg(windows)]` 加入，Linux 不會執行，不計入本次通過數。

## 待 Windows 隔離實機複驗

- 執行完整 Rust 測試，確認 `windows_normal_restore_then_verbatim_open_keeps_id`。
- 一般／verbatim 磁碟與 UNC 路徑：備份內別名重複拒絕、既有合併略過、離線還原後來源重新上線及重新開書，確認 ID、收藏、進度、設定。
- 同一本書開啟後返回書庫，卡片收藏／取消後返回閱讀，工具列應同步，下一次切換正確；IPC 失敗及等待期間切換其他書籍不可污染新書。
- ZIP／CBZ、含點資料夾與單張圖片的原生加入及重新加入，確認標題與狀態。
- Windows 大小寫、連結別名與斷線網路分享仍需實機複驗；離線且無法解析的不同連結目標無法僅由路徑字串判定，需來源上線後再解析。

所有原生人工驗收使用隔離測試資料庫；本次自動化結果不代表打包、更新或正式發版驗收。
