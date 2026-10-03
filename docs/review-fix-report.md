# MangaFolio A／R1／R2 完整修正範圍審查

日期：2026-10-03（Asia/Tokyo）。審查者：Codex（本次與實作者為同一代理，不宣稱獨立審查者驗收）。

## 結論

在實際程式差異、Linux 回歸測試與靜態檢查涵蓋的情境中，A、R1、R2 符合修正要求，未確認需阻擋此修正的新增缺陷。本輪 R1 補強未確認引入功能回歸。此結論不是 Windows 真實路徑、原生介面或正式發版通過。

## 固定審查版本

- **完整修正範圍**：`7f095125c5fa9fff1cc74446458d0ac4dc6fc229` → `878d2bb6918fe1adbf1b0d394dbec6f6f478bdd3`。A、R1、R2 判定以此範圍為準。
- **本輪補強差異**：`197668666a08507a8af796b50befe61b99b8c33e` → `878d2bb6918fe1adbf1b0d394dbec6f6f478bdd3`。另外檢查 selected ID、來源衝突、交易與 namespace 變更是否影響原功能。
- **交辦文件版本**：`17ff0fb4d1b09500d67bd53fdfcc6b6bd047eef6`。已確認與產品修正提交只有 `docs/review-fix-handoff.md` 差異，沒有產品程式變更。
- 同時閱讀原 `docs/review-report.md` 的 R1／R2 觸發與期望，未僅憑交辦單或舊測試紀錄判定。
- 驗證工作區 HEAD 為文件提交，起始乾淨；其產品 tree 與固定產品 head 相同。

## 需求判定與證據

| 項目 | 判定與實際證據 |
| --- | --- |
| A 書名 | `book.rs` 壓縮檔分支使用 `file_stem()`，資料夾與單圖父資料夾仍使用 `dir_title()`。Rust 測試真實建立 ZIP／CBZ、注入含副檔名舊標題，再重新加入，檢查不含副檔名的新標題及 ID／收藏／進度／最近閱讀時間／偏好；含點資料夾與單圖父資料夾名稱保持完整。 |
| A 搜尋 | 搜尋本來僅比對 title，沒有比對 path。新增全形 `ｂ` 測試通過；它證明搜尋邏輯，不單獨證明 Rust 標題產生，兩者分別由各自測試覆盖。 |
| R1 還原 | `restore_json` 驗證整份備份後才開始交易，來源鍵去重，重複來源整批拒絕；既有來源合併略過、不覆寫既有狀態。離線路徑保留原字串，重新上線後以 canonical 來源比對找回還原 ID。相關 Rust 回歸通過。 |
| R1 選取 ID | `open_library_book` 明確將 `Some(id)` 傳入註冊；在 transaction 內重查 ID、驗證來源一致，掃描全部匹配，不走精確路徑／第一筆優先。多筆來源匹配在任何 UPDATE／INSERT 前拒絕。選取兩個舊 alias ID 均回傳衝突，並未換 ID、合併或刪除。 |
| R1 失敗保護 | 真實 SQLite 舊重複紀錄測試比對完整備份、sqlite_sequence、目前 BookSlot Arc、session、generation、next_session；衝突後均不變。來源不一致、不存在 ID、trigger 注入 UPDATE 失敗測試均保持備份不變。一般重新加入亦拒絕多匹配。 |
| R1 正常開書 | 唯一來源别名還原與離線重新上線整合測試執行實際 open_source；檢查選取 ID、收藏、進度／頁名、偏好及儲存路徑。重新加入保留最近閱讀時間；成功開書依既有 mark_opened 更新最近時間，並非不當狀態覆寫。 |
| R1 Windows 鍵 | 只轉換磁碟絕對路徑及具有 server／share 的 UNC；Volume、GLOBALROOT、device、磁碟相對路徑與不完整 UNC 保持原樣。純字串測試及離線 Windows 別名去重測試通過；Windows 真實 canonicalize 仍未執行。 |
| R2 收藏同步 | `library.toggleFavorite` 在 IPC 成功後更新卡片及 bookId 相同的 reader；catch 不更新收藏，回應時若 reader 已切到其他 ID 不寫入其狀態。真實 Pinia stores／mock IPC 三項測試涵蓋成功／取消及 reader 下一次 toggle、失敗、等待期間切書。 |

前端 `reader.loadBook` 在 await 開書成功後才寫入新書與清空 slots；IPC 衝突失敗只回傳 false 並記錄 error。此段已靜態檢查；新增的當前閱讀不切換測試直接驗證 Rust 狀態，並非原生 UI 人工證據。

## 補強回歸檢查

- source_id 從精確／第一筆優先改為唯一匹配，符合舊重複來源必須拒絕的需求；relink 同樣以來源鍵檢查其他 ID，不會自動合併。
- register_selected 的查找、metadata 更新、讀回與 commit 在同一 IMMEDIATE transaction；錯誤時 Drop 回滾。相同 path 仍採原 INSERT ON CONFLICT，沒有調整 AUTOINCREMENT。
- 開書衝突在 mark_opened、session 增量與 BookSlot 替換前返回；收藏及閱讀狀態未默默取另一筆。
- 本輪未變更 A、R2 產品程式；完整範圍檢查與重跑測試支持兩者保留。
- 不全面正規化儲存路徑、不清理舊重複資料；這些是範圍約束，不視為遺漏修復。

## 本次新驗證

| 命令 | 結果 | 本次完整輸出 |
| --- | --- | --- |
| `npm test` | exit 0；7 passed，0 failed | `docs/review-fix-review-results/npm-test.json` |
| `npm run build` | exit 0；TypeScript／Vite 成功 | `docs/review-fix-review-results/npm-build.json` |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | exit 0；26 passed，0 failed；main／doc 零案例 | `docs/review-fix-review-results/cargo-test.json` |
| `git diff --check 7f095125 878d2bb` | exit 0；無輸出 | 本次命令實際執行，未沿用舊紀錄 |

Linux 環境沿用既有工具鏈與編譯快取；測試執行真實來源 IO 與臨時 SQLite，未啟動正式應用，未碰使用者實際資料庫。

## 未執行與剩餘限制

- 未執行 Windows 專屬 `windows_normal_restore_then_verbatim_open_keeps_id`，未驗真實磁碟一般／verbatim／UNC、大小寫與連結別名。純字串測試不代替此驗收。
- 未執行原生 UI 收藏、返回閱讀、錯誤提示、目前畫面保留、原生檔案視窗驗收；未驗打包、安裝或自動更新。
- 來源檔案系統外部變動競態、離線且無法解析的不同連結別名、網路來源 canonicalize 延遲仍是已披露限制。多筆旧資料需另外確認处理，程式不自動合併。
- register transaction 不包含後續 mark_opened；若該後續 SQL 另行失敗，註冊已更新的標題／頁數可能已提交。這是開書流程原有邊界，不能由本次註冊回滾測試推論「所有開書錯誤都不修改資料」。本輪針對的來源衝突會在任何註冊寫入前拒絕，未確認違反本次衝突失敗要求。

本次僅產生審查報告與新驗證輸出，未修改產品程式、commit、push、合併或發布。後續本地 Claude／Codex 可沿上述固定範圍獨立複驗。
