---
title: 第三階段 T2-R1 修正複審
type: review
date: 2026-10-04
reviewer: 天城／Codex 桌面互動審查
---

# 第三階段 T2-R1 修正複審

## 結論與版本

**前輪後端 T2-R1 已修復，可就該項結案；本輪另確認 1 項 P2 的既有面板錯誤殘留，仍阻擋「恢復後面板錯誤消失」的完整宣稱。** 這是先前未涵蓋的前端路徑，不是此次 Rust 條件修改引入的退化。Windows Rust **49／49**、前端 **21／21**、build 及固定範圍 diff-check 通過。

- Brief：`docs/third-phase-t2-r1-handoff.md`。
- 聚焦比較：`52e30f5865a0596514f5fa9e03187c3f1e28a3cf` → `0b883c85e77ea8046fa851464c80a86fbbb7c513`。
- 累積修正基準：`421d2acea22fccb7f2822848c80ac5a2a265270e`；整體第三階段基準：`e38f7a84c33126e11a141d10c0f38f4defb8985f`。累積範圍沿用既有三份獨立報告，核對此次修正的影響，不重開已確認需求。
- 實際 checkout：`feature/series-library-reading`，HEAD `00f679962ca64746cd2be98f17c50eca3874c5a6`。固定產品之後只有文件／驗證紀錄差異。
- 本次產品 diff 只有 `library_safety.rs` 的恢復錯誤前綴／清除條件與 `library_reading.rs` 兩個回歸測試，其餘為文件／證據；行號對應固定產品 `0b883c8`。
- 原工作區已有 `src-tauri/Cargo.toml` 修改，未納入固定產品驗證、未更動；三份歷史報告保留。

## 必要修正

### T2-UI1 — [P2] 立即備份錯誤被面板重複保存，排程恢復後仍顯示失敗

- **位置**：`src/components/BackupPanel.vue:81–84`、`:37–38`、`:139–143`；排程入口為 `src/App.vue:20–22,69`。
- **觸發**：使用者在備份面板按「立即建立本機安全備份」，遇到已建立備份但清理失敗；保持面板掛載，下一次排程恢復仍失敗，故障排除後再一次排程檢查成功。面板應反映恢復成功，但仍顯示第一次立即操作的失敗。
- **原因**：`automatic()` 在 `backup.maintain(true)` 捕捉錯誤後，再將 `backup.error` throw 給 `run()`，後者複製到面板自己的 `error` ref。後續 `maintain(false)` 雖清空 store 的 `error` 並套用後端返回的空 `lastError`，沒有清除面板本地副本；模板又優先顯示 `error`。本地錯誤只在下一次面板 `run()` 或元件重新掛載時清除，configure／成功排程不會清除。
- **影響**：後端恢復與保留清理已成功，面板仍宣告清理失敗、下次會重試，形成與真實狀態矛盾的常駐警告。沒有確認資料遺失。此路徑原已存在，直接影響本輪交辦的畫面恢復要求，故列為剩餘必要修正。
- **獨立重現**：使用固定產品的真實 Vue 元件／Pinia，隔離 mock IPC 回傳首次「備份已建立，但保留清理失敗」、第二次「備份狀態恢復／清理失敗」、第三次成功空錯誤。首次透過真實立即備份按鈕觸發，後兩次呼叫 App 實際註冊的 900000ms 排程 callback（加速觸發，不等待 30 分鐘）。最終 `pending=false`、store `error=""`、`settings.lastError=""`，但 `.backup-panel [role=alert]` 仍為 `Error: 備份已建立，但保留清理失敗…cleanup failed`，`pageErrors=[]`。缺陷斷言成立；mock 不代表原生 SQL／I/O 測試。
- **建議**：讓自動備份操作的錯誤由 backup store 單一來源呈現，避免再存入通用面板錯誤；或明確區分本地錯誤的操作來源，成功恢復時只清除對應備份錯誤，保留仍有效的匯入／預覽／還原錯誤。補真實元件回歸：立即失敗 → 排程再失敗 → 排程恢復成功，面板 alert 消失；尚未重試的建立失敗仍可見，非備份錯誤不得被誤清。

