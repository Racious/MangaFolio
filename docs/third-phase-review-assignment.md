# 第三階段獨立審查與隔離驗收交辦

最新面板修正 **T2-UI1**：產品 `b1a02dd65d80cb85c6b633ce65d197ac8ca4fb3f`，詳見 [本次交辦／完整驗證](third-phase-t2-ui1-handoff.md)。本次 Linux npm21／Rust48、build／diff、八項真實元件情境與兩支整合 script 通過；最新 Windows／原生流程未執行。前輪 0b883c8 的独立 Windows49 通過只涵蓋前輪；以下保留歷史交辦與證據。

最新狀態：45568a8 已完成獨立 Windows47／npm21 及四支 browser 複審；T1／T3 通過，剩餘 T2-R1 現已補強，最新產品 `0b883c85e77ea8046fa851464c80a86fbbb7c513`。請依 [T2-R1 交辦](third-phase-t2-r1-handoff.md) 核對聚焦、累積與完整範圍，新結果另寫 `docs/third-phase-t2-r1-review-report.md`。本次 Linux48／npm21、build／diff 通過，最新 Windows／原生驗收尚未執行；下方為前輪歷史交辦，不作為最新驗證。

日期：2026-10-04 Asia/Tokyo。Repository：Racious/MangaFolio。分支：`feature/series-library-reading`。

固定開發基準：`e38f7a84c33126e11a141d10c0f38f4defb8985f`。
最終產品提交：`ad0e366cb0d7c4b583968243f8d864d58fce0d3e`。本文件的後續提交只固定 SHA／驗證對應，不另更改產品；以 Git 記錄辨識文件提交。
基準已含 A／R1／R2、N1／N2、UI 精修 e2a87d6 與主線 merge；舊審查不能涵蓋本次程式。

## 最新 T1／T2／T3 修正（待獨立複審）

原產品 ad0e366 與下方 npm21／Rust44 為首輪歷史資料。兩份独立報告已確認 T1–T3；本次修正產品 `45568a804b1228c9016f9a7fd2bd876218673b2e`，Linux npm21／Rust46、build、diff check 與四支 browser 回歸通過，最新 Windows 尚未執行。請優先依 [修正複審交辦](third-phase-review-fix-handoff.md) 審查聚焦與完整範圍；新結果寫入 `docs/third-phase-review-fix-report.md`，既有兩份報告保留不改。

## 審查要求

閱讀本文件、`third-phase-plan.md`、`third-phase-validation.md`、`third-phase-visual-validation.md`、`library-guide.md`、`handoff.md`、原始 `review-report.md`、`review-fix-handoff.md`、`next-phase-review-fix-handoff.md` 與 `next-phase-review-fix-report.md`。審查固定基準至最終產品的實際 diff、測試及產品流程，不只依交辦文字判定。

先判定 S1–S6，再檢查 A／R1／R2、N1／N2 與既有 F1–F6 是否退化。首輪結果已寫入 `docs/third-phase-review-report.md`，本次新結果依上方修正交辦另寫，舊報告保留不改。每個問題提供檔案位置、觸發條件、影響、證據及建議修法。未另行授權，不自行修改、提交、推送、合併或發布。

## 功能與主要檔案

| 需求 | 行為／修改位置 | 覆蓋 |
| --- | --- | --- |
| S1 | `library_reading.rs` 交易批次系列；`series.ts` 聚合；`SeriesShelf.vue`／`LibraryView.vue` 單層書架、改名／移除歸屬、冊數、已讀與頁面進度分開、來源失效 | 缺少 ID 整批回滾、來源保留；10,000 本量測，初始 60 個系列封面 |
| S2 | 共用三檢視，首頁與系列各自搜尋／排序／位置；純數字唯一且相鄰集數才給下一集；`ReaderActions.vue` 最後跨頁組入口 | 重複、缺值、全形、特殊、跳號、尾集、離線；save／load 失敗保留 reader |
| S3 | `BookDetailsPanel.vue` 集中封面、資訊、來源、收藏、編輯／管理、下一集與書籤 | 三風格側欄、窄窗 dialog、既有資訊編輯與收藏同步 |
| S4 | `library_reading.rs` CRUD；`commands.rs` 依頁名驗證後開書；`BookmarksPanel.vue` 純文字名稱／筆記、雙頁明確指定頁；reader store 共用切換保護 | 上限 200、名稱 80、筆記 2000、還原 ID 重映射、來源略過、缺頁不替換 session、SQL 回滾、HTML 不執行 |
| S5 | `library_safety.rs` 升級前 SQLite online backup、預覽、保留清理；`library.rs` JSON v3／create_new；`BackupPanel.vue`／backup store 自動／手動／失敗狀態 | WAL、升級失敗、SQL DDL 回滾、未知檔案不清理、阻擋目錄 I/O 失敗、預覽唯讀、還原重新驗證 |
| S6 | `themes.css` 語意 token、共用 BookCover／ContinueReading／系列／詳情；`ImmersiveReader.vue` 覆蓋工具列、保存固定設定、Teleport 書籤 | 三風格三檢視、明暗、IME／鍵盤／觸控 browser、窄視窗、錯誤常駐、原生圖片滿高 |

其餘修改：`lib.rs` IPC 註冊、`Cargo.toml` 啟用 rusqlite backup feature；`api/library.ts`／`backend.ts` 型別；`App.vue` 自動檢查／關閉保護；`BookCard.vue` 詳情入口；`LibraryManager.vue` 批次系列與安全備份面板；`AppearanceSettings.vue`／`appearance.ts` 固定工具列偏好；library／reader store；`LibraryGuide.vue` 教學；tests 與三支 validate scripts。無新增外部主題、來源掃描、帳號或發布功能。

