// 產生 MangaFolio 全套應用圖標 — 去背源圖版（比照 Amagi Core：乾淨源圖 + 單次縮放）
import sharp from 'sharp';
import pngToIco from 'png-to-ico';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const ICONS = join(root, 'src-tauri/icons');
// 去背源圖（透明底、主體）
const RAW = process.env.ICON_SRC || 'C:/Users/Racious/Downloads/ChatGPT_Image_2026年7月5日_上午03_39_00-removebg-preview.png';

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
  '32x32.png': 32, '128x128.png': 128, '128x128@2x.png': 256, 'icon.png': 512,
  'Square30x30Logo.png': 30, 'Square44x44Logo.png': 44, 'Square71x71Logo.png': 71,
  'Square89x89Logo.png': 89, 'Square107x107Logo.png': 107, 'Square142x142Logo.png': 142,
  'Square150x150Logo.png': 150, 'Square284x284Logo.png': 284, 'Square310x310Logo.png': 310,
  'StoreLogo.png': 50,
};
for (const [name, size] of Object.entries(pngTargets)) {
  writeFileSync(join(ICONS, name), await pngBuffer(size));
  console.log(`PNG  ${name.padEnd(22)} ${String(size).padStart(4)}px`);
}

const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];
const icoBuffers = await Promise.all(ICO_SIZES.map(pngBuffer));
writeFileSync(join(ICONS, 'icon.ico'), await pngToIco(icoBuffers));
console.log(`ICO  icon.ico              [${ICO_SIZES.join(',')}]`);
console.log('\n完成。');