## 前輪項目與完成度核對

| 項目 | 判定與證據 |
| --- | --- |
| T2-R1 後端錯誤殘留 | **已修復**。恢復與清理均成功後，新增清除 `RECOVERY_FAILURE_PREFIX`；錯誤產生與識別共用常數，對外文案不變。仍失敗的 `?` 分支會先返回 Err，不會執行成功清除 |
| 連續 DELETE 失敗回歸 | 本輪 Windows 通過。enabled=true 的 24h gate 內與 enabled=false 都驗證清空錯誤、成功時間不變、已登記有效 JSON 1 份、原檔名／位元組不變、下次仍穩定、未知有效 JSON／其他文件／升級快照保留 |
| 未恢復的建立錯誤 | 本輪 Windows 通過。INSERT 失敗後，停用及 24h gate 內的清理成功不誤清建立錯誤；最近成功與檔案数不變。此次條件並未包含建立／登記／寫入失敗前綴 |
| T1 | Windows v1、v2／WAL 與真正 DDL collision 回滾測試本輪持續通過；前輪結案不變 |
| T2 登記／時間／清理 | manifest INSERT 失敗保護、成功時間恢復先於每日 gate、未知檔保護與保留清理測試本輪持續通過；畫面端另受 T2-UI1 限制 |
| T3／S1–S6／F1–F6 | 此輪未改返回位置、系列／閱讀器或 schema／JSON 契約。相關原始前端／Rust 回歸通過，沒有確認本次引入的新退化；T3 五組 browser 證據沿用前輪，非本輪重跑 |
| A／R1／R2、N1／N2 | 來源 ID／別名衝突、收藏與狀態保持、v1/v2 相容及交易回滾的既有測試本輪通過；不要求已排除的 v2 全庫狀態回填 |

本次沒有新增持久化階段列舉，前綴識別採既有契約；共用常數加兩個有實際分支覆蓋的測試足以處理本輪後端問題，沒有理由僅因偏好不同方案而阻擋其結案。

## 本輪驗證

所有驗證在 repo 外固定產品 Git archive 副本 `mangafolio-t2-r1-review-0b883c8-20261004/source` 執行。測試 Fixture 建立臨時 DB／來源；沒有啟動正式 Tauri identifier 或接觸使用者 APPDATA。

| 命令／案例 | 本輪結果 |
| --- | --- |
| npm ci --ignore-scripts --no-audit --no-fund --offline | 使用既有離線快取，84 packages，成功 |
| npm test | **21／21 通過**，exit 0 |
| npm run build | vue-tsc／Vite、1613 modules，**通過**，exit 0 |
| cargo test --locked --offline --manifest-path src-tauri/Cargo.toml | Windows 原始 **49／49 通過**，exit 0；包含新增兩項與 Windows cfg，未追加 Rust probe |
| 聚焦固定範圍 git diff --check | 從固定副本執行，以該副本為 work-tree，**通過**，exit 0 |
| independent-ui-recovery.py | **1 情境缺陷重現成功**，exit 0；代表缺陷斷言成立，不代表 UI 正確 |

UI 獨立重現腳本與 `independent-ui-recovery/results.json` 僅在暫存副本，未寫回產品或覆寫歷史證據；隔離 Vite 伺服器已停止。主導方的 Linux 48／48 與修正前失敗 log 已核對，和本輪 Windows 49／49 分開，不把 Linux 結果冒充本輪執行。

## 限制與接手

- 本輪沒有重跑前輪四支完整 browser script、Linux 原生流程或 Windows 原生 Tauri；UI 重現只驗證元件／store／排程與隔離 IPC 契約。Rust 49 項通過不代表原生面板、ACL／ENOSPC、UNC、選檔／關閉重啟、IME／觸控、高 DPI、輔助科技、全部三檢視返回矩陣或原 PNG 比對已驗收。
- 主導方可評估前輪 **T2-R1 後端結案**，另逐項評估 **T2-UI1**；採納後补面板錯誤來源處理及回歸，再確認完整恢復宣稱。審查方不代填採納／裁決。
- 此輪僅新增指定報告，未修改產品、設定或既有成果，未 commit／push／合併／發布。
