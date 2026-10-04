---
title: 第三階段獨立審查補充核對
type: review
date: 2026-10-04
reviewer: 天城／Codex 桌面互動審查
---

# 第三階段審查補充核對

## 結論與版本

**確認 1 項 P1、2 項 P2，與既有第三階段報告一致；尚不宜視為 Windows 升級驗收通過。** 本輪重新執行固定產品的 Windows Rust、前端測試、build，以及兩種返回位置與備份登記失敗重現，沒有修改產品。

開工時工作區在 `main`，HEAD 為 `e38f7a84c33126e11a141d10c0f38f4defb8985f`，指定交辦單不在目前 checkout，但可由本機 `feature/series-library-reading` 的 `a6c11396f1ae829f4a235fbb14290a2c0a243936` 讀取。已有未追蹤 `docs/third-phase-review-report.md`，故保持原文不改，本次結果另存此檔。這是同一產品的證據核對，**不是修正後複審通過**。

- Brief：`a6c1139:docs/third-phase-review-assignment.md`。
- 固定比較：`e38f7a84c33126e11a141d10c0f38f4defb8985f` → `ad0e366cb0d7c4b583968243f8d864d58fce0d3e`。
- 產品 head 後的 `a6c1139` 修改 8 個文件／證據檔，沒有產品程式差異。
- 行號均對應固定產品 `ad0e366`，不對應目前 main。
- 已核對交辦、計畫／驗證／視覺文件、原報告、新增系列／書籤／備份後端及其交易、store、返回與閱讀器／詳情／備份 UI 呼叫路徑。

## 必要修正

### T1 — [P1] Windows 舊書庫升級快照同步失敗，阻擋啟動

- **位置**：`src-tauri/src/library_safety.rs:28–30`，由 `library.rs:233–235` 的 v1／v2 升級入口呼叫。
- **觸發與影響**：Windows 打開既有 v1 或 v2 書庫。SQLite 快照完成後，程式用唯讀 `File::open` 的 handle 呼叫 `sync_all`，收到 os error 5。錯誤向上傳至啟動 setup，使使用者不能升級進入程式；原 schema 不會因這次失敗被升級。
- **本輪重驗**：新隔離副本原始 45 項 Rust 測試為 **43 passed／2 failed**、退出碼 1。`upgrade_snapshot_includes_uncheckpointed_wal_and_failure_keeps_v2` 在 `library_reading.rs:421`、`v1_database_migrates_without_losing_identity_or_progress` 在 `library.rs:1333` 均收到「存取被拒」。與既有報告的最小唯讀／可寫同步 probe 相符；該 probe 本輪未重跑。
- **建議**：以具寫入權限的 handle 同步快照，保留真正備份／同步失敗即阻止升級的契約。重驗 Windows v1／v2、WAL、原資料與成功快照保留，並確認 migration DDL 失敗案例確實抵達 DDL，而非快照提早失敗造成假綠燈。

### T2 — [P2] 自動備份登記失敗仍更新成功時間，排程跳過恢復

- **位置**：`src-tauri/src/library_safety.rs:158–174,218–227` 與 `library.rs:652–659`。
- **觸發與影響**：JSON 已成功寫入／同步，但 manifest INSERT 失敗。`export_to` 已先更新 `last_success`，manifest 為空；回傳「備份失敗」，下次非強制檢查又因 24 小時間隔跳過。可用檔案未登記，不能納入保留清理；成功時間與錯誤文字矛盾，持續強制重試會留下未納入保留控制的檔案。
- **本輪重現**：只在新暫存副本追加 Rust 測試，以真實 SQLite trigger 注入 INSERT 失敗。結果為 `files=1, registered=0, last_success=Some(...)`，下一次 `automatic_backup(false)` 沒有恢復登記。追加測試 1／1 通過，代表缺陷斷言成立，不代表產品行為正確。
- **建議**：區分建立、登記與清理階段；保留已成功 JSON，回報部分成功，增加登記恢復／pending 狀態。成功寫入可仍計入最近備份時間，但不能用建立間隔阻止 bookkeeping 重試。

### T3 — [P2] 返回書庫未保存展開範圍，窄窗保存了錯誤捲動容器

