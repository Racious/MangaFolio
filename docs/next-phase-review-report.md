---
title: 下一階段獨立程式碼審查報告
type: review
date: 2026-10-03
reviewer: 天城 / Codex
---

# 下一階段獨立程式碼審查報告

## 結論

確認 **2 項 P2（中度）問題**，均影響 F1 自動閱讀狀態。現有測試通過，但不足以支持 F1 已完整完成的宣稱；應修正後複審。沒有確認新的 P0／P1 問題。既有 A／R1／R2 在本輪程式檢視及隔離測試中未發現退化。

本報告是固定產品範圍的獨立審查，不代表 Windows 原生完整驗收或發布核可。只新增本報告，未修改產品、提交或推送。

## 範圍與方法

| 項目 | 核對結果 |
| --- | --- |
| 分支 | `feature/library-management-ui` |
| 基準 | `bac9674f3b0d2a5c2e19774d5294d7cbfd2f86b8` |
| 產品 head | `08ccc3e447372ade1834e8d3d63cabc8df23c5b0` |
| 審查時本機 HEAD | `3a3dbc85829c5057db231ac48831acf672bded9e`；產品 head 之後為文件／證據補齊，未以本機 HEAD 擴大產品範圍 |
| 固定差異 | 84 個檔案，包含產品、測試、腳本、文件及圖片 |
| 文件 | 閱讀交辦單、原始報告、修正交辦及 r2 報告、下一階段計畫、驗證與設計 QA 紀錄 |
| 程式 | 核對 SQLite／migration／restore／metadata／IPC、匯入及書庫 store、reader 契約、管理／編輯／標籤 UI、外觀及教學、測試與驗證腳本 |
| 測試隔離 | 由固定產品 head 的 Git archive 建立系統暫存目錄副本；實際檔案來源與 SQLite 均為臨時資料 |
| 環境 | Windows x64（10.0.26200）、Node 22.22.3、npm 10.9.8、Rust／Cargo 1.95.0；瀏覽器驗證使用 Edge、Playwright 1.63.0 |

本機既有 `src-tauri/Cargo.toml` 修改，以及 `AGENTS.md`、`CLAUDE.md`、`CLAUDE-CODE-FEEDBACK.md`、Android／iOS icons 未追蹤檔，不納入固定範圍，亦未更動。未接觸使用者實際 APPDATA 書庫。

## 發現

### N1 — [P2] 雙頁最後一組無法自動標為已讀

- **位置**：`src-tauri/src/library.rs:389–392`，尤其第 391 行；相同判定在 `src-tauri/src/library_metadata.rs:94`。相關既有前端契約為 `src/stores/reader.ts:96–104,189–202`。
- **觸發條件**：自動狀態、6 頁書籍、雙頁模式且不獨立封面，翻到最後一組第 5／6 頁，保存進度。此時 reader 的 `atLast=true`，`viewIndices=[4,5]`，但保存的 `index=4` 是跨頁起始索引。
- **原因**：新後端只以 `index >= page_count - 1` 判定已讀，沒有考慮雙頁最後一組。因此比較 `4 >= 5` 得到 false，儘管讀者已看見最後頁。重新選「依進度判定」也沿用同一條件，無法修正。
- **影響**：正常讀完仍顯示「閱讀中」，不會出現在已讀篩選，且仍可能被選為繼續閱讀。其他最後一組為雙頁的頁數／封面設定也受影響；最後一組為單頁者不受此觸發影響。
- **獨立重現**：在暫存副本載入實際 reader store，僅 mock IPC，取得以下快照；再用 6 張真實臨時 PNG 與實際 Rust／SQLite 保存相同索引及偏好，確認狀態為 `reading`。

```text
atLast=true, viewIndices=[4,5]
snapshot.index=4, preferences.pageMode=double, doubleCover=false
CONFIRMED double final spread [4,5], pageCount=6, saved index=4, status=reading
```

- **建議修法**：統一完成判定，讓後端依頁數、索引與已保存的閱讀偏好計算目前跨頁是否包含最後頁，或使用明確且可驗證的末頁訊號。保存進度與恢復自動狀態應使用一致規則。保留續讀的起始索引契約，不宜直接把所有最後跨頁改存最後頁索引。補單／雙頁、封面獨立開關、奇／偶頁數及手動狀態不覆寫的案例。

### N2 — [P2] 重新加入／指定來源後，自動狀態未依新頁數更新

- **位置**：`src-tauri/src/library.rs:334`、`350`（重新加入更新頁數）及 `554`（relink 更新頁數／有效續讀索引）。
- **觸發條件與獨立重現**：使用真實臨時圖片資料夾、實際 library API 與 SQLite，完成以下操作；所有階段 `statusManual=false`。
  1. 加入 3 頁書並保存最後頁索引 2，自動狀態為 `read`。
  2. 來源新增第 4 頁後重新加入，頁數更新為 4，續讀仍為索引 2，但狀態仍是 `read`。
  3. 呼叫「依進度判定」使其恢復 `reading`，再 relink 至含相同原 3 頁的資料夾；有效續讀成為第 3／3 頁，狀態卻仍是 `reading`。

```text
CONFIRMED reimport: manual=false page=3/4 status=read
CONFIRMED relink: manual=false page=3/3 status=reading
```

- **原因**：這些寫入更新 `page_count`／`last_index`，卻完整保留先前 `reading_status`，沒有區分手動標記與自動推導值。保存 metadata 不應讓自動狀態失去與目前進度的一致性。
- **影響**：新增未讀頁後仍被已讀篩選收錄，繼續閱讀候選會略過該書；較短來源最後頁則可能仍顯示閱讀中。必須再保存進度或手動重設自動判定才能修復，重新加入／relink 結果本身沒有修復。
- **建議修法**：在同一更新交易內，以更新後有效頁數／續讀索引重新推導 `statusManual=false` 的狀態，並與 N1 使用一致的完成判定；`statusManual=true` 保留指定狀態。補來源擴頁、縮頁及頁名重新定位的測試，同時驗證 ID／收藏／偏好／自訂資訊／標籤未遺失。

