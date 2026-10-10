// 產生 MangaFolio 全套應用圖標 — 去背源圖版（比照 Amagi Core：乾淨源圖 + 單次縮放）
import sharp from 'sharp';
import pngToIco from 'png-to-ico';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const ICONS = join(root, 'src-tauri/icons');
// 母圖保存在倉庫；可用 ICON_SRC 覆寫以預覽其他圖示。
const RAW = process.env.ICON_SRC || join(root, 'docs/images/branding/app-icon.png');

// 裁掉透明邊 -> 主體貼齊（放大效果）。保持原解析度，不做中間縮放。
let trimmed;
try {
  trimmed = await sharp(RAW).trim({ threshold: 10 }).toBuffer();
  const m = await sharp(trimmed).metadata();
  console.log(`trim(透明邊): 主體裁齊 -> ${m.width}x${m.height}`);
} catch (e) {
  console.log('trim 失敗，改用原圖:', e.message);
  trimmed = await sharp(RAW).toBuffer();
}

// 每尺寸從主體單次縮放到目標（contain=完整不裁切、透明底、放大填滿）；小尺寸邊緣銳化
const SHARPEN_MAX = 64;
const pngBuffer = async (size) => {
  let p = sharp(trimmed).resize(size, size, {
    fit: 'contain',
    background: { r: 0, g: 0, b: 0, alpha: 0 },
  });
  if (size <= SHARPEN_MAX) p = p.sharpen({ sigma: 1, m1: 0, m2: 2 });
  return p.ensureAlpha().png().toBuffer();
};

const pngTargets = {
  '32x32.png': 32, '64x64.png': 64, '128x128.png': 128, '128x128@2x.png': 256, 'icon.png': 512,
  'Square30x30Logo.png': 30, 'Square44x44Logo.png': 44, 'Square71x71Logo.png': 71,
  'Square89x89Logo.png': 89, 'Square107x107Logo.png': 107, 'Square142x142Logo.png': 142,
  'Square150x150Logo.png': 150, 'Square284x284Logo.png': 284, 'Square310x310Logo.png': 310,
  'StoreLogo.png': 50,
};
for (const [name, size] of Object.entries(pngTargets)) {
  writeFileSync(join(ICONS, name), await pngBuffer(size));
  console.log(`PNG  ${name.padEnd(22)} ${String(size).padStart(4)}px`);
}

// Tauri Windows以ICO第一幀作執行中視窗圖示；32px避免16px被放大。
const ICO_SIZES = [32, 16, 24, 48, 64, 128, 256];
const icoBuffers = await Promise.all(ICO_SIZES.map(pngBuffer));
writeFileSync(join(ICONS, 'icon.ico'), await pngToIco(icoBuffers));
console.log(`ICO  icon.ico              [${ICO_SIZES.join(',')}]`);
// Windows標題列採透明書本；工作列與安裝檔維持完整綠底圖示。
const titleSource = join(root, 'docs/images/branding/title-icon.png');
writeFileSync(join(ICONS, 'title-bar.png'), await sharp(titleSource)
  .trim({ threshold: 10 }).resize(32, 32, {
    fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 },
  }).sharpen({ sigma: 1, m1: 0, m2: 2 }).ensureAlpha().png().toBuffer());
console.log('PNG  title-bar.png          32px（透明標題列）');
console.log('\n完成。');