## 資料、升級及失敗處理

SQLite v3 增加 bookmarks、backup_settings 與 automatic_backups。本機偏好與清理 manifest 不納入 JSON。既有 v1／v2 升級前先一致性保存全部 SQLite 資料（含 WAL）；失敗中止升級，安全快照不隨保留政策刪除。migration SQL 失敗整批回滾。v3 既有自動閱讀狀態不全面回填。

JSON v3 含既有書籍資訊、標籤、收藏、進度、偏好、書籤／筆記；讀取 v1／v2、拒絕較新版與無效關聯，不部分還原。新來源重新映射 ID 與書籤；已有來源连同書籤略過，不覆寫現有資料。預覽不寫書庫；確認時重新讀取檔案並驗證。沒有漫畫來源、封面、外觀設定或本機備份偏好。

自動預設關閉、保留 1–20 份；啟動及每 15 分鐘檢查、24 小時成功間隔（手動 JSON 成功也計入最近備份），可立即備份。僅清理本程式登記、嚴格名稱、有效 JSON 的 regular file；未知／未登記／symlink／升級快照保留。JSON 匯出 create_new，不覆寫已有備份或來源。清理失敗另明確回報「備份已建立，但保留清理失敗」，不宣稱已清理的舊檔還在；新檔及剩餘檔保留。請特別審查 I/O 失敗、manifest 失敗、清理失敗與 last_success／last_error 語意。

## 首輪 Linux 歷史驗證

npm 21／Rust 44、build、diff check，以及 browser 既有功能／第三階段／沉浸閱讀測試，結果与完整輸出見 [本次驗證](third-phase-validation.md)。歷史 npm16／Rust37 不作為本次證據。Linux 不執行 Windows cfg 測試，舊 Windows 通過不能涵蓋本次。

Linux Tauri 的畫面、書籤寫入、滿高閱讀、系列詳情為有限原生證據；browser mock 為實際 Vue 和 mock IPC，不是 Windows 實測。PNG 原始參考歸檔受限，已直接比較對話中正確三張圖，不能將舊圖代入。

## Windows 隔離實機驗收清單

使用 VM／專用測試使用者，確認 app_data_dir 為測試位置。不得接觸既有 `%APPDATA%\com.racious.mangafolio\library.sqlite3`。先備妥隔離 v1／v2／v3 DB、ZIP／CBZ／圖片資料夾、一般／verbatim 磁碟／UNC 路徑與失效來源。

1. 啟動有 WAL 未 checkpoint 的 v2，確認先產生完整升級快照，再升 v3；注入權限／磁碟與 SQL 失敗，確認 schema、原資料與成功備份保留。未知新 schema 拒絕。
2. 建立 10／6／8／4 冊四套系列，批次指定、改名、移除歸屬；缺少 ID 整批失敗，原書籍／來源不刪不移；無系列仍在書庫，標籤與收藏概念分開。
3. 系列內搜尋，三檢視與三風格／明暗／密度／尺寸互換，確認選取、篩選、排序、續讀不變；返回書架／reader 返回恢复搜尋排序位置。重啟確認外觀及固定工具列保存。
4. 測試數字／全形／重複／缺值／特殊／跳號集數與最後一集；最後跨頁組下一集、失效來源、保存失敗、開書衝突均不得任意換書或換 ID。
5. 單／雙頁新增、編輯與刪除書籤，確認明確指定頁；插入前頁後依頁名跳轉；刪除標記頁後拒絕不使用舊索引；改來源保持 ID；HTML 筆記純文字、200／80／2000 限制。
6. 新舊 JSON 預覽不得寫庫；更換預覽檔後還原重新驗證；新增、既有略過、來源衝突、未知版本、無效關聯／SQL 失敗完整回滾。書籤映射新 ID、既有來源書籤不覆寫。
7. 自動備份開關／保留數／重啟、24h 間隔、立即備份、權限／磁碟失敗、最近成功及錯誤；未知文件、未登記同名規則文件、symlink與升級快照不清理；已有輸出拒絕覆寫。
8. 阅读工具列預設隱藏，hover、Esc、Tab、觸控與固定顯示，圖片尺寸不變；對話框可用、關閉後鍵盤翻頁正常、重要錯誤一直可見；中文原生 IME 組字、長標籤書名、640×480、高 DPI、螢幕閱讀器。
9. 實際比較三張正確參考與全部配套畫面，標明素材／資訊密度差異；補齊 references 原圖歸檔。不得把 mock 或設計稿當成原生驗收。
10. 回歸 A 壓縮書名與重加狀態、R1 選取 ID／多筆別名衝突不寫庫、R2 成功收藏同步與失敗／跨書保护；N1／N2 含陳舊 v2 自動狀態的 DB／JSON、重新開書／保存／重加／relink／「依進度判定」，手動狀態始終保留。
11. 大型本機／UNC 書庫實测來源檢查、封面與搜尋，不以本次 browser 聚合量測代替 I/O 效能。

## 已知界線

未完成獨立 Code Review／完整 Windows 驗收，未發布；Linux PRoot GTK 選檔限制，完整原生 import／relink／手動匯出／還原對話框未宣告通過。真正 ENOSPC／Windows ACL／實體觸控／原生 IME／高 DPI／輔助科技仍待驗證。v2 陳舊自動狀態保留原既有契約，不做全庫遷移；非數字／歧義集數只允許手動選書。自動備份需程式開啟；不排程關閉狀態的背景服務。參考原 PNG 無可下載 file ID，歸檔屬外部阻礙。