## 完成度稽核

以下「未發現缺口」限定於本輪程式檢視與已執行測試，不替代原生驗收。

| 宣稱範圍 | 審查結果與證據 |
| --- | --- |
| A 壓縮來源名稱／自訂名 | 未發現退化；Rust 測試及來源名／自訂名分離、重加保留資訊的實作支持原修正 |
| R1 來源鍵與選取 ID | 未發現退化；保留 ID、別名衝突與交易路徑；Windows normal／verbatim 路徑測試實際通過 |
| R2 收藏同步 | 未發現退化；成功後核對 reader ID，失敗及切書測試通過；批次操作同樣核對保留 ID |
| F1 狀態 | **未完整完成**：手動狀態及不清續讀有證據；自動完成判定存在 N1／N2 |
| F2 標籤 | 未發現缺口；名稱／限量／重名驗證、FK、CRUD、批次交易與 UI mock 通過 |
| F3 資訊 | 未發現缺口；長度驗證、搜尋、顯示名／來源名分離、保存失敗可重試、重加／relink 保留欄位有證據 |
| F4 匯入 | 未發現缺口；順序匯入、取消保留當前結果、失敗繼續、指定失敗項重試及 busy 防護有程式／測試證據；原生選檔待驗 |
| F5 失效來源 | 未發現新增來源識別缺口；missing 篩選與 relink UI 有證據，ID／資訊保留；relink 自動狀態受 N2 影響 |
| F6 相容性 | 未發現缺口；真實 v1 形狀、migration 失敗回滾、未來版本拒絕、v2 catalog／名稱映射、skip 不覆寫、restore SQL 失敗回滾及 create_new 有測試／實作證據 |
| U1–U2 現況與方向 | 文件提供現況、三方向及隔離來源說明；本輪未重做歷史基準擷取。交辦／計畫沒有逐項定義 U1、U2 編號，依所列現況與視覺方向核對，不擴張完成宣稱 |
| U3–U5 外觀與共用檢視 | registry／語意 token／共用業務邏輯支持；三風格 × 明暗 × 三檢視共 18 組 mock 通過，設定重新載入與容錯測試通過 |
| U6–U8 窄窗、狀態、教學 | 640×480 控制可達與基本焦點、無結果／空庫／失敗／進度、自願教學 mock 通過；人工檢視本輪窄窗截圖。螢幕閱讀器、高 DPI、原生 IME 尚不能宣告完成驗收 |

## 實際執行與結果

所有建置／腳本與額外重現案例均在固定 head 的暫存副本執行。額外案例沒有寫回產品測試檔。

| 項目 | 結果 |
| --- | --- |
| `npm ci --ignore-scripts --no-audit --no-fund` | 成功，83 packages；使用獨立暫存 npm cache |
| `npm test` | **15／15 通過** |
| `npm run build` | **通過**，vue-tsc 與 Vite production build，74 modules |
| `cargo test --locked --offline --manifest-path src-tauri/Cargo.toml` | 固定產品原始測試 **35／35 通過**；比交辦 Linux 34 項多出 Windows cfg 案例 |
| `git diff --check <base> <product-head>` | 通過，退出碼 0 |
| `scripts/validate-next-phase.py` | 通過，11 組檢查、`pageErrors=[]`；包含 18 外觀組合、36 語意 token 對比、10,000 筆 mock 首 60 筆、窄窗、編輯失敗重試、匯入與教學 |
| `scripts/smoke-library-management.py` | 通過；選取／批次收藏、移除取消與失敗、進度 flush／discard、relink、備份還原及窄窗 mock 流程 |
| N1 追加重現 | 實際 reader store 快照 + 額外 Rust 測試，確認最後跨頁仍為 `reading`；測試退出碼 0 表示成功確認缺陷 |
| N2 追加重現 | 額外 Rust 測試確認重新加入及 relink 狀態陳舊；退出碼 0 表示成功確認缺陷 |

兩項追加重現不併入產品原始 35 項通過數。瀏覽器腳本首次因 localhost IPv4／IPv6 綁定位址不同而連線失敗；改為 `127.0.0.1` 綁定後兩套腳本成功。完成後已停止測試 Vite 伺服器。

## 未執行與邊界

- 未啟動 Windows 原生 Tauri UI，未執行完整隔離 VM／測試帳戶矩陣：原生選檔、dialog／IPC 的實際整合、正常關閉立即重啟、高 DPI、螢幕閱讀器、真正中文 IME 及系統明暗變更仍待實機驗收。
- Rust 測試使用真實臨時檔案及 SQLite，可以支持 Windows 後端行為；瀏覽器腳本使用 mock IPC，只能支持 UI 狀態與前端整合，不替代原生資料操作。
- 10,000 本瀏覽器案例是純 mock 資料，不代表 10,000 個真實來源、封面解碼或 UNC 可用性同步檢查的效能。UNC 斷線／恢復及網路大庫延遲未量測。
- 已知 schema 無降級、備份大小／筆數上限、Unicode lowercase 非 NFKC、取消不中斷當前解析及 localStorage 不隨書庫備份，依現有契約評估，未列為新缺陷。
- 未進行安裝升級、簽章、打包或發版驗證。

## 後續建議

先修正 N1／N2 並補涵蓋雙頁末組與來源頁數變更的回歸測試，再對修正範圍複審。其後仍需依交辦單完成隔離 Windows 原生矩陣，才有足夠證據宣告完整實機驗收。
