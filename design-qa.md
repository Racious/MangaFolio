# MangaFolio 三款新介面 Design QA

- 日期：2026-10-10；實際產品 Vue SFC／CSS，隔離 mock IPC 書庫，不等於原生 WebView2 驗收。
- 正本稽核：vault `projects/mangafolio/reports/2026-10-10-ui-layouts-design-qa.md`；本檔是 Product Design 要求的專案內驗證證據。
- source visual truth：`.amagi/verification/ui-design-20261010/concept-{1,2,3}.png`，依老爺選定全部三款；SHA256 見同目錄 concept-manifest.json。
- implementation：`http://127.0.0.1:1420/.amagi/verification/ui-redesign.html`；真 LibraryView／SettingsDialog／LibraryNavigation／BookDetailsPanel，測試資料與 IPC 替身。
- implementation screenshot：同目錄 `implementation-{workbench,gallery,studio}-final.png`。
- viewport：1487×1058 CSS px；來源與實作 PNG 均1487×1058，1:1，無框架／瀏覽器chrome，未做密度縮放。窄版額外430×900。
- state：workbench 深色推薦／全部作品，gallery 深色推薦／全部作品＋續讀，studio 明亮推薦／全部作品＋星海紀行資訊側欄；均 medium grid、舒適。來源studio是漫畫分頁／雲上之城；資料、頁數、封面与收藏數依測試書庫不同，不以數值差異判定版型漂移。

## 比較證據

- Full-view：2026-10-10 同一次圖片輸入並列三組來源與最終實作，共六圖，逐組目視比較；沒有用分開觀看冒充並列。三款均有單一加入入口、常用搜尋／媒體分頁與封面主體，來源sidebar284、topnav64、rail132／drawer344的區域比例已落地。工作台五欄、藝廊六欄、書架選取後四欄。
- Focused region：來源concept-3原尺寸圖右側x1143–1487與`studio-detail-region.png`（344×1058）在同一次輸入比較。260×390封面、26px標題、閱讀主動作、直接更換封面与編輯可讀；附加來源資訊与書籤收合，保留既有功能。
- 表單細節：`settings-final.png`與`settings-custom-final.png`（後者是較早一輪已保存自訂色的證據）；色碼#ffffff／#000000／#ffcc00在真表單填寫後保存、reload值一致；獨立背景文字與面板文字避免黑白反向時看不清。
- 窄版：`narrow-workbench-final.png`、`narrow-gallery.png`、`narrow-studio-top.png`，430×900，兩欄封面、頂部可見設定與媒體類型。工作台document scrollWidth430，不水平溢位，設定按鈕top12／bottom59.33。

## 五個必要表面

| 表面 | 檢查與結論 |
| --- | --- |
| 字體與階層 | Windows Segoe UI／Microsoft JhengHei；藝廊標題Noto Serif TC／PMingLiU／serif回退44px，工作台40px、書架34px、卡片15px。來源為生成圖、無可取得字體檔，系統中文字形回退為明列的產品約束。小字12px、焦點与長名截斷已目視。 |
| 間距與布局 | 工作台側欄284、藝廊topnav64／padding52、書架rail132與資訊344；封面2:3，gallery短續讀80px；較原草圖多管理與失效來源入口屬保留現有功能，未擠出主要作品區。 |
| 色彩與tokens | 推薦青玉／暖暮／紙韻；背景、面板、重點色獨立、前景自動推導，灰色fallback用純黑確保4.5文字對比。色票与版型可交叉選擇；未宣稱整站WCAG認證。 |
| 圖片與素材 | forest／city／sea三張真ImageGen PNG1024×1536，僅隔離測試內容，不植入正式書庫。封面用原圖object-fit cover，没有CSS／SVG假畫；與來源插畫主題／書名不同為使用者內容可變差異，圖片清晰，無暫代空白。標準圖示使用Phosphor。 |
| 文字內容 | 繁中、加入作品／設定／漫畫圖集／影片／預設播放器一致；資訊展開細節保留；不加入串流、帳號、影片進度或未提供功能。主畫面無技術實作說明。 |

## 比較歷史與修正

| 輪次／程度 | 先前差異／影響 | 修正與修後證據 |
| --- | --- | --- |
| R1 P1 | 驗測fixture缺App flex容器，gallery首屏截掉／底部設定離開畫面。不是原生產品bug。 | fixture#app flex＋100vh；gallery-r2与三款final完整。 |
| R1 P2 | 工作台sidebar204與來源284差太多、heading弱。 | sidebar284／title40／brand26；workbench-final。 |
| R1 P2 | 藝廊續讀與重复計數太高，封面首屏密度偏低。 | 短續讀80、標題與數量同行、重複results隱藏／六欄；gallery-final。 |
| R1 P2 | 書架資訊160px封面太小、附加資訊過密；名稱資訊按鈕中心被閱讀封面搶點。 | compact drawer344／260px cover，details收合、名稱點擊區限底部80px；studio-final／detail-region。 |
| R2 P1 | 窄工作台側欄隱藏媒體且主區未顯示媒體切換。 | narrow時主區媒體切換；narrow-workbench-final與真SFC recovery test。 |
| R2 P2 | 自訂文字色碼只change儲存，填寫未立即保存；背景白面板黑時透明控制文字不清。 | 有效@input保存、非法@change復原；背景／面板token分開；custom-colors真SFC與settings-custom-final reload。 |
| R2 P2 | 短設定頁導致視窗上下跳；Guide highlight透傳導致help進入就關閉。 | dialog固定可視高度、內頁scroll；Guide embedded不透傳；最後browser AX仍顯示設定／使用說明，真Guide SFC測試。 |

## 互動、驗證與限制

- 已實作且前端真SFC／store測試：六款切換、原偏好解析、獨立色票、合法色碼／錯誤復原、來源忘記成功／失敗、pending關閉保護、Esc與焦點返回、query／selection保留、導航、封面直接換成功／取消／失敗、既有媒體／備份互動。
- 瀏覽器已操作：三款style、明暗、五設定分頁、窄版媒體按鈕、名稱開資訊、封面替換回饋、色碼輸入＋reload、加入與篩選menu、搜尋／媒體／tag／status、關閉設定回原位置。
- `npm.cmd test`44／44，0 fail，6313.352ms；`npm.cmd run build`vue-tsc＋Vite1624modules，2.78s，exit0。完整輸出同目錄npm-{test,build}-final.log。
- `browser-console-final.json`為warn／error查詢結果[]。介面資料是mock，不把它當Tauri IPC／播放器／備份原生成功證據；原生驗測另依guided-smoke-test進行。
- 本輪Rust沒有新程式diff；影音前輪59／59證據沿用。Claude獨立審查尚待，不將作者QA当交叉審查。
- 剩餘P3：生成圖與Windows字體的光學差異；實際收藏數／封面內容不同；是明列約束，沒有待修P0／P1／P2。

## Implementation Checklist

- [x] 同尺寸full-view與資訊focused-region比較。
- [x] 五個必要表面與每項P1／P2修正留證據。
- [x] 三款結構差異、獨立自訂色與集中設定。
- [x] 前端測試與建置、console、窄版媒體入口。
- [ ] 原生WebView2／預設播放器／讀屏等使用者實機驗收，另案進行，不阻擋本次瀏覽器視覺比對。

final result: passed