- **位置**：`src/components/LibraryView.vue:32,106,111–136,998–1026`；`App.vue` 返回 reader 後重新掛載書庫。
- **觸發與影響**：超過初始 60 本並在後段開書，或窄視窗捲動後開書再返回。元件的 `limit`／`seriesLimit` 重設為 60，原位置超出目前列表高度；窄窗實際捲動 `.library-layout`，卻只保存 `.library-content.scrollTop`。使用者需重新展開並尋找書籍，S2 返回位置契約未完整達成。
- **本輪重現**：實際 Vue＋100 本 mock IPC，寬窗開第 80 本後返回：渲染數量 `100 → 60`，scrollTop `9486 → 0`。640×480：layoutScroll `3293 → 0`，contentScroll 一直是 0。兩個缺陷斷言成功，`pageErrors=[]`；使用既有重現腳本，在新副本／新本機 port 重跑，沒有覆寫舊輸出。
- **建議**：保存展開範圍或書籍錨點，待資料刷新與列表高度就緒才恢復；統一或追蹤實際捲動 owner。補 >60 本／系列、系列內返回、窄窗及尺寸切換回歸。

## 完成度及回歸

| 範圍 | 核對判定 |
| --- | --- |
| S1 系列 | 聚合、歧義集數與批次缺少 ID 回滾有本輪程式／測試支持；沒有確認新的必要修正 |
| S2 詳情／下一集 | 相鄰唯一數字集數、失效來源與 save／load 失敗保留 reader 的測試通過；返回位置受 T3 阻擋 |
| S3 書籍詳情 | 共用資訊、收藏、編輯／管理入口與窄窗 dialog 有程式／既有 browser 證據；本輪未完成原生矩陣 |
| S4 書籤／筆記 | CRUD／限量、純文字、按頁名跳轉、缺頁不換 session、ID 映射／skip／還原 SQL 回滾本輪測試通過 |
| S5 安全備份 | 預覽唯讀、v3／舊 JSON、驗證／回滾及保留清理案例通過；T1／T2 阻擋完整完成宣稱 |
| S6 風格／閱讀器 | 本輪型別及 build 通過，檢查固定偏好、覆蓋工具列、dialog 與錯誤入口；三套完整 browser 測試沿用既有報告，不稱本輪重跑。原圖／原生 IME／觸控／無障礙仍待驗 |
| A／R1／R2、N1／N2、F1–F5 | 本輪相關前端／Rust 案例通過，未確認新退化；不重新要求既有 v2 全庫狀態回填 |
| F6 相容性 | JSON 路徑未確認新退化；Windows 舊 DB 升級因 T1 失敗，不能判相容性整體通過 |

## 本輪執行證據與限制

| 項目 | 結果 |
| --- | --- |
| 新副本 | 固定產品 Git archive，系統暫存 `mangafolio-third-phase-confirm-20261004/source`；原 checkout 未切分支 |
| npm ci | offline／ignore-scripts，84 packages，成功 |
| npm test | **21／21 通過**，退出碼 0 |
| npm run build | **通過**，vue-tsc／Vite，1613 modules，退出碼 0 |
| 固定範圍 diff-check | 在固定產品副本作為 work-tree 且由該副本執行，**通過**、退出碼 0；產品／測試／腳本的獨立檢查亦通過 |
| 原始 cargo test --locked --offline | **43 通過／2 失敗**，退出碼 1；尚未追加 probe 前執行，不將 probe 計入原始 45 項 |
| 追加 manifest probe | **1／1 缺陷重現成功**，退出碼 0 |
| 追加 Vue 返回位置 probe | **2 情境缺陷重現成功**，退出碼 0，無 pageErrors；隔離 Vite 測試伺服器已停止 |

初次從目前 main 做 diff-check 時，main 缺少第三階段 logs 的 attributes，歷史原始輸出出現尾端空白警告；改從固定產品副本執行並指定該 work-tree 後通過。固定副本 `docs/third-phase-results/.gitattributes` 只排除原始 logs 的尾端空白檢查，不能把 checkout 差異當產品新增缺陷。

既有完整報告 `docs/third-phase-review-report.md` 保留。其三支 browser 整合、Linux 有限原生畫面及原圖比對界線本輪未全部重跑；本輪只重跑上表及聚焦重現。沒有啟動正式 Tauri identifier、操作使用者 APPDATA、修改產品、提交／推送／合併或發布。

仍需隔離 Windows 原生選檔／UNC／ACL／ENOSPC／關閉重啟／IME／觸控／高 DPI／輔助科技，以及正確原始 PNG 對照。主導方應逐項評估 T1–T3，修正後再複審；沒有已修正或審查通過宣稱。
