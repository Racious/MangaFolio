# 工作日誌

初版以 2026-10-02（UTC）整理，2026-10-03（Asia/Tokyo）追加本輪工作；未記錄精確執行時間，不補造時間戳。提交紀錄可使用 `git log --oneline` 核對。新增紀錄請附日期、具體成果、驗證、提交與未完成事項。

## 前期：環境與產品檢視

- 按 cloud-environment-onboarding:setup 準備 Node 22.23.3、Rust 1.95.0 及 user-local Linux Tauri 依賴，保存可重用的設定草稿。
- 檢視產品並建議先實作書庫、收藏、最近閱讀、書名搜尋與續讀。
- 詳細產品檢視資料在原雲端工作區 `/workspace/MangaFolio-review/product-review.md`，不是目前 repo 內的正式規格。

## 2026-10-02：核心功能

- 建立 `feature/library-favorites-resume`，加入 SQLite 書庫、封面快取、收藏、搜尋、最近閱讀、進度與偏好記憶。
- 為切書與預載加入 session 隔離；改善小視窗工具列、輸入時快捷鍵處理與儲存錯誤提示。
- 驗證：前端建置、3 個前端測試、11 個 Rust 測試通過；Linux 原生桌面實測通過。詳見 [驗證紀錄](validation.md)。
- 提交：`06469a6`。初次完成時只在雲端本機提交，沒有聲稱已發布安裝包。

## 2026-10-02：雲端中斷與分支推送

- 曾因雲端離線及 `exec-server protocol error` 無法讀取截圖或執行 Git，先前貼出的雲端路徑也未在使用者介面成功顯示圖片。
- 連線恢復後確認分支、原提交與工作目錄仍在；依使用者要求推送到 GitHub。
- 透過 `git ls-remote` 核對遠端為 `06469a679df9910c7d165606d7850ae7ddeca47e`；用直接附圖方式展示保存的實測截圖。
- 未合併 main、未建立 PR、未發布新版本。

## 2026-10-02：新功能教學

- 首頁加入五步教學：加入 → 閱讀 → 收藏 → 搜尋 → 續讀。可指定步驟、標示操作區域、收合與重看，使用 localStorage 保存狀態。
- 加入 [新功能教學](library-guide.md) 及書庫實測截圖，更新 README 與 Unreleased。
- 驗證：前端建置與 3 個既有測試通過；以模擬 Tauri IPC 的 Chromium 檢查教學切換、持久化、收合與 640px 版面。未重跑 Rust 或聲稱完成 Windows 實測。
- 提交：`7fe4acb`，已推送並核對遠端為 `7fe4acbf602133951687dbb675acf6d543acfe8f`。

## 2026-10-02：文件交接整理

- 新增文件索引、開發交接、驗證紀錄及本日誌；保留教學與 CHANGELOG 的用途區分。
- 更新 README 對 P3 進度記憶的描述與架構入口，避免與現況混淆。
- 記錄資料備份方式、雲端專用設定位置、尚未審查／驗證的項目及後續優先工作。
- 此次為文件變更；核對來源、既有驗證輸出與文件相對連結，不重跑產品測試。提交與推送結果以此日誌所在的 Git commit 為準。

## 2026-10-03：書庫管理、備份還原與審查交辦（Asia/Tokyo）

- 依使用者要求接續管理與備份開發，沿用原功能分支。
- 新增卡片選取、批次收藏／取消收藏／移除、單本重新指定 ZIP／CBZ／圖片資料夾來源；原始漫畫保留。
- 新增格式 v1 JSON metadata 備份與合併還原，16 MiB／10,000 本限制，整批驗證、交易回滾、相同來源略過及匯出不覆寫。
- 管理前保存目前閱讀位置，成功移除／重新指定載入中的書後清除舊閱讀狀態；管理期間限制重複操作與正常關閉。
- 驗證：前端建置與 3 個既有測試、18 個 Rust 測試、原生 debug build 通過；新增可重跑的 Playwright mock IPC smoke。
- 原生桌面確認管理畫面、批次收藏／取消收藏、取消／確認移除、操作中關閉限制與漫畫保留；選檔遇到 PRoot／GTK `Bad address`，調整執行環境後仍未完成匯出／還原全流程，列為本地補驗缺口。
- 實作提交：`df6c134`；更新教學、交接與驗證紀錄，新增固定提交範圍與報告格式的 [本地審查交辦單](review-assignment.md)。文件提交及推送以 Git 紀錄為準。

