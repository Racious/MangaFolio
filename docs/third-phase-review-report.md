---
title: MangaFolio 第三階段獨立審查報告
type: review
date: 2026-10-04
reviewer: 天城／Codex 桌面互動審查
---

# 第三階段獨立審查報告

**結論：確認 1 項高風險（T1／P1）及 2 項中風險（T2、T3／P2），建議修正及複驗後再合併。** Windows 既有 v1／v2 書庫升級目前會阻擋啟動；Linux 全綠不能支持 Windows 升級可用。未修改產品、提交、推送、合併、發布或接觸使用者實際書庫。

## 固定版本與審查邊界

| 項目 | 實際範圍 |
| --- | --- |
| Repository／分支 | Racious/MangaFolio／`feature/series-library-reading` |
| Brief | `docs/third-phase-review-assignment.md`，開工時 HEAD `a6c11396f1ae829f4a235fbb14290a2c0a243936` |
| 固定 base | `e38f7a84c33126e11a141d10c0f38f4defb8985f` |
| 固定產品 head | `ad0e366cb0d7c4b583968243f8d864d58fce0d3e` |
| 比較語意 | 兩提交 tree diff；179 檔，含程式、測試、文件、截圖及歷史嘗試輸出 |
| head 之後 | `a6c1139` 僅更新 8 檔文件／證據 metadata，沒有產品程式差異 |
| 工作區狀態 | 開工乾淨；審查途中工作區由外部切回 `main` 並更新至 `e38f7a8`。審查及驗證始終使用固定產品副本，天城未切分支或同步 Git |
| 驗證環境 | Windows；Node 22.22.3、npm 10.9.8、Rust／Cargo 1.95.0 |
| 隔離 | Git archive 匯出固定產品至獨立系統暫存目錄；Rust 使用臨時 DB／來源，browser 使用實際 Vue＋in-memory mock IPC |

已閱讀交辦指定的計畫、驗證、視覺驗證、教學、handoff、原 R1／R2 報告、A／R1／R2 交辦、N1／N2 複審交辦與報告；核對實際產品差異及相關呼叫路徑。Vault MangaFolio 活頁仍是 2026-07-05 狀態，本次以固定交辦與 repo 文件為需求依據。以下行號全部對應固定產品 `ad0e366`，不套用目前 main 的行號。

## 必要修正

### T1 — 高／P1：唯讀 handle 的 sync_all 讓 Windows v1／v2 升級無法啟動

- **位置**：`src-tauri/src/library_safety.rs:28–30`；入口 `library.rs:233–235`、`lib.rs:23`。
- **觸發**：Windows 啟動已存在的 schema v1 或 v2 書庫，即使備份目錄可寫、SQLite online backup 已成功。
- **錯誤路徑／影響**：快照完成後以 `File::open` 開啟唯讀 handle，再呼叫 `sync_all()`。本機回傳 `PermissionDenied / os error 5`；錯誤經 `Library::open` 傳回 Tauri setup，中止正常啟動。原 schema 保留，但使用者不能完成升級及進入閱讀。每次重試仍會走同一路徑。
- **實際證據**：固定產品原始 Windows Rust 測試 **43 通過／2 失敗，退出碼 1**。`upgrade_snapshot_includes_uncheckpointed_wal_and_failure_keeps_v2`（`library_reading.rs:421`）及 `v1_database_migrates_without_losing_identity_or_progress`（`library.rs:1333`）均在 `Library::open` 回傳「存取被拒」。獨立最小 Rust probe 亦確認唯讀 `File::open(...).sync_all()` 失敗、同檔可寫 handle 成功，排除單純備份路徑 ACL 不足。
- **建議**：以具寫入權限的 handle 完成快照同步，例如 `OpenOptions::new().read(true).write(true)`；保留「真正備份／同步失敗就阻止升級」契約。修正後重驗 Windows v1、v2、未 checkpoint WAL、原 schema／資料及快照保留，以及真正 I/O 失敗。
- **完成度注意**：本輪 `migration_sql_failure_rolls_back_new_tables_and_keeps_safe_snapshot` 雖通過，但其斷言允許在快照同步階段提前失敗；此 Windows 結果不能證明已抵達 migration DDL 注入點。需確認修正後確實執行 DDL 再失敗、回滾。

