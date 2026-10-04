# 書庫功能開發交接

更新日期：2026-10-04（Asia/Tokyo）。範圍：本機書庫、收藏、搜尋、續讀、閱讀狀態、標籤、自訂資訊、匯入管理、三種可切換介面與備份還原、系列書架、下一集、書籤／筆記與安全備份。

最新進度：独立複審確認 T1／T3、T2 核心及 T2-R1 後端已修復，另提既有面板殘留 T2-UI1；本次已修正面板重複保存錯誤，產品 `T2_UI1_PRODUCT_SHA_PENDING`。Linux npm21／Rust48、build／diff、八項新元件情境及兩支整合 script 通過。最新 Windows／獨立複審尚未執行，前輪 0b883c8 的 Windows49 是歷史證據。接手先讀 [最新 T2-UI1 交辦](third-phase-t2-ui1-handoff.md)，原四份報告完整保留。

## 目前狀態

| 項目 | 狀態 |
| --- | --- |
| Repository | `Racious/MangaFolio` |
| 開發分支 | `feature/series-library-reading`；固定基準 e38f7a8，產品 SHA 見第三階段交辦 |
| 核心功能提交 | `06469a6`：本機書庫、收藏、搜尋與續讀 |
| 教學功能提交 | `7fe4acb`：五步教學與附截圖文件 |
| 管理與備份提交 | `df6c134`：批次管理、重新指定來源、備份與合併還原 |
| 程式版本 | 仍為 `0.1.4`，未新增發版 tag |
| 整合與發版 | 第三階段未合併 main、未發布；基準 e38f7a8 是先前已授權的 merge |
| 審查 | 已自我檢查與測試；第三階段已完成首輪獨立審查並修正 T1–T3，最新修正尚待獨立複審／Windows 整體驗收；舊審查不涵蓋新增程式 |

這是可試用的開發版。測試通過不代表完成正式審核或商業發版驗收。GitHub Releases 的既有安裝包尚未包含此分支功能。

## 已完成與範圍限制

- 首頁封面書庫，可加入多本 ZIP／CBZ，或直接包含圖片的資料夾。
- 整本收藏、最近閱讀、部分書名搜尋、自然書名排序；搜尋支援大小寫與全形／半形正規化。
- 自動續讀，保留頁面名稱、頁碼及每本書的閱讀設定；來源消失時保留收藏與進度。
- 切書、返回書庫與正常關閉前等待儲存；失敗時顯示錯誤，正常關閉會保留視窗供重試。
- 跨書籍 session 隔離、延遲封面載入、小視窗工具列改善。
- 可收合、預設不打斷使用者的更新教學、對應區域框線、步驟與收合狀態記憶。
- 可勾選與批次收藏／取消收藏／移除書庫項目，原始漫畫保留。
- 單本重新指定 ZIP／CBZ 或圖片資料夾，保留書籍 ID、收藏、閱讀設定與進度。
- JSON 備份與合併還原，既有相同來源路徑保留、無效資料整批拒絕；匯出不覆寫既有檔案。

本輪另完成閱讀狀態、標籤、自訂資訊、批次匯入结果、來源失效入口；三種風格和三種檢視獨立可切換，主題／密度／封面尺寸記憶。第三階段已擴充頁面書籤／筆記與系列書架；未提供遞迴掃描、作者管理、全文搜尋或雲端同步。詳見 [本輪計畫](next-phase-plan.md)。

## 取得、執行與檢查

