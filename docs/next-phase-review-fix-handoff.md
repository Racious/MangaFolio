# N1／N2 修正複審交辦

日期：2026-10-03（Asia/Tokyo）。分支：`feature/library-management-ui`。
原獨立報告：[next-phase-review-report.md](next-phase-review-report.md)，保持原文不改。

| 提交 | 固定 SHA |
| --- | --- |
| 下一階段原始開發基準 | `bac9674f3b0d2a5c2e19774d5294d7cbfd2f86b8` |
| 首輪產品 | `08ccc3e447372ade1834e8d3d63cabc8df23c5b0` |
| 本次修正基準／審查報告提交 | `1394670f3b4ac63ab69fcedcabb650a2b0e97879` |
| 本次最新產品 | `10af40e99c703c31332e00f5ab5bcd3c7ebe2ccc` |

複審差異固定為 `1394670f3b4ac63ab69fcedcabb650a2b0e97879` → `10af40e99c703c31332e00f5ab5bcd3c7ebe2ccc`。整體下一階段範圍固定為 `bac9674f3b0d2a5c2e19774d5294d7cbfd2f86b8` → `10af40e99c703c31332e00f5ab5bcd3c7ebe2ccc`。必須先判定聚焦修正範圍的 N1／N2，再核對整體範圍 F1–F6、UI（U1–U8）及 A／R1／R2 是否退化，分別提供判定與證據。後續提交僅更新交辦／驗證／教學文件，不改產品程式；文件提交用 `git log -1 -- docs/next-phase-review-fix-handoff.md` 核對。

## 修正內容

- `src-tauri/src/library_metadata.rs`：共用 `automatic_status` 依單／雙頁、獨立封面、頁數與索引計算目前視圖是否含末頁，對齊 reader 的 pairStartOf／indicesOf。無閱讀時間仍為未讀；不改跨頁起始索引。
- N1：`library.rs::save_progress` 在交易內核對有效頁碼，以共用規則保存自動狀態，手動狀態不覆寫。「依進度判定」也使用同一規則。v1 schema 升級及 v1 備份預設狀態同步使用，避免舊資料雙頁末組仍套單頁判定。
- N2：普通／別名重新加入與 relink 在原有交易內，依頁名重新定位、缺頁時限制索引，再更新自動狀態；手動已讀／未讀保留。ID、收藏、閱讀時間、偏好、自訂資訊、標籤均保留。失敗整筆回滾，不替換目前 reader。
- `tests/favorite-store.test.mjs`：實際 reader store 跨頁末組與 progressSnapshot 契約回歸。只 mock IPC，不宣稱原生驗收。
- `library.rs` 測試：1–8 頁、單／雙頁、獨立封面開關、每個有效索引、自動重設、手動狀態；真實 PNG 來源擴頁／縮頁／頁名定位、重新加入及 relink；注入 SQL 失敗整筆回滾；舊 schema／v1 備份末組。A／R1／R2 既有測試仍通過。

沒有修改前端翻頁、儲存索引或來源識別契約，沒有 schema 升版、全庫回填、路徑遷移或自動合併重複紀錄。

## 各輪驗證邊界

- 首輪獨立 Windows 審查：npm 15／Rust 35，固定產品 08ccc3e；見原報告。不是最新修正的 Windows 結果，也不是 Windows 原生完整驗收。
- N1／N2 修正後 Linux：npm 16／Rust 37，固定最新產品 `10af40e99c703c31332e00f5ab5bcd3c7ebe2ccc`，詳列如下。最新修正仍待獨立複審與 Windows 驗證。
- 首輪 Linux 歷史結果：npm 15／Rust 34，當時 Windows cfg 未執行；保留原始紀錄，不代替最新驗證。

## 本次修正後 Linux 實際驗證

