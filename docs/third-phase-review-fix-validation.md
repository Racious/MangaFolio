# T1／T2／T3 本次修正驗證

最新 T2-R1 補強產品：`T2_R1_PRODUCT_SHA_PENDING`，交辦與本次 Linux npm21／Rust48、build／diff 输出見 [T2-R1 修正交辦](third-phase-t2-r1-handoff.md)。最新 Windows 尚未執行；45568a8 的獨立 Windows47 通過不涵蓋本次。下方保留前輪歷史範圍與證據。

日期：2026-10-04 Asia/Tokyo。固定基準 `421d2acea22fccb7f2822848c80ac5a2a265270e`；對應產品 `45568a804b1228c9016f9a7fd2bd876218673b2e`。本次重跑，不引用歷史測試宣告最新程式通過。

| 命令／案例 | 結果 | 完整輸出 |
| --- | --- | --- |
| npm test | 21 通過，exit 0 | [npm-test.log](third-phase-review-fix-results/npm-test.log) |
| npm run build | 通過，exit 0 | [npm-build.log](third-phase-review-fix-results/npm-build.log) |
| cargo test --locked --manifest-path src-tauri/Cargo.toml | Linux 46 通過，exit 0 | [cargo-test.log](third-phase-review-fix-results/cargo-test.log) |
| git diff --check | 通過，exit 0 | [diff-check.log](third-phase-review-fix-results/diff-check.log) |
| validate-next-phase.py | 通過，exit 0；原管理功能與 10000 本量測 | [browser-library.log](third-phase-review-fix-results/browser-library.log) |
| validate-immersive-reader.py | 通過，exit 0；工具列、錯誤、IME guard、dialog／touch browser | [browser-reader.log](third-phase-review-fix-results/browser-reader.log) |
| validate-third-phase.py | 通過，exit 0；系列／下一集／書籤／預覽／三風格 | [browser-third.log](third-phase-review-fix-results/browser-third.log) |
| validate-third-phase-review-fixes.py | 5 組 T3 回歸通過，exit 0；120 本／系列、錨點、內外 scroll owner | [browser-scroll.log](third-phase-review-fix-results/browser-scroll.log) |

退出碼見 [checks.json](third-phase-review-fix-results/checks.json)。T3 每次返回的完整筆數、前後 scroll／offset 與 pageErrors 見 [results.json](third-phase-review-fix-results/browser-scroll/results.json)；實際 Vue 截圖在同目錄。原功能／三風格畫面在 `docs/images/third-phase/review-fixes/`，全部為本次 browser screenshot，不是生成稿或原生 Windows。

Rust 新增 2 個 T2 測試，原 cleanup 測試增加 SQL 失敗重試與未知檔保留；DDL rollback 測試加實際錯誤斷言。A／R1／R2、N1／N2、v1/v2、WAL、系列／書籤與備份回滾原測試維持通過。只用臨時來源及 DB；browser 使用記憶體 IPC，不操作使用者 DB。

## 未執行與歷史結果

最新 Windows Rust cfg 與 Windows 原生實機流程未執行；Linux 原生 Tauri 本次未重跑。原 Windows 審查 Rust 43／45（兩個 T1 失敗）與原 Linux 44 是歷史資料，不代表最新修正已通過 Windows。ACL／ENOSPC、UNC、原生選檔、中文 IME／實體觸控、高 DPI、輔助科技及正確原圖歸檔仍待隔離驗收，步驟見修正交辦。

## 執行中失敗紀錄

`attempts/` 保留初次 Rust 46 通過及 build 通過輸出；第一次新 browser script 因 Vite 未啟動而連線拒絕，啟動隔離 Vite 後通過。補強「檔案已刪／SQL 清理失敗」分支後有一次 Rust 編譯 moved name 錯誤，已改為借用名稱；保留該失敗輸出，最終 cargo 重新執行。這些嘗試不取代上表最後一次驗證。
