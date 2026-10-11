// Dependency-free UI behavior tests. The VM runs the actual served application code.
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const source = fs.readFileSync(path.join(__dirname, '../static/app.js'), 'utf8');

function setup({ charts = false, fetch } = {}) {
  const elements = new Map(), alerts = [], requests = [], chartConfigs = [], events = {};
  let poll;
  function element(id) {
    if (!elements.has(id)) {
      const classes = new Set();
      elements.set(id, {
        value: '', textContent: '', innerHTML: '', disabled: false, checked: false,
        style: {}, dataset: {}, children: [], events: {},
        classList: { add: (...names) => names.forEach(n => classes.add(n)),
          remove: (...names) => names.forEach(n => classes.delete(n)), contains: n => classes.has(n) },
        addEventListener(name, fn) { this.events[name] = fn; },
        appendChild(child) { this.children.push(child); },
        querySelector() { return element(id + '-badge'); },
        getContext() { return {}; },
      });
    }
    return elements.get(id);
  }
  const modes = ['fixed_rounds', 'target_profit', 'combined', 'till_bust'];
  const buttons = modes.map(mode => { const b = element(mode); b.dataset.mode = mode; return b; });
  const filters = ['all', ...modes].map(mode => {
    const b = element('filter-' + mode); b.dataset.modeFilter = mode; return b;
  });
  for (const [id, value] of Object.entries({ selectedMode: 'fixed_rounds', runs: '8',
    start_capital: '100', min_bet: '5', max_bet: '500', rounds_per_run: '3',
    target_profit: '15', roulette_type: 'european' })) element(id).value = value;
  class Chart {
    static defaults = { font: {} };
    constructor(ctx, config) { chartConfigs.push(config); }
    destroy() {}
  }
  const context = vm.createContext({
    document: { getElementById: element, createElement: () => element('row-' + elements.size),
      querySelectorAll: selector => selector === '.mode-btn' ? buttons :
        selector === '.history-filter-btn' ? filters : [] },
    window: { location: {}, addEventListener: (name, fn) => { events[name] = fn; } },
    console, alert: message => alerts.push(message), confirm: () => true,
    setInterval: fn => { poll = fn; return 1; }, clearInterval: () => {},
    fetch: async (url, options) => { requests.push({ url, options }); return fetch ?
      fetch(url, options) : { ok: true, json: async () => [], text: async () => '' }; },
    ...(charts ? { Chart } : {}),
  });
  vm.runInContext(source, context);
  return { element, alerts, requests, events, chartConfigs,
    evaluate: code => vm.runInContext(code, context),
    submit: () => element('simForm').events.submit({ preventDefault() {} }),
    poll: () => poll() };
}
function result(mode = 'fixed_rounds') {
  return {
    mode, params: { start_capital: 100, rounds_per_run: 3, unlimited_max_bet: false },
    total_runs: 8, failed_runs: 2, failed_percentage: 25, bankrupt_runs: 1,
    bankrupt_percentage: 12.5, target_reached_runs: 0, target_reached_percentage: 0,
    round_limit_percentage: 75, hit_safety_cap_runs: 2, hit_safety_cap_percentage: 25,
    avg_rounds_played: 3, median_rounds_played: 3, min_rounds_played: 1, max_rounds_played: 3,
    p10_rounds: 1, p90_rounds: 3, avg_result: 105, avg_profit: 5,
    median_result: 110, median_profit: 10, max_capital_overall: 120, min_result: 0,
    max_consecutive_losses: 2, max_consecutive_wins: 3, duration_ms: 5,
    distribution: [{ label: '0 € (Zero bankroll)', min_val: 0, count: 1, percentage: 12.5 }],
    rounds_distribution: [{ label: '3 spins', count: 8, percentage: 100 }],
    timeline_percentiles: [{ round: 0, mean: 100, median: 100, p10: 100, p25: 100, p75: 100, p90: 100 }],
    sample_trajectories: [], statistics_sample_size: 8, timeline_sample_size: 8,
  };
}
test('loading an empty history never starts a simulation automatically', async () => {
  const ui = setup(); await ui.events.DOMContentLoaded();
  assert.deepEqual(ui.requests.map(r => r.url), ['/api/history']);
  assert.match(ui.element('chartsSubheader').textContent, /Charts unavailable/);
});
test('mode selector exposes the matching fields', () => {
  const ui = setup();
  ui.evaluate("setSimulationMode('combined')");
  assert.equal(ui.element('selectedMode').value, 'combined');
  assert.equal(ui.element('targetProfitContainer').classList.contains('hidden'), false);
  ui.evaluate("setSimulationMode('till_bust')");
  assert.equal(ui.element('roundsFieldContainer').classList.contains('hidden'), true);
  assert.equal(ui.element('targetProfitContainer').classList.contains('hidden'), true);
  assert.equal(ui.element('tillBustInfoNotice').classList.contains('hidden'), false);
});
test('integer parsing rejects fractions and preserves exponent notation', () => {
  const ui = setup();
  assert.equal(ui.evaluate("parseCleanInt('1.5', 0)"), 0);
  assert.equal(ui.evaluate("parseCleanInt('1e3', 0)"), 1000);
  assert.equal(ui.evaluate("parseCleanFloat('5.25', 0)"), 5.25);
  assert.equal(ui.evaluate("parseCleanFloat('5junk', 0)"), 0);
});
test('invalid runs, spin limits and profit targets never reach the API', async () => {
  for (const [id, value, mode] of [
    ['runs', '1.5', 'fixed_rounds'], ['runs', '1000000001', 'fixed_rounds'],
    ['rounds_per_run', '0', 'fixed_rounds'], ['target_profit', '0', 'target_profit'],
  ]) {
    const ui = setup(); ui.element(id).value = value; ui.evaluate("setSimulationMode('" + mode + "')");
    await ui.submit(); assert.equal(ui.requests.length, 0); assert.equal(ui.alerts.length, 1);
  }
});
test('server failures remain visible and restore the submit button', async () => {
  const ui = setup({ fetch: async () => ({ ok: false, text: async () => 'Disk full' }) });
  await ui.submit();
  assert.match(ui.element('statusText').textContent, /failed.*Disk full/);
  assert.equal(ui.element('submitBtn').disabled, false);
});
test('successful submissions render results and refresh history without Chart.js', async () => {
  const ui = setup({ fetch: async url => ({ ok: true, json: async () => url === '/api/simulate' ? result() : [] }) });
  await ui.submit();
  assert.equal(ui.alerts.length, 0); assert.match(ui.element('card1Value').textContent, /25/);
  assert.match(ui.element('statusText').textContent, /successfully/);
  assert.deepEqual(ui.requests.map(r => r.url), ['/api/simulate', '/api/history']);
});
test('cancellation and asynchronous failures are not displayed as success', async () => {
  for (const progress of [{ is_cancelled: true }, { is_finished: true, error: 'Could not save CSV' }]) {
    const ui = setup({ fetch: async url => ({ ok: true, json: async () => url === '/api/simulate' ?
      { job_id: 'job_test' } : { percentage: 0, completed_runs: 0, total_runs: 8,
        speed_runs_per_sec: 0, estimated_remaining_ms: 0, elapsed_ms: 1, ...progress } }) });
    ui.evaluate("setSimulationMode('till_bust')"); await ui.submit(); await ui.poll();
    assert.doesNotMatch(ui.element('statusText').textContent, /successfully/);
    assert.equal(ui.element('submitBtn').disabled, false);
    const payload = JSON.parse(ui.requests[0].options.body);
    assert.equal(payload.rounds_per_run, 0);
  }
});
test('all result modes render with Chart.js and show sample sizes', () => {
  for (const mode of ['fixed_rounds', 'target_profit', 'combined', 'till_bust']) {
    const ui = setup({ charts: true });
    ui.evaluate('updateUIWithResults(' + JSON.stringify(result(mode)) + ')');
    assert.match(ui.element('chartsSubheader').textContent, /Statistics sample: 8/);
    assert.ok(ui.chartConfigs.length >= 3);
  }
});
test('history shows zero target rates and the actual bust rate instead of hardcoded 100%', () => {
  const ui = setup();
  const row = { ...result('till_bust'), timestamp: '<img src=x onerror=alert(1)>',
    runs: 8, min_bet: 5, max_bet: 500, start_capital: 100, rounds_per_run: 3, max_capital: 120 };
  ui.evaluate('historyData = ' + JSON.stringify([row]) + '; renderHistoryTable();');
  const html = ui.element('historyTableBody').children[0].innerHTML;
  assert.match(html, /12.5% bust/); assert.match(html, /25.0% capped/);
  assert.ok(!html.includes('<img')); assert.ok(html.includes('&lt;img'));
});
test('failed history deletion is visible to the user', async () => {
  const ui = setup({ fetch: async () => ({ ok: false, text: async () => 'Permission denied' }) });
  await ui.element('clearHistoryBtn').events.click();
  assert.match(ui.alerts[0], /Permission denied/);
});

test('served UI text uses English labels throughout', () => {
  const html = fs.readFileSync(path.join(__dirname, '../static/index.html'), 'utf8');
  assert.match(html, /lang="en"/);
  assert.doesNotMatch(html + source, /[äöüÄÖÜß]|\\b(?:Anzahl|Netto|erreicht|Runden|Gewinnziel|Historie|Bis)\\b/);
});