保存本地未提交修改後，依 [新功能教學](library-guide.md#取得與啟動) 取得分支。工具需求：Node.js 22、Rust 1.95.0，以及 Tauri 2 的平台系統依賴。Windows 需 C++ Build Tools 與 WebView2；Linux 需 GTK／WebKitGTK 等依賴。

```bash
npm ci
npm run tauri dev
```

`npm run dev` 只啟動前端；書庫與閱讀需要原生 Tauri IPC，不能用一般瀏覽器取代完整桌面實測。驗證命令與驗收清單見 [驗證紀錄](validation.md)。

## 架構與修改入口

| 檔案 | 職責與接手注意事項 |
| --- | --- |
| `src-tauri/src/library.rs` | SQLite schema v3、來源去重、收藏、進度、封面與書庫 IPC。資料庫較新的 schema 會拒絕開啟，不會重設資料。 |
| `src-tauri/src/book.rs` | 書籍來源路徑與格式，資料夾及 ZIP／CBZ 頁面來源。 |
| `src-tauri/src/commands.rs` | 開啟來源、還原進度、session ID、算繪及預載快取隔離。新增書籍切換流程時需保留 session 檢查。 |
| `src-tauri/src/lib.rs` | 以 Tauri app_data_dir 初始化書庫並註冊指令。 |
| `src/api/library.ts`、`src/api/covers.ts` | IPC 型別與延遲封面佇列；可見封面最多兩個工作並行。 |
| `src/stores/library.ts`、`src/lib/library.ts` | 書庫畫面狀態、加入、收藏、篩選與排序。 |
| `src/stores/reader.ts`、`src/App.vue` | 400ms 延遲儲存、序列化儲存、切書／回書庫／關閉前補存。教學以外的進度錯誤不可直接忽略。 |
| `src/components/LibraryView.vue`、`BookCard.vue` | 書庫介面、封面、收藏與操作入口。封面卸載時釋放 blob URL。 |
| `src/components/LibraryGuide.vue` | 更新教學與 localStorage 設定；儲存不可用時仍可操作教學。 |
| `src/components/LibraryManager.vue` | 確認對話框、批次操作、來源重指定、JSON 備份還原與目前閱讀狀態同步。 |
| `scripts/smoke-library-management.py` | Python Playwright 的 mock IPC 介面 smoke；不測原生檔案視窗。 |
| `src-tauri/capabilities/default.json` | 主視窗新增 allow-destroy，以便等待進度儲存後關閉。 |

新增依賴只有 Rust `rusqlite`（bundled SQLite），不需另外安裝資料庫；前端沿用既有 Phosphor 圖示；第三階段只啟用 rusqlite backup feature，無新增 npm 依賴。

## 資料、備份與相容性

- `library.sqlite3` 及 `covers/` 存於 Tauri 的應用資料目錄，identifier 為 `com.racious.mangafolio`。由 `app_data_dir()` 解析實際路徑，請勿硬編碼雲端測試路徑。
- SQLite 使用 WAL。手動備份前正常關閉所有程式實例；執行中只複製主資料庫檔可能遺漏尚未 checkpoint 的資料。要在線備份應使用 SQLite backup API。
- 書籍保存 canonical 來源路徑；漫畫不會搬入應用資料目錄。備份 SQLite 不等於備份漫畫本體。
- 重複加入同一路徑會更新資料並保留 ID、收藏與進度；單純加入不會標為已閱讀。
- 續讀優先比對頁面名稱；頁面找不到時使用範圍內的保存頁碼。
- 教學狀態位於 WebView localStorage：`mangafolio.library-guide.v1.hidden`、`mangafolio.library-guide.v1.step`，不在 SQLite。可用「開始教學」重看，不需清空書庫。
- 未提供可逆 schema 遷移方案。未來改 schema 前，先定義升級與復原流程；回退程式時保留資料備份，不要刪除資料庫來繞過版本檢查。

目前 JSON 備份 v3 與 SQLite schema v3 分別版本化；v1／v2 庫先安全快照再交易升級，v1／v2 備份仍可讀，較新版本拒絕。新增欄位／標籤契約與回滾測試見本輪計畫及驗證。備份最多 16 MiB／10,000 本書，僅包含書庫 metadata；原始漫畫、封面與教學狀態另行保存。匯出使用 create_new，不覆寫既有檔案；還原整批驗證後以 transaction 合併，同路徑不修改，新書籍產生新 ID。移除只刪資料列及自有封面快取，原始漫畫不碰；有缺失 ID 時批次操作回滾。

移除或重新指定目前載入的書時，介面先 flushProgress，再操作資料層，成功後 discardBook 清除 timer、slot、書籍 ID 與舊閱讀狀態。管理期間阻止其他書庫操作與正常關閉。重新指定來源與封面操作共用 cover_lock，再取得 SQLite connection lock；保留這個鎖順序以避免交叉等待。

## 雲端環境交接

原工作區 `/workspace/MangaFolio` 保留不變；本輪使用 `/workspace/MangaFolio-management-ui` worktree。Node／Rust、Linux 系統依賴與工具設定在 checkout 外的 `/workspace/.mangafolio-tools/`，不會隨 Git clone 帶到本地。

保留此雲端工作區時，可在 shell 執行 `. /workspace/.mangafolio-tools/env.sh` 啟用工具；該目錄的 `start.md` 記錄無實體螢幕的桌面啟動、Xvfb、PRoot 與 WebKit helper 的處理方式。這些是此環境的專用設定，不是一般本地啟動需求。先前已保存 onboarding 設定草稿；本次文件整理未變更或確認其發布狀態。

曾遇到環境離線及 `exec-server protocol error`，導致短暫無法取檔／推送。連線恢復後確認原提交仍在，並成功推送、讀回遠端 hash。問題出在雲端連線，未將其判定為產品缺陷。原始 Rust 測試日誌與額外實測截圖仍在 `/workspace/MangaFolio-review/`；Git 中保留的是整理後的紀錄與選用截圖。

## 下一位接手者的優先事項

1. 先做獨立 Code Review：進度儲存順序、關閉處理、session 隔離、SQLite 版本與錯誤處理。
2. 在 Windows 重驗教學、加入、收藏、切書、立即關閉及重啟續讀；再驗 MSI／NSIS、安裝／解除安裝及升級。
3. 發版前完成安全審查；目前配置的 CSP 為 null，應評估桌面權限、來源內容與更新流程，這項配置事實尚不構成已驗證漏洞。
4. 本輪已完成系列／標籤及管理擴充；先依 next-phase-review-assignment.md 審查及驗收，範圍外功能另行提案。

本地 Claude／Codex 的完整審查範圍、指令與交付要求見 [審查作業交辦單](review-assignment.md)。尤其補驗原生選檔、備份匯出／還原及來源重指定：此雲端 PRoot／GTK 在選檔階段遇到 `Bad address`，調整執行環境後仍未完成全流程。

本次未建立新 PR。準備合併時，以 [驗證紀錄](validation.md) 為驗收依據，透過 PR 審查。正式發版再依主 README 的版本同步、tag、release 工作流程執行；不要把分支推送當成安裝包發布。

## 本輪新增架構入口與風格擴充

- `src-tauri/src/library_metadata.rs` 管理 schema 升級、狀態、資訊及標籤；`library.rs` 整合書籍讀寫與備份交易，保留 A／R1 路徑身份規則。
- `src/lib/import.ts` 與 library store 控制逐項結果、取消／重試及互斥；`src/api/library.ts` 定義 IPC 資料契約。
- `src/lib/appearance.ts` 集中風格 registry／設定白名單，`src/stores/appearance.ts` 保存與套用，`src/themes.css` 集中語意 palette、間距、半徑與尺寸 token。新增內建風格在 registry 登記并提供明暗 token；避免將顏色散入各元件或複製功能邏輯。
- LibraryView／BookCard 共享三種檢視的資料與操作；BookEditor、TagManager、LibraryManager 負責資訊及批次操作，原生 dialog 提供編輯焦點管理。
- 頁碼／閱讀時間與 statusManual 分離；metadata、標籤不因 register／relink 覆寫。詳見本輪資料契約及實際測試。
- 外觀 localStorage 不在書庫備份；未知值／損壞使用預設，保存失敗維持可操作。來源可用性仍列庫時計算，10000 本離線本地測試不代表 UNC 來源效能。

最終審查交辦與固定產品提交見 [next-phase-review-assignment.md](next-phase-review-assignment.md)。驗證見 [next-phase-validation.md](next-phase-validation.md)，不要把先前 validation.md 的記錄當成本轮 Windows 驗收。

## 第三階段接手重點

固定基準 e38f7a8；產品 `ad0e366cb0d7c4b583968243f8d864d58fce0d3e`，後續文件提交只固定產品對應。詳見 [第三階段交辦](third-phase-review-assignment.md)、[本次驗證](third-phase-validation.md)、[實際預覽](third-phase-visual-validation.md)。

- 系列優先沿用 series／volume；每本單一系列、單層，不移动來源。書庫與系列搜尋／排序／位置各自保存。下一集僅純數字唯一相鄰，離線與歧義拒絕任意跳轉。
- `library_reading.rs`：系列交易與書籤；`library_safety.rs`：升級前 SQLite online backup、還原預覽、本機安全備份與保留 manifest。
- schema／JSON v3，v1／v2 JSON 可讀；未知版本拒絕，既有來源含書籤略過不覆寫；升級前備份涵蓋 WAL，失敗中止。備份不含來源／封面／外觀或本機自動備份設定。
- `series.ts`、`SeriesShelf.vue`、`BookDetailsPanel.vue`、`BookmarksPanel.vue`、`ReaderActions.vue`、`BackupPanel.vue` 與 backup store 共用原 store／IPC；樣式集中語意 token。書籤 dialog Teleport 到 body，不受隱藏工具列影響。
- 自動預設關閉，啟動／每 15 分鐘檢查，一天一次，可手動立即；保留 1–20，僅登記且可驗證 JSON 可清理。升級快照／未知檔案永不依自動保留數清理。
- 本次 npm21／Rust44、build及 browser 回歸通過；Linux Tauri 有隔離畫面／書籤／下一集有限實測。Windows、實體觸控／IME、完整原生檔案視窗仍未驗收。所有測試來源／DB在臨時隔離路徑。
- 正確視覺參考是使用者 `01_56_49`／`01_56_58`／`01_57_03` 三張內嵌圖，已直接檢視；雲端取不到原始 PNG 位元組，正式 references 歸檔待可下載原附件，不引用舊圖。