### T2 — 中／P2：備份登記失敗的結果與成功時間不一致，且無後續登記重試

- **位置**：`src-tauri/src/library_safety.rs:158–174`、`:218–227`；共用匯出副作用 `library.rs:652–659`。
- **觸發**：自動 JSON 已完整寫入及同步，但 `automatic_backups` INSERT 或後續登記 transaction 失敗；本輪以 INSERT trigger 注入 SQL 失敗。
- **錯誤路徑／影響**：`export_to()` 已先更新 `last_success`；登記失敗時 `backup_created` 仍是 false，回傳「備份失敗」。實際新 JSON 存在，卻未被 manifest 登記，因此不納入保留清理；排程又依剛更新的成功時間跳過下一次嘗試。使用者看到失敗訊息及新成功時間，無法判斷可用備份與清理狀態；若持續強制重試，這些未登記的新檔不受保留份數控制。
- **實際證據**：隔離副本追加一項 Rust 重現，真實 SQLite／檔案系統；確認 `files=1`、`registered=0`、`last_success=Some(...)`，回傳「備份失敗…manifest failed」。下一次 `automatic_backup(false)` 回傳 Ok、檔案數仍是 1，沒有登記恢復。測試通過表示缺陷斷言成功，非產品行為通過。
- **建議**：分開表示「JSON 建立成功」「登記失敗」「清理失敗」階段；讓自動流程負責成功時間／manifest 一致性，共用寫檔函式回傳檔案建立結果。保留已建立的新備份，清楚回報部分成功，並設計登記恢復或 pending 狀態，避免 24h 建立間隔同時阻止 bookkeeping 恢復。若有效 JSON 的寫入仍定義為一次成功備份，保留該成功時間也可，但須另處理登記重試。

### T3 — 中／P2：返回位置只保存數字，未保存展開數量及實際捲動容器

- **位置**：`src/components/LibraryView.vue:32`、`:106`、`:111–136`；重新載入／掛載 `App.vue:55–56`；窄窗捲動配置 `LibraryView.vue:998–1026`。
- **觸發**：① 展開超過 60 本書，在後段開書再返回；② 640×480 窄視窗捲動書庫後開書再返回。
- **錯誤路徑／影響**：`limit`／`seriesLimit` 是元件本地 ref，閱讀後重新掛載恢復成 60；只有一次 nextTick 的 scrollTop 恢復也未等待 refresh／對應列表高度就緒。窄窗實際捲動的是 `.library-layout`，但保存及恢復一直使用 `.library-content`。因此無法達成交辦 S2 的返回位置恢復，使用者需重新展開與尋找書籍。
- **實際證據**：實際 Vue、100 本隔離 mock 書籍、1440×1000，展開至 100 本後開第 80 個卡片再返回：`renderedBooks 100 → 60`，`scrollTop 9486 → 0`。640×480 追加流程：外層 `layoutScroll 3293 → 0`，內層 `contentScroll` 一直是 0。pageErrors 為空，兩個缺陷斷言皆成功。既有第三階段腳本只有小型列表／首頁與系列往返的情境，未覆蓋這兩項。
- **建議**：以 store 保存各檢視的展開範圍或可恢復的書籍錨點，列表資料及渲染完成後才恢復；寬窄布局統一捲動容器，或明確追蹤當前 owner。補大於 60 本／60 系列、系列內返回、640×480 及切換尺寸的回歸。

## S1–S6 完成度與舊功能回歸