| 命令 | 結果 | 完整原始輸出 |
| --- | --- | --- |
| `npm test` | exit0，16 passed | [npm-test.log](next-phase-results/review-fix-n1-n2/npm-test.log) |
| `npm run build` | exit0 | [npm-build.log](next-phase-results/review-fix-n1-n2/npm-build.log) |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | exit0，37 passed；main／doc 零案例 | [cargo-test.log](next-phase-results/review-fix-n1-n2/cargo-test.log) |
| `git diff --check` | exit0 | [diff-check.log](next-phase-results/review-fix-n1-n2/diff-check.log) |

UTC 執行時間、命令與退出碼：[exit-codes.json](next-phase-results/review-fix-n1-n2/exit-codes.json)。本次輸出獨立保存，不覆寫或借用首輪／獨立審查結果。所有新來源與 SQLite 用臨時隔離目錄，未操作使用者實際 APPDATA 書庫。

## 複審與實機待驗

請同時閱讀原審查報告與實際程式差異／測試，特別核對 N1／N2 與 A／R1／R2 是否退化。結果寫入 `docs/next-phase-review-fix-report.md`，提供位置、觸發條件、影響與建議修法。未經另外授權，不自行修改、提交或推送。

本輪 Linux 未執行 Windows cfg 測試、原生 Tauri UI 或新增原生畫面擷取；原審查 Windows35項通過不是本次 Windows 驗證。仍需隔離 Windows 實機確認雙頁末組（奇偶頁數及獨立封面）、已讀篩選／續讀候選、重新加入增減頁、relink 與手動狀態，再依原交辦完整矩陣驗收。未以 mock 或 Rust 測試代替實機。

既有 v2 陳舊自動狀態於重新開書／重新加入／relink、保存進度或選「依進度判定」時修正；不在啟動時全面回填現有 v2 庫。既有 v2 備份還原仍保留其保存狀態；需要上述操作重新推導。手動狀態不因修正失去指定值。未合併 main、force push、打包或發布；待獨立複審與原生驗收。

## 既有 v2 陳舊狀態驗收

本節是複審及隔離實機要求，不宣稱已執行。只調整文件，沒有新增全庫回填或產品修改。

1. 使用臨時來源、隔離 v2 SQLite 與 v2 備份建立至少兩種陳舊自動狀態（statusManual=false）：6 頁、雙頁且封面不獨立、index=4 卻保存 reading；以及單頁來源已由3頁增至4頁、index=2 卻保存 read。另備手動已讀與未讀對照。
2. 啟動既有 v2 庫，記錄保存值與初始書庫顯示；將 v2 備份還原至沒有相同來源的隔離庫，確認原保存狀態保留。相同來源 skip 的案例另列，不能把略過誤當新還原成功或修正生效。
3. 每個觸發操作用相同快照重置，避免前一個操作已修正而掩蓋後續案例。分別執行重新開書、保存進度、重新加入、relink、選「依進度判定」。記錄操作前、後端成功／失敗後、進度保存完成後，以及返回／刷新書庫後的狀態和時間點；區分資料庫已更新與前端尚未刷新，不只看最終截圖。
4. 成功後核對已讀篩選與繼續閱讀候選與新狀態一致（同時考慮來源可用性、閱讀時間等既有候選條件），續讀索引依既有契約保存或按頁名定位；ID、收藏、偏好、自訂資訊及標籤無非預期變更。操作失敗不得部分更新或污染目前 reader。
5. 重新開書、保存進度、重新加入、relink 不覆寫手動已讀／未讀，v2 還原保留手動值。明確選「依進度判定」是使用者解除手動標記，屬既有契約的例外，須單獨確認，不能誤稱仍維持手動值。
6. 核對 `docs/library-guide.md` 對舊自動狀態的更新方法、啟動／v2 還原不全面回填及手動狀態例外說明；同時檢查程式內教學是否清楚。缺口列入新報告，提供位置及建議，不自行修改產品。

若觀察結果或上述行為與產品需求不符，先提出具體觸發、影響、建議修法與相容性／交易風險，待另行授權。不得直接進行全庫遷移；本輪不擴大產品範圍。原報告保留不改，新結果統一寫入 `docs/next-phase-review-fix-report.md`。