## 待續工作（更新）

獨立審查、Windows 與安裝包驗收、安全審查、原生備份全流程驗收及下一輪管理擴充尚未完成。優先順序見 [交接](handoff.md#下一位接手者的優先事項)。

## 2026-10-03 UTC：書庫管理與可切換介面

- 核對遠端基準 bac9674f3b0d2a5c2e19774d5294d7cbfd2f86b8，從原功能分支建立 feature/library-management-ui 及隔離 worktree，原工作區修改保留；A／R1／R2 及回歸保留。
- 實際啟動 Chromium mock 與 Linux Tauri 隔離測試，擷取現況後提供三張設計示意；使用者選定全三種並要求未來容易增減改色。
- 完成 F1–F6：手動／自動閱讀狀態、多標籤、自訂資訊、逐項匯入取消／重試、失效來源管理、schema／備份 v2 及 v1 相容／交易回滾。
- 完成 U3–U8：三種風格及三種共用檢視、獨立明暗／密度／封面尺寸設定、localStorage 容錯、響應式導覽、狀態／結果與自願教學。外觀 registry 和語意 token 集中，不下載外部主題或執行 CSS。
- 本次 npm 15 項、Rust34 項、build、diff-check 通過；18 外觀組合、管理 smoke、36 色對、10000 資料與封面佇列驗證。完整輸出見 next-phase-results/final/ 與 ui/。
- Linux 原生資訊編輯、續讀／返回及三種風格畫面；全部資料與來源在 /tmp/mangafolio-next-phase-native/，未碰使用者實際資料庫。
- Windows cfg／native、原生選檔全矩陣、原生 IME／高 DPI／螢幕閱讀器及 UNC 效能未執行，清楚交辦。未合併 main／force push／發版。
- 最終產品與文件提交 SHA 於產品提交後由獨立文件提交固定於 next-phase-review-assignment.md；交付材料包括實際程式、原審查、計畫／驗證／審查交辦及畫面證據。

- 產品提交 `08ccc3e447372ade1834e8d3d63cabc8df23c5b0`；後續文件提交固定 SHA 並明確納入既有忽略規則下的完整 .log 測試輸出，不改產品。

## 2026-10-03（Asia/Tokyo）：N1／N2 審查修正

- 同步獨立報告提交1394670；修正雙頁末組自動已讀及來源頁數改變後陳舊狀態。統一規則，保留跨頁起始索引／手動狀態；重新加入及 relink 同交易定位／推導，失敗回滾。
- 舊 schema 升級／v1 備份使用相同規則；未全庫回填 v2 或變更路徑識別。
- 本次 npm16／Rust37、build與diff-check通過，新增奇偶頁／封面／手動狀態、真實來源變化、metadata保存及注入SQL失敗測試。完整輸出 review-fix-n1-n2/。
- 產品提交 `10af40e99c703c31332e00f5ab5bcd3c7ebe2ccc`，後續僅文件提交。未操作實際使用者庫；Windows cfg／native未執行，待複審。交辦 next-phase-review-fix-handoff.md。


## 2026-10-04｜介面精修與沉浸閱讀預覽

以 `54fc37596d5d34bfed72760bb8770969ab9f7774` 為基準，保留審查報告，重新比對三種原設計方向及實際畫面。調整封面尺寸、書名層次、續讀區、列表欄位、圖示及裝飾邊界，風格差異集中到主題參數。開書時上下工具列預設隱藏，hover、鍵盤焦點、Esc 與觸控可開啟；工具列覆蓋畫布，錯誤與重試仍可見。

隔離瀏覽器及 Linux Tauri 有實際截图，驗證輸出另存 `docs/next-phase-results/visual-refinement`，不覆蓋歷史證據。本次先提供 [完整畫面](ui-refinement-preview.md) 給使用者看，修改保留工作區，未產生本次提交。Windows 原生及實體觸控尚未驗收。

使用者後續授權將目前功能分支合併主線，要求保留 merge commit；先提交介面精修及隔離驗證證據，再以 `--no-ff` 合併。Windows 本次原生驗收仍未執行，合併不代表發布版本。


## 2026-10-04｜第三階段系列、書籤與安全備份

基準 e38f7a84c33126e11a141d10c0f38f4defb8985f，從已保留 merge 的最新主線建立 feature/series-library-reading；前階段 UI 精修 e2a87d6 已在基準。起始 workspace 乾淨，原工作區／提交保留，未找到適用 AGENTS.md。

使用者訂正參考圖片，已直接讀取對話正確三張，不引用舊圖。先記錄配套設計及 v3 契約，再完成系列／詳情、保守下一集、書籤純文字筆記、WAL 一致性升級快照、自動備份／唯讀還原預覽，以及三風格共用介面。

整合期間修正書籤 dialog 受隱藏工具列影響（Teleport）及關閉 dialog 阻擋 Esc／方向鍵；新增回歸。保存完整嘗試輸出與最終 npm21／Rust44、build、三組 browser 測試。大型書庫 10,000 本聚合／初始 60 個系列封面量測；另補切集 in-flight 關閉視窗保護與成功備份後清理失敗的精確回報。修正共用封面在來源重新上線後的更新與過期非同步結果保護，並以保留的詳情元件測試。有限 Linux Tauri 實際滿高閱讀、詳情、書籤寫入與下一集證據。

第三階段未合併 main／未發版，Windows 等未驗項與原 PNG 歸檔外部限制明列於第三階段驗證／交辦；產品 ad0e366cb0d7c4b583968243f8d864d58fce0d3e，後續文件提交固定對應。

## 2026-10-04 第三階段 T1／T2／T3 修正

接手基準 421d2acea22fccb7f2822848c80ac5a2a265270e，工作區乾淨，保留兩份原報告。修正 Windows 快照 sync handle、DDL 假通過防護、自動 JSON 登記／成功時間／重啟重試、已刪檔 manifest 清理，以及 >60 本／系列與窄視窗返回位置。產品 45568a804b1228c9016f9a7fd2bd876218673b2e；無 schema 遷移、無用戶資料操作、無主線合併或發布。Linux npm21／Rust46、build、diff check、四支 browser script 通過，完整輸出与待 Windows 複驗見 third-phase-review-fix-validation.md／handoff.md。

## 2026-10-04 T2-R1 連續恢復失敗補強

接手 52e30f5865a0596514f5fa9e03187c3f1e28a3cf；工作區乾淨、保留三份原第三階段審查。先以新回歸在原產品重現過期錯誤（exit101），再補恢復／清理成功的錯誤清除條件，共用現有階段文案常數。新增停用與 24h gate 內連續失敗恢復，以及未恢復建立錯誤不得清除的兩項測試。產品 0b883c85e77ea8046fa851464c80a86fbbb7c513；Linux48／npm21、build／diff 通過，完整輸出與待 Windows 項目見 third-phase-t2-r1-handoff.md。未碰使用者 DB，無 schema／新功能、無主線合併或發布。

## 2026-10-04 T2-UI1 面板錯誤單一來源

接手 1d62ec7f5203e32a533ff926d89c1cf3fb16820b，工作區乾淨。原四份報告保留；新報告確認 T2-R1 後端結案，面板錯誤另留副本。真實 Vue／Pinia／App 排程重現成功恢復仍 alert（預期 exit1）；僅修 BackupPanel automatic() 不重複 throw store 的錯誤，成功通知加空錯誤條件。八項新元件回歸、既有書庫／第三階段整合、Linux48／npm21／build／diff 通過。產品 T2_UI1_PRODUCT_SHA_PENDING；完整輸出與未執行項目見 third-phase-t2-ui1-handoff.md。無用戶 DB、schema、主線合併或發布。
