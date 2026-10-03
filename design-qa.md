# MangaFolio 視覺與操作自檢

2026-10-03 UTC。這是開發者設計 QA，獨立 Code Review 與 Windows 原生驗收仍待進行。

## 比較材料

參考：`docs/images/next-phase/directions/01-calm-bookshelf.png`、`02-catalog-workbench.png`、`03-night-reading.png`，三張使用相同四本樣本，使用者選擇全數實作，之後補充要求容易擴充改色。

實作：`docs/images/next-phase/implemented/`（Chromium、1440×1000、同四本樣本；mock IPC），及 `native-implemented/`（真實 Linux Tauri／WebKit）。已開啟並視覺檢查三風格主要畫面、窄視窗、編輯 dialog、管理、閱讀與返回。參考圖為設計示意，含裝飾漫畫封面；產品封面來自使用者來源，驗證用隔離圖片，未將設計稿當可點擊頁面。

## 判斷與修正

- 三種方向以暖白茶綠、冷白藍及炭灰暖金呈現，共用導覽／工具區、封面／列表與狀態。來源封面是內容，不固化示意漫畫或作者功能。
- 初版列表資訊堆疊過高；調整詳細列表為欄位配置、緊湊列表減少重複頁碼行，仍保留狀態百分比、系列／集數、標籤、收藏、編輯與失效說明。
- 初版窄視窗教學佔用首屏；改成預設收合的短入口。導覽、標籤及外觀可收合，主要加入／管理／搜尋／檢視保持可用。
- 初版控制邊界對比較低；集中修正 theme token，36 色對驗證通過文字 4.5:1／邊界 3:1。關鍵狀態另以文字呈現。焦點可見，資訊編輯使用原生 dialog。
- 18 種風格／明暗／檢視組合共用資料，保留搜尋／排序／選取；重新載入恢復外觀，窄視窗無水平溢出，長書名及長標籤不破壞容器。
- 業務元件消費語意 token，風格 registry／theme 檔集中，增加風格或改色不需要複製檢視或資料操作。

## 尚待外部驗收

Windows WebView2、中文原生 IME、螢幕閱讀器、高 DPI、所有原生選檔流程及 UNC 大型書庫，見 `docs/next-phase-validation.md`。這些不在本次 Linux／Chromium 視覺自檢中宣告通過。未宣告完整 WCAG 合規。

final result: passed

此結果僅指本次已擷取畫面及瀏覽器互動的設計自檢；獨立审查結果應寫入 `docs/next-phase-review-report.md`。
