# 第三階段實際驗證

日期：2026-10-04 Asia/Tokyo；隔離 Linux 環境，固定基準 `e38f7a84c33126e11a141d10c0f38f4defb8985f`，對應產品 `PRODUCT_SHA_PENDING`。後續只補文件 SHA；機器結果 [checks.json](third-phase-results/checks.json)。不使用舊測試紀錄代替本次。

## 必要命令（本次執行）

| 命令 | 結果／退出碼 | 完整原始輸出 |
| --- | --- | --- |
| `npm test` | 21 通過，0 | [npm-test.log](third-phase-results/npm-test.log) |
| `npm run build` | Vue 型別檢查與 Vite build 通過，0 | [npm-build.log](third-phase-results/npm-build.log) |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | 44 通過，0 | [cargo-test.log](third-phase-results/cargo-test.log) |
| `git diff --check` | 通過，0 | [git-diff-check.log](third-phase-results/git-diff-check.log) |

環境：Node 22、Rust 1.95、Linux GTK／WebKitGTK sysroot；`CARGO_TARGET_DIR=/workspace/MangaFolio/src-tauri/target`。沒有變更 Cargo.lock 的 dependency resolution，使用 --locked。原始 logs 有輸出格式空白，`.gitattributes` 僅对 logs 關閉尾端空白檢查以原樣保留證據；程式與文件仍檢查。所有單元測試有隔離臨時來源／DB，不操作實際使用者庫。

## 測試覆蓋

- 原 A／R1／R2、N1／N2：壓縮書名、全形搜尋、重新加入資訊／狀態、所選 ID／來源別名拒絕、session 不污染、收藏成功同步及失敗／跨書保護仍保留并通過。
- Rust 新增 7 個測試：系列批次缺失 ID 回滾；書籤 CRUD／200 上限／重映射新 ID／既有來源略過／刪書 cascade；預覽唯讀與還原驗證／SQL 回滾；自動備份保留規則、I/O 失敗及成功後清理 bookkeeping 失敗；升級前未 checkpoint WAL 一致性與備份失敗保留 v2；migration DDL 途中失敗原 v2／表資料保留；書籤按頁名找新索引及缺頁後書库與 reader session 不變。
- npm：系列手動已讀與頁面進度不同、數字／全形／特殊／重複／缺值／跳號／尾集／離線；書籤定位；固定工具列偏好損壞／缺值；實際 reader store 保存／開書／書籤失敗保留目前狀態。
- 舊 v1 schema／JSON 及 v2 JSON 測試仍通過，未做既有 v2 陳舊自動狀態全庫回填。

## Browser 整合（實際 Vue＋隔離 mock IPC）

| 執行 | 結果 | 輸出 |
| --- | --- | --- |
| `scripts/validate-next-phase.py` | 既有完整功能、18 組風格／主題／檢視、資訊／標籤／匯入／回滾與對比，0 | [browser-regression.log](third-phase-results/browser-regression.log)、[results](third-phase-results/regression/results.json) |
| `scripts/validate-third-phase.py` | 三風格配套畫面、離線／上線後詳情封面更新、系列搜尋／三檢視／返回狀態、批次失敗、唯讀預覽、書籤純文字／缺頁、固定、下一集失敗／成功、切書中原生 close-request 回呼保護及窄窗，0 | [browser-third-phase.log](third-phase-results/browser-third-phase.log)、[完整三風格圖庫驗證](third-phase-results/browser-gallery.log)、[results](third-phase-results/browser/results.json) |
| `scripts/validate-immersive-reader.py` | 預設滿高／hover／Esc／IME／焦點／觸控／錯誤常駐／關閉書籤 dialog 不擋方向鍵，0 | [reader-regression.log](third-phase-results/reader-regression.log)、[results](third-phase-results/reader/immersive-reader.json) |

