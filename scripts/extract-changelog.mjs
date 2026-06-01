/**
 * extract-changelog.mjs
 * CI 工具：從 CHANGELOG.md 提取指定版本的更新說明，
 * 合併下載表格後輸出至 release-body.md
 *
 * Usage: node scripts/extract-changelog.mjs <version>
 * Example: node scripts/extract-changelog.mjs v0.1.0
 */

import { readFileSync, writeFileSync } from 'fs'

const rawVersion = process.argv[2] ?? ''
const version = rawVersion.replace(/^v/, '')

const downloadTable = `## 下載

| 類型 | 說明 | 檔案 |
|------|------|------|
| 安裝版（MSI，推薦） | Windows Installer，自動整合捷徑與解除安裝 | \`MangaFolio_${version}_x64_en-US.msi\` |
| 安裝版（NSIS） | 輕量安裝程式 | \`MangaFolio_${version}_x64-setup.exe\` |
| 攜帶版（免安裝） | 單一 .exe，無需安裝，直接執行 | \`MangaFolio_${version}_portable.exe\` |

> MangaFolio — 跨平台、高效能、忠於傳統閱讀體驗的本地漫畫／圖集閱讀器

---

`

let changelog = ''
try {
  changelog = readFileSync('CHANGELOG.md', 'utf-8')
} catch {
  writeFileSync('release-body.md', downloadTable)
  process.exit(0)
}

const versionHeader = `## v${version}`
const start = changelog.indexOf(versionHeader)

if (start === -1) {
  writeFileSync('release-body.md', downloadTable)
  process.exit(0)
}

const afterHeader = start + versionHeader.length
const nextVersion = changelog.indexOf('\n## v', afterHeader)
const versionSection = nextVersion === -1
  ? changelog.slice(start)
  : changelog.slice(start, nextVersion)

writeFileSync('release-body.md', downloadTable + versionSection.trim() + '\n')
console.log(`Release body generated for v${version}`)
