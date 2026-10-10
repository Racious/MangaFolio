# MangaFolio 應用圖示

母圖 `app-icon.png` 採用薄荷綠書本造型；圓角外側透明，綠色底保留。

重新產生：

1. 執行 `npm run tauri -- icon docs/images/branding/app-icon.png --output .amagi/icon-generated`，取得 ICNS。
2. 將 `.amagi/icon-generated/icon.icns` 複製到 `src-tauri/icons/icon.icns`。
3. 執行 `node scripts/gen-icons.mjs`，產生 PNG 與七尺寸 Windows ICO。
4. 將 `src-tauri/icons/32x32.png` 複製到 `public/favicon.png`。

所有尺寸由母圖直接縮小；ICO 包含 16、24、32、48、64、128、256px。

Windows ICO 以 32px 幀排第一：目前 Tauri 會取第一幀作為執行中視窗圖示，避免 16px 圖被放大。其餘尺寸保留供檔案總管使用。


標題列另用透明母圖 `title-icon.png`，由 gen-icons 產生 32px `title-bar.png`。Windows 啟動時先從執行檔的圖示資源載入綠底版本並設定工作列 Big 圖示，再以 Tauri set_icon 換成透明標題列 Small 圖示，避免工作列借用 Small 圖示。安裝檔仍用綠底版本；載入失敗僅記錄錯誤，不阻擋書庫啟動。