| 範圍 | 本輪判定／證據 |
| --- | --- |
| S1 系列 | 交易批次缺少 ID 回滾、來源保留、聚合與三檢視程式及測試有支持；10,000 本為資料／mock 量測，非本機／UNC I/O 驗收 |
| S2 系列詳情／下一集 | 數字／全形、歧義／跳號／離線拒絕任意選書，以及保存／開書失敗保留 reader 有支持；返回位置有 T3 缺口 |
| S3 書籍詳情 | 共用詳情、收藏、編輯／管理與窄窗入口有靜態及 browser 證據；未完成 Windows 原生操作矩陣 |
| S4 書籤／筆記 | Rust CRUD、200 上限、ID 重映射、已有來源略過、SQL 還原回滾及依頁名跳轉有支持；實際 Vue mock 的純文字與缺頁流程通過，非 Windows 原生 IPC 驗收 |
| S5 安全備份 | v3／舊 JSON、唯讀預覽、重新驗證、create_new、未知檔案保留及清理失敗有支持；Windows 升級受 T1 阻擋，登記失敗有 T2 缺口 |
| S6 風格／閱讀器 | 型別／build、三支 browser 檢查及目前部分截圖核對有支持；窄窗返回有 T3。原生 IME／觸控、高 DPI、輔助科技及正確原始參考 PNG 的獨立對照仍未完成 |
| A／R1／R2 | 本輪 Windows 測試涵蓋壓縮名稱、選取 ID／多來源衝突、一般／verbatim、離線恢復、收藏成功／失敗／跨書保護，未發現新退化 |
| N1／N2、F1–F5 | 自動／手動狀態、雙頁末組、來源改動、metadata／標籤、匯入互斥及失效來源的既有測試仍通過；未要求或宣稱 v2 全庫回填 |
| F6 相容性 | JSON 合併／回滾未發現新退化；既有 DB 的 Windows 升級因 T1 未達可用，不能以整體相容通過收尾 |

## 本輪實際驗證

| 執行 | 本輪結果 |
| --- | --- |
| 固定 diff／`git diff --check e38f7a8 ad0e366` | 通過；後續 8 檔僅文件／證據 metadata |
| `npm ci --ignore-scripts --no-audit --no-fund` | 最終隔離 UI 副本依 lockfile 安裝成功，84 packages |
| `npm test` | **21／21 通過**，退出碼 0 |
| `npm run build` | **通過**，vue-tsc／Vite，退出碼 0 |
| `cargo test --locked --offline --manifest-path src-tauri/Cargo.toml` | 固定原始產品 **43 通過／2 失敗**，共 45；退出碼 1。比 Linux 44 多一項 Windows cfg，失敗詳見 T1 |
| 追加隔離 Rust manifest 重現 | **1／1 缺陷重現成功**，退出碼 0；不併入產品原始案例數 |
| 三支既有 browser 腳本 | `validate-next-phase.py`、`validate-third-phase.py`、`validate-immersive-reader.py` 均退出碼 0；只把固定副本連線改為 localhost:1435，IPC 為 mock |
| 追加實際 Vue 返回位置重現 | **2 情境缺陷重現成功**；100 本／窄窗，pageErrors=[] |
| Windows 唯讀／可寫同步 probe | 唯讀 Err(os error 5)、可寫 Ok；僅操作臨時檔案 |

首次借用舊版 node_modules 不含本次新增的 icon 依賴，build 失敗；首次 offline 安裝無完整 cache，沙箱內 online 安裝也失敗。這些不列為產品缺陷。最終建立可供核准安裝程序存取的獨立副本，以 lockfile 安裝成功後重跑 npm test／build 及 browser，以上表為最終結果。Rust 快取複製至新的獨立 target，未修改舊審查產物。

本機證據留存（不屬產品設定）：

- Rust／同步 probe 副本：`C:/Users/Racious/AppData/Local/Temp/mangafolio-third-review-ad0e366-20261004-bcuetgle/`；獨立 manifest 測試只追加於其 source，原始 45 項先執行再追加。
- UI 副本：`C:/Users/Racious/AppData/Local/Temp/mangafolio-third-ui-review-20261004-fzf4lm9p/`；含 `npm-test.log`、`npm-build.log`、三支 `validate-*.log`、`browser-checks.json`、`browser-probes.py`／`browser-probes.json`，完整 browser 產物位於該副本內。

## 限制與接手

- 未啟動正式 identifier 的 Tauri 或碰觸 `%APPDATA%/com.racious.mangafolio/library.sqlite3`；未完成 VM／專用測試使用者的 Windows 原生清單、選檔、UNC、ACL、真正 ENOSPC、原生 IME／觸控、高 DPI、螢幕閱讀器、打包及升級安裝。
- Linux 原生畫面／SQL 與原圖比對沿用開發者明確有限的證據；本輪未獨立重跑 Linux native，也未取得三張正確參考原始 PNG。不能因此宣告視覺完全一致或可及性合規。
- 本輪結論限固定產品；尚無修正完成、複審通過或發布可用宣稱。主導方應逐項評估 T1–T3，修正後補聚焦證據並複審。交辦未指定 resolution 路徑，審查方不代填採納／異議或修復結果。