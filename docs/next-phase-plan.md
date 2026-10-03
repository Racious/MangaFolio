# 下一階段開發計畫

日期：2026-10-03 UTC。Repository：Racious/MangaFolio。分支：`feature/library-management-ui`。
固定開發基準：`bac9674f3b0d2a5c2e19774d5294d7cbfd2f86b8`；開始前核對遠端最新提交一致，包含 A、R1、R2 產品修正 `878d2bb6918fe1adbf1b0d394dbec6f6f478bdd3`。
已閱讀指定五份文件；未找到適用 AGENTS.md。從功能分支建立隔離 worktree `/workspace/MangaFolio-management-ui`；原兩個工作區保持不變。

## 工作進度與設計決定

| 階段 | 狀態 |
| --- | --- |
| 基準核對、實際現況擷取 | 完成：Chromium mock IPC 與 Linux 原生隔離環境，證據分開保存 |
| 三個視覺方向 | 已提供相同四本資料的設計示意；使用者選擇三種全部實作，設定中自由切換 |
| 資料契約、F1–F6 | 完成：schema／備份 v2、狀態、標籤、資訊編輯、匯入結果與遺失來源 |
| U3–U8 | 完成：共用三種檢視、三種風格、主題、密度、封面尺寸、響應式及自願教學 |
| 整合驗證與交辦 | 見驗證紀錄及審查交辦；Windows 實機與原生選檔完整矩陣待驗，不宣告獨立審查通過 |

使用者補充要求易於擴充與改色：風格 registry 位於 `src/lib/appearance.ts`，語意色彩／間距／尺寸 token 集中於 `src/themes.css`。風格、明暗、檢視、密度、封面尺寸是獨立欄位；共用元件與資料操作，不維護三套功能。新增內建風格主要修改 registry 與 token。沒有外部主題、自訂 CSS 執行或新插件架構。

## 資料契約

- SQLite schema v2：v1 以單一交易升級欄位與標籤表；未知較新版本拒絕。不可降回 v1 程式直接使用升級後資料庫；請先保存舊備份。無全庫路徑遷移、重複資料自動合併或 AUTOINCREMENT 調整。
- 閱讀狀態 `unread`／`reading`／`read` 與 `statusManual`。未保存閱讀時間為未讀；保存未到最後頁為閱讀中，最後頁為已讀。手動標記持續生效，後續進度不覆寫；「依進度判定」可恢復自動。標記不清除頁碼、頁名、閱讀時間或偏好。批次失敗整批回滾。
- `sourceTitle` 沿用 books.title；`customTitle` 留白使用來源名。重新加入更新來源名稱，不覆寫自訂資訊。系列／集數為文字、備註為純文字；長度依序 256／256／64／4000 Unicode 字元。搜尋顯示名、來源名、系列、集數、備註、標籤，排除來源路徑；保留自然排序。
- tags／book_tags 多對多；標籤 trim 後 1–64 字元，不含控制字元，Unicode lowercase 判斷重名（未作 NFKC 合併）。最多 1000 標籤／每本 100。刪標籤只刪關聯，批次指派／移除交易處理。
- 備份 v2 含新欄位、完整標籤 catalog（包含未指派標籤）與關聯，以名稱映射跨庫 ID。讀 v1 時推導狀態及空資訊。較新版本、無效／重複來源與標籤關聯整批拒絕；交易還原，既有來源略過不覆寫 metadata。備份快照同一讀交易；匯出 create_new，不覆寫來源或既有備份。上限 16 MiB／10000 本不變。
- 匯入逐項結果 added／updated／failed／conflict／canceled。取消停止尚未開始項目，當前項完成後保留；失敗不阻止後續項目，可重試失敗／衝突／未開始者。匯入與管理、收藏、開書互斥，正常關閉等待操作完成。
- 來源失效集中篩選，不自動移除。單本 relink 保留 ID、收藏、進度、偏好及新增資訊，衝突拒絕。來源識別仍沿用 R1。
- 外觀 localStorage v1：style calm/catalog/night；theme light/dark/system；view grid/detail/compact；density comfortable/compact；coverSize small/medium/large。逐欄白名單解析；缺失／損壞採預設，保存失敗不阻止使用。舊外觀設定補預設 style。只在本機保存，不納入書庫備份。

## 視覺證據與現況改善

基準實際畫面在 `images/next-phase/current/`（mock IPC）及 `native-current/`（Linux 原生），不是舊截圖推測。管理面板和展開教學占窄視窗首屏；本輪改為共用工具區、可收合導覽／設定／標籤／備份，教學預設收合並可重開。有效選取跨搜尋、篩選及外觀切換保留，面板提示可能含其他篩選中的選取。

設計示意位於 `images/next-phase/directions/`：靜謐書架（暖白茶綠）、目錄工作台（冷白藍）、夜讀書房（炭灰暖金）。示意中的作者搜尋與失效數字不列入產品規格。實作畫面在 `implemented/`；原生畫面在 `native-implemented/`，樣本漫畫為隔離測試圖片，不包含使用者資料。

不新增書籤、雲端、帳號、付費、背景監控／全碟掃描。未合併 main、force push、打包、簽章或發布。最終固定提交及審查要求見 `next-phase-review-assignment.md`。
