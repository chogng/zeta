// Run with playwright-cli run-code --filename after opening a local HTTP root
// containing manifest.json, the ANSI fixtures, xterm.js and xterm.css.
async page => {
  await page.setContent('<link rel="stylesheet" href="xterm.css"><div id="terminal"></div>');
  await page.addScriptTag({ url: 'xterm.js' });
  const cases = await page.evaluate(async () => (await fetch('manifest.json')).json());
  const results = [];
  for (const fixture of cases) {
    await page.evaluate(async fixture => {
      window.term?.dispose();
      window.term = new Terminal({ cols: fixture.width, rows: fixture.height, scrollback: 10000 });
      term.open(document.getElementById('terminal'));
      window.input = [];
      term.onData(data => input.push(data));
      await new Promise(resolve => term.write('PREEXISTING-SHELL\r\n', resolve));
      const bytes = new Uint8Array(await (await fetch(fixture.file)).arrayBuffer());
      await new Promise(resolve => term.write(bytes, resolve));
    }, fixture);
    const verify = () => page.evaluate(fixture => {
      if (term.buffer.active.type !== 'normal') throw Error('Alternate buffer activated');
      const buffer = term.buffer.active;
      const text = Array.from({ length: buffer.length }, (_, i) => buffer.getLine(i).translateToString(true)).join('').replace(/\s/g, '');
      let previous = -1;
      for (const marker of ['PREEXISTING-SHELL', 'ZETA-SHELL-SENTINEL', ...fixture.markers]) {
        if (text.split(marker).length !== 2) throw Error(`${fixture.file}: ${marker} missing or duplicated`);
        const position = text.indexOf(marker);
        if (position <= previous) throw Error(`${marker} out of order`);
        previous = position;
      }
      return { historyRows: buffer.baseY };
    }, fixture);
    const before = await verify();
    await page.evaluate(() => term.resize(25, 8));
    const after = await verify();
    await page.locator('.xterm-screen').hover();
    await page.mouse.wheel(0, -500);
    await page.waitForFunction(() => term.buffer.active.viewportY < term.buffer.active.baseY);
    const input = await page.evaluate(() => window.input);
    if (input.length) throw Error(`Wheel generated panel input: ${JSON.stringify(input)}`);
    results.push({ case: fixture.file, before, after, wheel: 'terminal scrollback only' });
  }
  await page.evaluate(results => { window.historyResults = results; }, results);
  return results;
}