pageErrors 都為空。10,000 本／1,000 系列聚合最後量測約 15.0 ms，初始渲染 60 個系列封面；單次 browser 本地量測，不代表 Windows／UNC 磁碟與來源 I/O。既有 10,000 本搜尋／封面佇列量測見 regression results。圖片為明確標記的隔離來源，不是設計示意圖冒充成品。

## 有限 Linux 原生實測

使用當前後端 `cargo build --locked --manifest-path src-tauri/Cargo.toml`（0），搭配當前 Vite 前端、Xvfb／PRoot 真實 Tauri WebKit／SQLite／圖片解碼；非 packaged release。最終原生重啟曾因 WebKit 子程序未啟動而空白，在隔離 PRoot 測試程序使用 `WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1`／`WEBKIT_DISABLE_DMABUF_RENDERER=1` 後確認 WebKit 子程序啟動及畫面；這些只設於測試 shell，未加入產品設定。輸出 [native-build.log](third-phase-results/native-build.log)、[native.log](third-phase-results/native.log)，[畫面](images/third-phase/native/)。

只使用 `/tmp/mangafolio-third-phase-native/{data,config,cache,sources}`，28 本／4 系列／1 離線來源，純隔離生成圖片。原生已確認立即本機安全備份產生 v3 JSON（28 本／2 書籤）、系列詳情、書籍側欄、閱讀圖片滿高及工具列 hover 不改畫布、書籤新增寫入 SQLite、最後頁下一集切換。讀取隔離 SQL 確認 book 3 保存 `119 / 120.png`，book 4 已開啟且進度 `0 / 001.png`；新書籤 book 3、082.png／81 已保存。原生輸入自動化有字元事件延遲，筆記寫入內容依 native-results 精確記錄，不把自動化排程當成中文 IME 驗收。

[原生保存 artifacts 檢查](third-phase-results/native-artifacts.log)（退出碼 0；script 僅讀固定隔離路徑 DB，mode=ro）與 [精確 SQL 結果](third-phase-results/native-results.json)。原生流程用來補證 IPC／SQLite／影像，不宣告全部需求通過；畫面對照見 [視覺驗證](third-phase-visual-validation.md)。

## 失敗與修正紀錄

[attempts](third-phase-results/attempts/) 保留本次早期完整輸出。Rust 初次編譯／版本斷言失敗後修正 rusqlite API、fixture、新版拒絕測試；browser 初期精確文字／select 名稱 mock 對應修正。整合實際發現書籤 dialog 受工具列隱藏狀態影響，改 Teleport 到 body；閉合 dialog 阻擋 Esc／翻頁改為只檢查真正開啟的 dialog，新增回歸。本次最終命令重新執行全部通過，未用歷史結果替代。

## 未執行與已知限制

- Windows cfg 專屬測試、Windows WebView2／UNC／ACL、原生 IME、高 DPI、螢幕閱讀器、實體觸控及完整獨立 Code Review 未執行。
- PRoot GTK 原生選檔環境限制，完整原生匯入／relink／手動匯出／還原選檔流程未完成；browser mock 與 Rust 函數測試不能代替原生驗收。
- 阻擋備份路徑造成 I/O 失敗已測；真實磁碟耗盡／權限／突然斷電未實測。schema rollback、SQLite WAL、JSON validation 不等於完整災難復原演練。
- JSON 保留 16 MiB／10,000 本上限，大量長筆記也計入大小，超限明確拒絕，不部分匯出。自動備份需程式開啟，24h 間隔包含最近手動成功備份。
- 非數字／重複／缺少／跳號集數只允許手動選書；v2 陳舊自動狀態維持既有更新時機，詳見教學與交辦。
- 三張正確原圖已直接檢視，但雲端未取得可下載原始位元組，正式 references PNG 歸檔屬外部阻礙；未用舊圖或生成圖替代。

完整 Windows 隔離步驟與獨立報告位置見 [第三階段交辦](third-phase-review-assignment.md)。未宣告正式發布或完整可及性合規。
