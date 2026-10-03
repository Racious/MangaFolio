# 書庫與教學驗證紀錄

更新日期：2026-10-03（Asia/Tokyo）。本文件分階段整理已執行的結果；歷史驗證保留各自基準，最新管理功能驗證如下。

## 第二輪管理與備份驗證

程式基準：`df6c134`（包含初版與教學）。

| 項目 | 結果 |
| --- | --- |
| `npm run build` | TypeScript 與 Vite 成功。 |
| `npm test` | 3 passed、0 failed。 |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | 18 passed、0 failed；原有 11 項與新增 7 項。 |
| `cargo build --locked --manifest-path src-tauri/Cargo.toml` | 原生 debug build 成功。 |
| Rustfmt／`git diff --check` | 修改的 Rust 格式及 diff 檢查通過。 |
| Chromium／Playwright mock IPC smoke | 批次選取與收藏、篩選清除選取、取消／失敗／成功移除、目前閱讀先 flush 後 discard、重新指定來源、匯出、失敗／成功還原、640px 寬度通過。腳本已納入 repo。 |
| Linux 原生 Tauri | 教學與管理畫面正常；批次收藏／取消收藏後確認 SQLite 值；取消移除保留兩本，操作中關閉會保留視窗；確認移除後資料列清空且原始 CBZ 仍在，完成後可正常關閉。使用隔離測試資料目錄，不修改正式書庫。 |
| 原生匯出／還原／重新指定來源全流程 | 未完成。匯出選檔時 PRoot／GTK 顯示目錄 `Bad address`，取消後程式可繼續操作；設定 PROOT_NO_SECCOMP=1、GIO_USE_VFS=local 重試仍失敗。尚未判定為產品缺陷，需本地重驗。 |

新增的 7 個 Rust 測試覆蓋：

- 批次收藏／移除包含不存在 ID 時回滾、重複 ID 拒絕、原始來源保留及自有封面清理。
- 來源搬移及頁面重排後保留 ID、收藏、偏好、最近閱讀時間並依檔名定位。
- 重新指定到既有來源時拒絕並保留兩本資料。
- 備份 roundtrip、相同來源略過保留現況、不同 ID 重建與離線 metadata 持久化。
- 損壞／超大備份或無效設定拒絕，沒有部分還原或修改現有資料。
- 較新備份版本、重複來源、相對路徑拒絕。
- 匯出有效備份，不覆寫既有備份或漫畫來源。

新增原生畫面見 [管理介面截圖](images/library-management.png)。原生選檔限制是待補驗項目，不會用 mock 結果替代。詳細驗收與報告要求見 [審查交辦單](review-assignment.md)。

## 管理介面 smoke 重跑方式

此項為可選的開發檢查。使用 Python 虛擬環境，在一個終端執行 `npm run dev`，另一個終端執行：

```bash
python -m pip install playwright
python -m playwright install chromium
python scripts/smoke-library-management.py
```

若系統已有 Chromium，可用 `CHROMIUM_EXECUTABLE` 指定實際路徑，省略 Playwright 的瀏覽器下載。Linux 雲端驗證使用 `/usr/bin/chromium`。選配 `SMOKE_SCREENSHOT_PATH` 可保存介面截圖；父目錄需已存在。

腳本在獨立瀏覽器 context 中使用記憶體測試書庫，連接 Vite 預設 port 1420 並模擬 IPC，不改實際 SQLite 或漫畫。Python／Playwright 沒有加入 npm 或 Rust 依賴。

## 第一輪歷史驗證

