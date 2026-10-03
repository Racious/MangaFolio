# N1／N2 修正複審交辦

日期：2026-10-03（Asia/Tokyo）。分支：`feature/library-management-ui`。
原獨立報告：[next-phase-review-report.md](next-phase-review-report.md)，保持原文不改。

| 提交 | 固定 SHA |
| --- | --- |
| 下一階段原始開發基準 | `bac9674f3b0d2a5c2e19774d5294d7cbfd2f86b8` |
| 首輪產品 | `08ccc3e447372ade1834e8d3d63cabc8df23c5b0` |
| 本次修正基準／審查報告提交 | `1394670f3b4ac63ab69fcedcabb650a2b0e97879` |
| 本次最新產品 | `10af40e99c703c31332e00f5ab5bcd3c7ebe2ccc` |

複審差異固定為 `1394670f3b4ac63ab69fcedcabb650a2b0e97879` → `10af40e99c703c31332e00f5ab5bcd3c7ebe2ccc`。全部下一階段產品範圍為 bac9674 → 10af40e。後續提交僅更新交辦／驗證／教學文件，不改產品程式；文件提交用 `git log -1 -- docs/next-phase-review-fix-handoff.md` 核對。

## 修正內容

- `src-tauri/src/library_metadata.rs`：共用 `automatic_status` 依單／雙頁、獨立封面、頁數與索引計算目前視圖是否含末頁，對齊 reader 的 pairStartOf／indicesOf。無閱讀時間仍為未讀；不改跨頁起始索引。
- N1：`library.rs::save_progress` 在交易內核對有效頁碼，以共用規則保存自動狀態，手動狀態不覆寫。「依進度判定」也使用同一規則。v1 schema 升級及 v1 備份預設狀態同步使用，避免舊資料雙頁末組仍套單頁判定。
- N2：普通／別名重新加入與 relink 在原有交易內，依頁名重新定位、缺頁時限制索引，再更新自動狀態；手動已讀／未讀保留。ID、收藏、閱讀時間、偏好、自訂資訊、標籤均保留。失敗整筆回滾，不替換目前 reader。
- `tests/favorite-store.test.mjs`：實際 reader store 跨頁末組與 progressSnapshot 契約回歸。只 mock IPC，不宣稱原生驗收。
- `library.rs` 測試：1–8 頁、單／雙頁、獨立封面開關、每個有效索引、自動重設、手動狀態；真實 PNG 來源擴頁／縮頁／頁名定位、重新加入及 relink；注入 SQL 失敗整筆回滾；舊 schema／v1 備份末組。A／R1／R2 既有測試仍通過。

沒有修改前端翻頁、儲存索引或來源識別契約，沒有 schema 升版、全庫回填、路徑遷移或自動合併重複紀錄。

## 本次實際驗證

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
