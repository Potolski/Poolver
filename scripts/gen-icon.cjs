const sharp = require('sharp');
const fs = require('fs');
const path = require('path');

// Recreate the icon at a proper hackathon-submission size (1024×1024).
// Submission tools usually want at least 512px and prefer square. We'll
// also export a 512 and 256 for flexibility. Background uses Poolver's
// dark accent (#0d0d0f) so the icon doesn't look anemic on white grids.

const VIEWBOX = 1024;
const PADDING = 64; // breathing room around the rings
const ICON_SVG = `<svg width="${VIEWBOX}" height="${VIEWBOX}" viewBox="0 0 ${VIEWBOX} ${VIEWBOX}" xmlns="http://www.w3.org/2000/svg">
  <rect width="${VIEWBOX}" height="${VIEWBOX}" rx="160" fill="#0d0d0f"/>
  <g transform="translate(${VIEWBOX/2}, ${VIEWBOX/2}) scale(${(VIEWBOX - PADDING * 2) / 32}) translate(-16, -16)">
    <circle cx="12" cy="16" r="8.5" stroke="#ebf3f9" stroke-width="0.8" fill="none"/>
    <circle cx="20" cy="16" r="8.5" stroke="#00c4ff" stroke-width="0.8" fill="none"/>
    <circle cx="16" cy="16" r="1.8" fill="#00c4ff"/>
  </g>
</svg>`;

const outDir = path.resolve('docs/assets');
fs.mkdirSync(outDir, { recursive: true });

(async () => {
  const buf = Buffer.from(ICON_SVG);
  for (const size of [1024, 512, 256]) {
    const out = path.join(outDir, `poolver-icon-${size}.png`);
    await sharp(buf).resize(size, size).png().toFile(out);
    console.log(`wrote ${out}`);
  }
  // Also save the SVG source we used (the master, distinct from the
  // 32×32 favicon at app/src/app/icon.svg).
  fs.writeFileSync(path.join(outDir, 'poolver-icon-master.svg'), ICON_SVG);
  console.log('wrote master SVG');
})();
