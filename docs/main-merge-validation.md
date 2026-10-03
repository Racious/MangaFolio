# 主線合併驗證

2026-10-04（Asia/Tokyo）。使用者明確授權將目前功能分支合併主線，要求保留 merge commit。

- 主線父提交：`d35af0027dd0b08eac84f520ceed797c1c3f2b40`。
- 功能分支父提交：`e2a87d626f420c004a5e53be188ee239c32f3943`。
- 使用 `git merge --no-ff --no-commit feature/library-management-ui`，合併無衝突；驗證後建立獨立 merge commit，其 SHA 與兩個父提交以 Git 紀錄為準。
- 本次包含已提交的書庫介面精修與沉浸閱讀；實際画面及範圍見 [預覽](ui-refinement-preview.md)。

合併工作區隔離於 `/workspace/MangaFolio-main-merge`，原功能工作區保留。測試來源／資料庫隔離，未操作使用者實際資料庫。

本次合併結果重新執行 npm 測試（16 項）、前端建置、Rust locked 測試（37 項）與 staged diff whitespace 檢查，全部通過，退出碼均為 0；完整輸出與退出碼見 [驗證紀錄](next-phase-results/main-merge/results.json)。

Windows 本次介面及閱讀器原生驗收仍未執行，不能將歷史審查通過視為最新修改已通過 Windows。此次只合併與推送，不進行發版、簽章或发布。