| 範圍 | 驗證基準 | 結果與限制 |
| --- | --- | --- |
| 前端建置 | 核心功能 `06469a6`；教學 `7fe4acb` | `npm run build` 成功，TypeScript 與 Vite 通過。 |
| 前端邏輯測試 | 核心及教學提交 | `npm test`：3 passed、0 failed。 |
| Rust 測試 | 核心功能 `06469a6` | `cargo test --locked --manifest-path src-tauri/Cargo.toml`：11 passed、0 failed。教學提交未修改 Rust，因此未重跑。main 與 doc-tests 的零測試不另計為通過案例。 |
| 原生桌面實測 | 核心功能；Linux、Xvfb、Tauri WebKitGTK | 加入兩本 CBZ、封面、收藏、搜尋、最近閱讀、續讀、閱讀設定與 640×480 工具列均確認。未代表 Windows 驗證。 |
| 教學介面操作 | 教學提交；Chromium、Playwright | 五步切換、區域框線、reload 後步驟保存、完成／收合／重開與 640px 寬度無橫向溢出通過。使用模擬 Tauri IPC，未測原生 IPC 或桌面 WebView 的教學操作。 |
| Git 推送 | `06469a6`、`7fe4acb` | 推送到功能分支成功，透過 `git ls-remote` 比對遠端 hash 與提交。 |

## 自動測試覆蓋

前端 3 項：

1. 中文書名、全形／半形、大小寫與空白搜尋。
2. 收藏／最近閱讀與搜尋交集；未讀與讀到第一頁的區別。
3. 自然書名排序、最近閱讀排序，不修改原始陣列順序。

Rust 11 項：

- 原有自然排序 3 項：數字順序、前導零、混合文字。
- 書庫 7 項：重啟持久化、重複加入保留狀態且不標成最近閱讀、頁面名稱還原與頁碼範圍、拒絕非法進度且不覆寫良好資料、來源消失保留資料、真實 PNG 封面快取、拒絕新版資料庫且不重設。
- session 1 項：舊 session 無法使用新書籍或其快取圖片。

## 原生操作證據

- 加入兩個測試 CBZ，原始檔案留在原位；紅綠封面與對應內容一致。
- 收藏與書名搜尋、最近閱讀篩選符合操作結果。
- 翻到第 2 頁後立即發送正常視窗關閉事件，在 400ms 延遲儲存前關閉，確認仍保存 index 1 與頁面名稱。
- 重啟後首頁顯示第 2 頁續讀；點入後確認是對應的第 2 頁圖片。
- 切換閱讀方向後回書庫，確認資料庫保存設定。
- 在 640×480 視窗確認工具列可完整操作。

[書庫實測截圖](images/library-preview.png) 已納入 Git，使用測試封面。額外截圖與原始 Rust 輸出保留在該次雲端工作區，見 [交接](handoff.md#雲端環境交接)。教學 smoke script 當時位於 `/tmp/mangafolio-guide-smoke.py`，未作為 repo 測試納入；後續可依下列清單手動重驗。

## 重驗命令

安裝依賴與平台工具後，在專案根目錄執行：

```bash
npm ci
npm test
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

在本次保留的雲端環境可先 source `/workspace/.mangafolio-tools/env.sh`；一般本地電腦不使用這個路徑。命令列成功仍需接著做原生操作驗收。

## 人工驗收清單

- [ ] 首次進入顯示教學，切換五步及對應框線；收合、重開、離開書庫返回、重啟後狀態合理。
- [ ] 加入 ZIP／CBZ／圖片資料夾；取消選取、重複加入、來源不可用與錯誤提示合理。
- [ ] 收藏／取消收藏在書庫及閱讀工具列同步，搭配搜尋與篩選正確。
- [ ] 翻頁後立即切書、返回書庫、正常關閉；重啟後每本書分別恢復位置與設定。
- [ ] 儲存失敗顯示錯誤，能重試；不要以強制結束驗證正常關閉的承諾。
- [ ] 快速翻頁／切書不混入別本圖片；視窗縮至 640×480，控制項仍可操作。
- [ ] Windows 目標環境重跑上列流程，再驗安裝、升級與解除安裝。

以上是待重驗清單，不是對所有案例已測試的宣告。

## 尚未完成的審核與測試

獨立 Code Review、安全審查、Windows 原生教學與閱讀驗收、MSI／NSIS 打包及簽章、正式更新端到端驗證、大型書庫效能測試皆尚未完成。測試不是對全部壓縮檔變體、來源變動或所有資料庫故障的完整覆蓋。
