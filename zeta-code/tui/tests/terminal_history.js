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
      // Match VS Code: ED 2 archives the visible screen instead of erasing it in place.
      window.term = new Terminal({ cols: fixture.width, rows: fixture.height, scrollback: 10000, scrollOnEraseInDisplay: true });
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
        const count = text.split(marker).length - 1;
        if (count !== 1) throw Error(`${fixture.file}: ${marker} expected once, got ${count}`);
        const position = text.indexOf(marker);
        if (position <= previous) throw Error(`${marker} out of order`);
        previous = position;
      }
      for (const marker of ['ZETA-TRANSIENT-WELCOME', 'ZETA-TRANSIENT-COMPLETION', 'ZETA-TRANSIENT-INPUT']) {
        if (text.includes(marker)) throw Error(`${fixture.file}: transient frame leaked into scrollback: ${marker}`);
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
  const panelResults = await page.evaluate(async () => {
    const steps = await (await fetch('status-repaint.json')).json();
    window.term?.dispose();
    window.term = new Terminal({ cols: 80, rows: 40, scrollback: 10000, scrollOnEraseInDisplay: true });
    term.open(document.getElementById('terminal'));
    const results = [];
    for (const step of steps) {
      term.resize(step.width, step.height);
      await new Promise(resolve => term.write(step.output, resolve));
      const buffer = term.buffer.active;
      const text = Array.from({ length: buffer.length }, (_, i) => buffer.getLine(i).translateToString(true)).join('\n');
      const count = text.split('ZETA-STATUS-SESSION').length - 1;
      const expected = step.stage === 'closed' ? 0 : 1;
      if (count !== expected) throw Error(`${step.stage}: expected ${expected} Status panels, got ${count}`);
      if (text.split('ZETA-SHELL-SENTINEL').length !== 2) throw Error(`${step.stage}: shell history changed`);
      const history = Array.from({ length: buffer.baseY }, (_, i) => buffer.getLine(i).translateToString(true)).join('\n');
      if (history.includes('ZETA-STATUS-SESSION')) throw Error(`${step.stage}: Status entered scrollback`);
      results.push({ stage: step.stage, panels: count });
    }
    return results;
  });
  results.push({ statusRepaint: panelResults });
  await page.evaluate(results => { window.historyResults = results; }, results);
  return results;
}
