// Run the web_typography Rust example first, then pass its output directory.
// Requires Playwright and Chromium (or installed Chrome).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { resolve } = require('node:path');
const { pathToFileURL } = require('node:url');

(async () => {
  const directory = resolve(process.argv[2]);
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    for (const [width, font, dpr] of [[1100, 16, 1], [390, 16, 2], [320, 20, 1]]) {
      const context = await browser.newContext({ viewport: { width, height: 700 }, deviceScaleFactor: dpr });
      const page = await context.newPage();
      await page.goto(pathToFileURL(resolve(directory, 'index.html')).href);
      await page.evaluate(size => { document.body.style.fontSize = `${size}px`; }, font);
      const dimensions = await page.evaluate(async () => {
        const image = [...document.images].find(img => img.getBoundingClientRect().width > 0);
        await image.decode();
        const wrapper = image.parentElement;
        return { width: image.getBoundingClientRect().width, src: image.getAttribute('src'),
          loaded: image.naturalWidth > 0, overflow: document.documentElement.scrollWidth > innerWidth,
          scrollWidth: wrapper.scrollWidth, clientWidth: wrapper.clientWidth };
      });
      const nominal = width > 600 ? 720 : 340;
      assert.equal(dimensions.src, width > 600 ? 'desktop.svg' : 'mobile.svg');
      assert.ok(Math.abs(dimensions.width - nominal * font / 16) < 0.1);
      assert.ok(dimensions.loaded);
      assert.equal(dimensions.overflow, false);
      if (width === 320) assert.ok(dimensions.scrollWidth > dimensions.clientWidth);
      await page.screenshot({ path: resolve(directory, `checked-${width}-${font}-${dpr}.png`), fullPage: true });
      console.log({ viewport: width, font, dpr, ...dimensions });
      await context.close();
    }
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
