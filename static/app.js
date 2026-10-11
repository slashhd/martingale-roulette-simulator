// Browser UI: the Rust API owns parameter validation and simulation results.
// Charts are optional so simulations and CSV history still work offline.
const chartsAvailable = typeof Chart !== 'undefined';
function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, char => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
  }[char]));
}
    let distributionChart = null;
    let roundsDistributionChart = null;
    let trajectoriesChart = null;
    let timelineChart = null;
    let trendsChart = null;

    let historyData = [];
    let currentHistoryFilter = 'all';
    let currentJobId = null;
    let jobPollInterval = null;

    // Form elements
    const simForm = document.getElementById('simForm');
    const selectedModeInput = document.getElementById('selectedMode');
    const runsInput = document.getElementById('runs');
    const runsDisplay = document.getElementById('runsDisplay');
    const startCapitalInput = document.getElementById('start_capital');
    const minBetInput = document.getElementById('min_bet');
    const maxBetInput = document.getElementById('max_bet');
    const unlimitedMaxBet = document.getElementById('unlimited_max_bet');
    const unlimitedNotice = document.getElementById('unlimitedNotice');
    const roundsPerRunInput = document.getElementById('rounds_per_run');
    const targetProfitInput = document.getElementById('target_profit');
    const rouletteTypeSelect = document.getElementById('roulette_type');

    // Dynamic mode containers
    const dynamicBoxIcon = document.getElementById('dynamicBoxIcon');
    const dynamicBoxTitle = document.getElementById('dynamicBoxTitle');
    const roundsFieldContainer = document.getElementById('roundsFieldContainer');
    const roundsFieldLabel = document.getElementById('roundsFieldLabel');
    const roundsFieldHint = document.getElementById('roundsFieldHint');
    const targetProfitContainer = document.getElementById('targetProfitContainer');
    const tillBustInfoNotice = document.getElementById('tillBustInfoNotice');

    const submitBtn = document.getElementById('submitBtn');
    const cancelJobBtn = document.getElementById('cancelJobBtn');
    const btnText = document.getElementById('btnText');
    const btnIcon = document.getElementById('btnIcon');
    const statusText = document.getElementById('statusText');
    const pingDot = document.getElementById('pingDot');
    const pingCore = document.getElementById('pingCore');

    const streamProgressContainer = document.getElementById('streamProgressContainer');
    const streamProgressBar = document.getElementById('streamProgressBar');
    const streamProgressRuns = document.getElementById('streamProgressRuns');
    const streamProgressSpeed = document.getElementById('streamProgressSpeed');
    const streamProgressEta = document.getElementById('streamProgressEta');

    // Mode Selector Logic
    document.querySelectorAll('.mode-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const mode = btn.dataset.mode;
        setSimulationMode(mode);
      });
    });

    function setSimulationMode(mode) {
      selectedModeInput.value = mode;

      document.querySelectorAll('.mode-btn').forEach(b => {
        const badge = b.querySelector('.mode-badge');
        if (b.dataset.mode === mode) {
          b.className = 'mode-btn p-3 rounded-xl border text-left transition flex flex-col justify-between bg-indigo-950/40 border-indigo-500/60 text-white shadow-sm';
          if (badge) badge.classList.remove('hidden');
        } else {
          b.className = 'mode-btn p-3 rounded-xl border text-left transition flex flex-col justify-between bg-slate-950 border-slate-800 text-slate-300 hover:border-slate-700';
          if (badge) badge.classList.add('hidden');
        }
      });

      // Adjust Dynamic Parameter Box
      if (mode === 'fixed_rounds') {
        dynamicBoxIcon.textContent = '⏱️';
        dynamicBoxTitle.textContent = 'Fixed spin settings';
        roundsFieldContainer.classList.remove('hidden');
        roundsFieldLabel.textContent = 'Spins per session';
        roundsFieldHint.textContent = 'e.g. 100';
        targetProfitContainer.classList.add('hidden');
        tillBustInfoNotice.classList.add('hidden');
      } else if (mode === 'target_profit') {
        dynamicBoxIcon.textContent = '🎯';
        dynamicBoxTitle.textContent = 'Profit target settings';
        roundsFieldContainer.classList.add('hidden');
        targetProfitContainer.classList.remove('hidden');
        tillBustInfoNotice.classList.add('hidden');
      } else if (mode === 'combined') {
        dynamicBoxIcon.textContent = '🔀';
        dynamicBoxTitle.textContent = 'Combined settings';
        roundsFieldContainer.classList.remove('hidden');
        roundsFieldLabel.textContent = 'Maximum spins per session';
        roundsFieldHint.textContent = 'e.g. 200';
        targetProfitContainer.classList.remove('hidden');
        tillBustInfoNotice.classList.add('hidden');
      } else if (mode === 'till_bust') {
        dynamicBoxIcon.textContent = '💀';
        dynamicBoxTitle.textContent = 'Until bust settings';
        roundsFieldContainer.classList.add('hidden');
        targetProfitContainer.classList.add('hidden');
        tillBustInfoNotice.classList.remove('hidden');
      }
    }

    function formatNumberLabel(num) {
      if (num >= 1000000000) {
        return (num / 1000000000).toLocaleString('en-GB', { maximumFractionDigits: 2 }) + ' billion';
      }
      if (num >= 1000000) {
        return (num / 1000000).toLocaleString('en-GB', { maximumFractionDigits: 2 }) + ' million';
      }
      return num.toLocaleString('en-GB');
    }

    function updateRunsDisplay() {
      const val = parseInt(runsInput.value, 10) || 0;
      runsDisplay.textContent = `${val.toLocaleString('en-GB')} (${formatNumberLabel(val)})`;
    }

    runsInput.addEventListener('input', updateRunsDisplay);

    document.querySelectorAll('.preset-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('.preset-btn').forEach(b => {
          b.classList.remove('bg-indigo-900/40', 'text-indigo-300', 'border-indigo-700/50');
          b.classList.add('bg-slate-800', 'text-slate-300', 'border-slate-700');
        });
        btn.classList.add('bg-indigo-900/40', 'text-indigo-300', 'border-indigo-700/50');
        btn.classList.remove('bg-slate-800', 'text-slate-300');
        runsInput.value = btn.dataset.runs;
        updateRunsDisplay();
      });
    });

    unlimitedMaxBet.addEventListener('change', () => {
      if (unlimitedMaxBet.checked) {
        maxBetInput.disabled = true;
        unlimitedNotice.classList.remove('hidden');
      } else {
        maxBetInput.disabled = false;
        unlimitedNotice.classList.add('hidden');
      }
    });

    // Chart Tabs
    const tabDistributionBtn = document.getElementById('tabDistributionBtn');
    const tabTimelineBtn = document.getElementById('tabTimelineBtn');
    const tabRoundsDistributionBtn = document.getElementById('tabRoundsDistributionBtn');
    const tabTrajectoriesBtn = document.getElementById('tabTrajectoriesBtn');
    const tabTrendsBtn = document.getElementById('tabTrendsBtn');

    const tabDistribution = document.getElementById('tabDistribution');
    const tabTimeline = document.getElementById('tabTimeline');
    const tabRoundsDistribution = document.getElementById('tabRoundsDistribution');
    const tabTrajectories = document.getElementById('tabTrajectories');
    const tabTrends = document.getElementById('tabTrends');

    function selectTab(btn, pane) {
      [tabDistributionBtn, tabTimelineBtn, tabRoundsDistributionBtn, tabTrajectoriesBtn, tabTrendsBtn].forEach(b => {
        b.className = 'chart-tab-btn px-3 py-1.5 rounded-lg text-slate-400 hover:text-slate-200 transition whitespace-nowrap';
      });
      [tabDistribution, tabTimeline, tabRoundsDistribution, tabTrajectories, tabTrends].forEach(p => p.classList.add('hidden'));

      btn.className = 'chart-tab-btn px-3 py-1.5 rounded-lg bg-indigo-600 text-white transition whitespace-nowrap';
      pane.classList.remove('hidden');
    }

    tabDistributionBtn.addEventListener('click', () => selectTab(tabDistributionBtn, tabDistribution));
    tabTimelineBtn.addEventListener('click', () => selectTab(tabTimelineBtn, tabTimeline));
    tabRoundsDistributionBtn.addEventListener('click', () => selectTab(tabRoundsDistributionBtn, tabRoundsDistribution));
    tabTrajectoriesBtn.addEventListener('click', () => selectTab(tabTrajectoriesBtn, tabTrajectories));
    tabTrendsBtn.addEventListener('click', () => {
      selectTab(tabTrendsBtn, tabTrends);
      renderTrendsChart();
    });

    function parseCleanFloat(val, fallback = 0) {
      if (typeof val === 'number') return val;
      if (!val) return fallback;
      const num = Number(String(val).trim());
      return Number.isFinite(num) ? num : fallback;
    }

    function parseCleanInt(val, fallback = 0) {
      if (val === '' || val === null || val === undefined) return fallback;
      const num = Number(val);
      return Number.isSafeInteger(num) ? num : fallback;
    }

    // Form Submit
    simForm.addEventListener('submit', async (e) => {
      e.preventDefault();

      const mode = selectedModeInput.value || 'fixed_rounds';
      const runs = parseCleanInt(runsInput.value, 0);
      const min_bet = parseCleanFloat(minBetInput.value, 0);
      const isUnlimited = unlimitedMaxBet.checked;
      const max_bet = isUnlimited ? 999999999.0 : parseCleanFloat(maxBetInput.value, 0);
      const start_capital = parseCleanFloat(startCapitalInput.value, 0);
      const rounds_per_run = (mode === 'target_profit' || mode === 'till_bust') ? 0 : parseCleanInt(roundsPerRunInput.value, 0);
      const target_profit_val = targetProfitInput.value ? parseCleanFloat(targetProfitInput.value, 0) : null;
      const roulette_type = rouletteTypeSelect.value;

      if (!Number.isSafeInteger(runs) || runs < 1 || runs > 1000000000) {
        alert("Runs must be an integer between 1 and 1,000,000,000.");
        return;
      }
      if (start_capital <= 0) {
        alert("Starting bankroll must be greater than €0.");
        return;
      }
      if (min_bet <= 0) {
        alert("Minimum bet must be greater than €0.");
        return;
      }
      if (!isUnlimited && max_bet < min_bet) {
        alert("Table limit must be at least the minimum bet.");
        return;
      }

      if ((mode === 'fixed_rounds' || mode === 'combined') && (rounds_per_run < 1 || rounds_per_run > 1000000)) {
        alert('Spins per session must be an integer between 1 and 1,000,000.');
        return;
      }
      if ((mode === 'target_profit' || mode === 'combined') && !(target_profit_val > 0)) {
        alert('Profit target must be greater than €0.');
        return;
      }
      setRunningState(true, runs);

      const payload = {
        runs,
        min_bet,
        max_bet,
        unlimited_max_bet: isUnlimited,
        start_capital,
        mode,
        rounds_per_run,
        target_profit: target_profit_val,
        roulette_type
      };

      try {
        const response = await fetch('/api/simulate', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload)
        });

        if (!response.ok) {
          const errText = await response.text();
          throw new Error(errText || 'Simulation error');
        }

        const data = await response.json();

        if (data.job_id) {
          currentJobId = data.job_id;
          cancelJobBtn.classList.remove('hidden');
          streamProgressContainer.classList.remove('hidden');
          startJobPolling(currentJobId);
        } else {
          updateUIWithResults(data);
          await loadHistory();
          setRunningState(false);
        }
      } catch (err) {
        alert("Simulation failed: " + err.message);
        setRunningState(false, undefined, 'Simulation failed: ' + err.message);
      }
    });

    cancelJobBtn.addEventListener('click', async () => {
      if (!currentJobId) return;
      try {
        await fetch(`/api/job/${currentJobId}/cancel`, { method: 'POST' });
        statusText.textContent = "Cancelling simulation...";
      } catch (err) {
        console.error(err);
      }
    });

    function startJobPolling(jobId) {
      if (jobPollInterval) clearInterval(jobPollInterval);

      jobPollInterval = setInterval(async () => {
        try {
          const res = await fetch(`/api/job/${jobId}`);
          if (!res.ok) {
            clearInterval(jobPollInterval);
            setRunningState(false, undefined, 'Could not retrieve job status.');
            return;
          }
          const progress = await res.json();

          const pct = progress.percentage.toFixed(1);
          streamProgressBar.style.width = `${pct}%`;
          streamProgressRuns.textContent = `${progress.completed_runs.toLocaleString('en-GB')} / ${progress.total_runs.toLocaleString('en-GB')} (${pct} %)`;

          const speedRuns = Number(progress.speed_runs_per_sec.toFixed(0)).toLocaleString('en-GB');
          streamProgressSpeed.textContent = `🚀 ${speedRuns} Runs/s`;

          const etaSec = (progress.estimated_remaining_ms / 1000).toFixed(1);
          streamProgressEta.textContent = `ETA: ${etaSec}s`;

          const elapsedSec = (progress.elapsed_ms / 1000).toFixed(1);
          document.getElementById('kpiDuration').textContent = `${elapsedSec}s`;

          if (progress.is_cancelled) {
            clearInterval(jobPollInterval);
            setRunningState(false, undefined, 'Simulation cancelled.');
          } else if (progress.is_finished && progress.error) {
            setRunningState(false, undefined, progress.error);
          } else if (progress.is_finished && progress.result) {
            clearInterval(jobPollInterval);
            updateUIWithResults(progress.result);
            await loadHistory();
            setRunningState(false);
          }
        } catch (err) {
          console.error("Polling error:", err);
        }
      }, 150);
    }

    function setRunningState(isRunning, runs, message = 'Simulation completed successfully') {
      if (isRunning) {
        submitBtn.disabled = true;
        submitBtn.classList.add('opacity-75', 'cursor-wait');
        btnText.textContent = `Computing ${formatNumberLabel(runs)} Runs...`;
        btnIcon.innerHTML = `<svg class="animate-spin h-4 w-4 text-white" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v8H4z"></path></svg>`;
        statusText.textContent = `Simulation running...`;
        pingCore.className = 'relative inline-flex rounded-full h-2.5 w-2.5 bg-amber-500';
        pingDot.className = 'animate-ping absolute inline-flex h-full w-full rounded-full bg-amber-400 opacity-75';
      } else {
        if (jobPollInterval) clearInterval(jobPollInterval);
        submitBtn.disabled = false;
        submitBtn.classList.remove('opacity-75', 'cursor-wait');
        btnText.textContent = "Start Monte Carlo simulation";
        btnIcon.textContent = "🚀";
        cancelJobBtn.classList.add('hidden');
        streamProgressContainer.classList.add('hidden');
        statusText.textContent = message;
        currentJobId = null;
        pingCore.className = 'relative inline-flex rounded-full h-2.5 w-2.5 bg-emerald-500';
        pingDot.className = 'animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75';
      }
    }

    // Present the stopping conditions most relevant to each mode.
    function updateUIWithResults(data) {
      const mode = data.mode || 'fixed_rounds';
      document.getElementById('chartsSubheader').textContent = chartsAvailable
        ? 'Statistics sample: ' + data.statistics_sample_size.toLocaleString('en-GB') +
          ' sessions; timeline sample: ' + data.timeline_sample_size.toLocaleString('en-GB') + ' sessions.'
        : 'Charts unavailable: connect to the internet and reload to load Chart.js. Simulation and CSV history remain available.';

      // Duration & Throughput
      const durMs = data.duration_ms;
      const durStr = durMs < 1000 ? `${durMs.toFixed(1)} ms` : `${(durMs / 1000).toFixed(2)} s`;
      document.getElementById('kpiDuration').textContent = durStr;
      const runsPerSec = (data.total_runs / (durMs / 1000)).toFixed(0);
      document.getElementById('kpiThroughput').textContent = `(${Number(runsPerSec).toLocaleString('en-GB')} Runs/s)`;

      // Secondary Bar: Rounds & Streaks
      document.getElementById('kpiAvgRounds').textContent = `${data.avg_rounds_played.toFixed(1)} / ${data.median_rounds_played.toFixed(0)}`;
      document.getElementById('kpiMaxLossStreak').textContent = `${data.max_consecutive_losses} sessions`;
      document.getElementById('kpiMaxWinStreak').textContent = `${data.max_consecutive_wins} sessions`;

      // Safety Cap Banner
      const safetyBanner = document.getElementById('safetyCapBanner');
      if (data.hit_safety_cap_runs > 0) {
        safetyBanner.classList.remove('hidden');
        document.getElementById('safetyCapBannerText').textContent =
          `${data.hit_safety_cap_runs.toLocaleString('en-GB')} of ${data.total_runs.toLocaleString('en-GB')} Runs (${data.hit_safety_cap_percentage.toFixed(2)} %) reached the safety cap! ${data.bankrupt_runs.toLocaleString('en-GB')} Runs (${data.bankrupt_percentage.toFixed(2)} %) went bust.`;
      } else {
        safetyBanner.classList.add('hidden');
      }

      // Adapt 4 Main KPI Cards by Mode
      if (mode === 'till_bust') {
        // Mode 4: Till Bust
        document.getElementById('card1Title').textContent = 'Session outcomes';
        const hasHitCap = data.hit_safety_cap_runs > 0;
        document.getElementById('card1Badge').textContent = hasHitCap ? '💀 Bust vs. 🛡️ Safety cap' : '100% Bust';
        document.getElementById('card1Badge').className = hasHitCap
          ? 'text-xs font-semibold px-2 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20'
          : 'text-xs font-semibold px-2 py-0.5 rounded bg-rose-500/10 text-rose-400 border border-rose-500/20';

        document.getElementById('card1Value').textContent = `${data.bankrupt_percentage.toFixed(2)} %`;
        document.getElementById('card1Value').className = 'text-3xl sm:text-4xl font-extrabold text-rose-400 font-mono-num';
        document.getElementById('card1SubValue').textContent = `Bust (${data.bankrupt_runs.toLocaleString('en-GB')} Runs)`;
        document.getElementById('card1ProgressBar').style.width = `${Math.min(100, data.bankrupt_percentage)}%`;
        document.getElementById('card1ProgressBar').className = 'bg-rose-500 h-full transition-all duration-500';

        document.getElementById('card1DetailLabel').textContent = hasHitCap ? 'Reached safety cap:' : '1 million Cap:';
        document.getElementById('card1DetailVal').textContent = hasHitCap
          ? `${data.hit_safety_cap_runs.toLocaleString('en-GB')} Runs (${data.hit_safety_cap_percentage.toFixed(2)} %)`
          : `0 (All sessions went bust)`;

        document.getElementById('card2Title').textContent = 'Survival time (spins)';
        document.getElementById('card2Badge').textContent = 'Spin range';
        document.getElementById('card2Value').textContent = `${data.avg_rounds_played.toFixed(1)}`;
        document.getElementById('card2DetailLabel').textContent = 'Shortest / longest:';
        document.getElementById('card2DetailVal').textContent = `${data.min_rounds_played} / ${data.max_rounds_played.toLocaleString('en-GB')} Spins`;
        document.getElementById('card2Subtext').textContent = hasHitCap
          ? `🛡️ ${data.hit_safety_cap_runs.toLocaleString('en-GB')} sessions (${data.hit_safety_cap_percentage.toFixed(2)}%) reached the safety cap!`
          : 'Average spins until bust across all sessions';

        document.getElementById('card3Title').textContent = 'Median survival time';
        document.getElementById('card3Badge').textContent = '50th percentile';
        document.getElementById('card3Value').textContent = `${data.median_rounds_played.toFixed(0)} spins`;
        document.getElementById('card3DetailLabel').textContent = '10th–90th percentile range:';
        document.getElementById('card3DetailVal').textContent = `${data.p10_rounds ? data.p10_rounds.toFixed(0) : '-'} to ${data.p90_rounds ? data.p90_rounds.toFixed(0) : '-'} spins`;
        document.getElementById('card3Subtext').textContent = 'Sampled median of spins played, including capped sessions';

        document.getElementById('card4Title').textContent = 'Peak bankroll';
        document.getElementById('card4Badge').textContent = 'Peak';
        document.getElementById('card4Value').textContent = `${data.max_capital_overall.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        document.getElementById('card4DetailLabel').textContent = 'Starting bankroll:';
        document.getElementById('card4DetailVal').textContent = `${data.params.start_capital.toFixed(0)} €`;
        const capNotice = data.params.unlimited_max_bet ? 'Peak bankroll includes capped sessions' : 'Highest bankroll at any point in a session';
        document.getElementById('card4Subtext').textContent = capNotice;

      } else if (mode === 'target_profit') {
        // Mode 2: Target Profit
        document.getElementById('card1Title').textContent = 'Target reached';
        document.getElementById('card1Badge').textContent = 'Target rate';
        document.getElementById('card1Badge').className = 'text-xs font-semibold px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20';
        document.getElementById('card1Value').textContent = `${data.target_reached_percentage.toFixed(2)} %`;
        document.getElementById('card1Value').className = 'text-3xl sm:text-4xl font-extrabold text-emerald-400 font-mono-num';
        document.getElementById('card1SubValue').textContent = `(${data.target_reached_runs.toLocaleString('en-GB')} Runs)`;
        document.getElementById('card1ProgressBar').style.width = `${data.target_reached_percentage}%`;
        document.getElementById('card1ProgressBar').className = 'bg-emerald-500 h-full transition-all duration-500';
        document.getElementById('card1DetailLabel').textContent = 'Went bust:';
        document.getElementById('card1DetailVal').textContent = `${data.bankrupt_runs.toLocaleString('en-GB')} (${data.bankrupt_percentage.toFixed(2)} %)`;

        document.getElementById('card2Title').textContent = 'Average final bankroll';
        document.getElementById('card2Badge').textContent = 'Mean';
        document.getElementById('card2Value').textContent = `${data.avg_result.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        const avgProfit = data.avg_profit;
        document.getElementById('card2DetailLabel').textContent = 'Net profit:';
        const pEl = document.getElementById('card2DetailVal');
        pEl.textContent = `${avgProfit >= 0 ? '+' : ''}${avgProfit.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        pEl.className = avgProfit >= 0 ? 'font-mono-num text-sm font-bold text-emerald-400' : 'font-mono-num text-sm font-bold text-rose-400';
        document.getElementById('card2Subtext').textContent = 'Average across all sessions, including losses and capped sessions';

        document.getElementById('card3Title').textContent = 'Spins until stopping';
        document.getElementById('card3Badge').textContent = 'Duration';
        document.getElementById('card3Value').textContent = `${data.avg_rounds_played.toFixed(1)}`;
        document.getElementById('card3DetailLabel').textContent = 'Median Spins:';
        document.getElementById('card3DetailVal').textContent = `${data.median_rounds_played.toFixed(0)} spins`;
        document.getElementById('card3Subtext').textContent = 'Average spins across all stopping conditions';

        document.getElementById('card4Title').textContent = 'Peak bankroll';
        document.getElementById('card4Badge').textContent = 'Peak All Runs';
        document.getElementById('card4Value').textContent = `${data.max_capital_overall.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        document.getElementById('card4DetailLabel').textContent = 'Lowest final bankroll:';
        document.getElementById('card4DetailVal').textContent = `${data.min_result.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        document.getElementById('card4Subtext').textContent = 'Highest bankroll across all sessions';

      } else if (mode === 'combined') {
        // Mode 3: Combined
        document.getElementById('card1Title').textContent = 'Target vs. spin limit vs. bust';
        document.getElementById('card1Badge').textContent = 'Outcomes';
        document.getElementById('card1Badge').className = 'text-xs font-semibold px-2 py-0.5 rounded bg-indigo-500/10 text-indigo-400 border border-indigo-500/20';
        document.getElementById('card1Value').textContent = `${data.target_reached_percentage.toFixed(1)} %`;
        document.getElementById('card1Value').className = 'text-3xl sm:text-4xl font-extrabold text-emerald-400 font-mono-num';
        document.getElementById('card1SubValue').textContent = `reached the target`;
        document.getElementById('card1ProgressBar').style.width = `${data.target_reached_percentage}%`;
        document.getElementById('card1ProgressBar').className = 'bg-emerald-500 h-full transition-all duration-500';
        document.getElementById('card1DetailLabel').textContent = `Spin limit: ${data.round_limit_percentage.toFixed(1)}% | Bust:`;
        document.getElementById('card1DetailVal').textContent = `${data.bankrupt_percentage.toFixed(1)}%`;

        document.getElementById('card2Title').textContent = 'Average final bankroll';
        document.getElementById('card2Badge').textContent = 'Mean';
        document.getElementById('card2Value').textContent = `${data.avg_result.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        const avgProfit = data.avg_profit;
        document.getElementById('card2DetailLabel').textContent = 'Net profit:';
        const pEl = document.getElementById('card2DetailVal');
        pEl.textContent = `${avgProfit >= 0 ? '+' : ''}${avgProfit.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        pEl.className = avgProfit >= 0 ? 'font-mono-num text-sm font-bold text-emerald-400' : 'font-mono-num text-sm font-bold text-rose-400';
        document.getElementById('card2Subtext').textContent = 'Average across all simulated sessions';

        document.getElementById('card3Title').textContent = 'Median final bankroll';
        document.getElementById('card3Badge').textContent = '50th percentile';
        document.getElementById('card3Value').textContent = `${data.median_result.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        const medProfit = data.median_profit;
        document.getElementById('card3DetailLabel').textContent = 'Median profit:';
        const mEl = document.getElementById('card3DetailVal');
        mEl.textContent = `${medProfit >= 0 ? '+' : ''}${medProfit.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        mEl.className = medProfit >= 0 ? 'font-mono-num text-sm font-bold text-emerald-400' : 'font-mono-num text-sm font-bold text-rose-400';
        document.getElementById('card3Subtext').textContent = 'Sampled median across all sessions';

        document.getElementById('card4Title').textContent = 'Spins played';
        document.getElementById('card4Badge').textContent = 'spins';
        document.getElementById('card4Value').textContent = `${data.avg_rounds_played.toFixed(1)}`;
        document.getElementById('card4DetailLabel').textContent = 'Spin limit:';
        document.getElementById('card4DetailVal').textContent = `${data.params.rounds_per_run} Spins`;
        document.getElementById('card4Subtext').textContent = 'Sessions may stop early after reaching the target or going bust';

      } else {
        // Mode 1: Fixed Rounds (Default)
        document.getElementById('card1Title').textContent = 'Sessions ending at a loss';
        document.getElementById('card1Badge').textContent = 'Loss / bust';
        document.getElementById('card1Badge').className = 'text-xs font-semibold px-2 py-0.5 rounded bg-rose-500/10 text-rose-400 border border-rose-500/20';
        document.getElementById('card1Value').textContent = `${data.failed_percentage.toFixed(2)} %`;
        document.getElementById('card1Value').className = 'text-3xl sm:text-4xl font-extrabold text-rose-400 font-mono-num';
        document.getElementById('card1SubValue').textContent = `(${data.failed_runs.toLocaleString('en-GB')} of ${data.total_runs.toLocaleString('en-GB')})`;
        document.getElementById('card1ProgressBar').style.width = `${Math.min(100, data.failed_percentage)}%`;
        document.getElementById('card1ProgressBar').className = 'bg-rose-500 h-full transition-all duration-500';
        document.getElementById('card1DetailLabel').textContent = 'Unable to place minimum bet:';
        document.getElementById('card1DetailVal').textContent = `${data.bankrupt_runs.toLocaleString('en-GB')} (${data.bankrupt_percentage.toFixed(2)} %)`;

        document.getElementById('card2Title').textContent = 'Average final bankroll';
        document.getElementById('card2Badge').textContent = 'Final bankroll';
        document.getElementById('card2Value').textContent = `${data.avg_result.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        const avgProfit = data.avg_profit;
        document.getElementById('card2DetailLabel').textContent = 'Net profit / loss:';
        const pEl = document.getElementById('card2DetailVal');
        pEl.textContent = `${avgProfit >= 0 ? '+' : ''}${avgProfit.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        pEl.className = avgProfit >= 0 ? 'font-mono-num text-sm font-bold text-emerald-400' : 'font-mono-num text-sm font-bold text-rose-400';
        document.getElementById('card2Subtext').textContent = 'Average across all simulated sessions';

        document.getElementById('card3Title').textContent = 'Median final bankroll';
        document.getElementById('card3Badge').textContent = '50th percentile';
        document.getElementById('card3Value').textContent = `${data.median_result.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        const medProfit = data.median_profit;
        document.getElementById('card3DetailLabel').textContent = 'Net profit / loss:';
        const mEl = document.getElementById('card3DetailVal');
        mEl.textContent = `${medProfit >= 0 ? '+' : ''}${medProfit.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        mEl.className = medProfit >= 0 ? 'font-mono-num text-sm font-bold text-emerald-400' : 'font-mono-num text-sm font-bold text-rose-400';
        document.getElementById('card3Subtext').textContent = 'Sampled median includes sessions that went bust';

        document.getElementById('card4Title').textContent = 'Peak bankroll';
        document.getElementById('card4Badge').textContent = 'Peak All Runs';
        document.getElementById('card4Value').textContent = `${data.max_capital_overall.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        document.getElementById('card4DetailLabel').textContent = 'Lowest final bankroll:';
        document.getElementById('card4DetailVal').textContent = `${data.min_result.toLocaleString('en-GB', { minimumFractionDigits: 2 })} €`;
        document.getElementById('card4Subtext').textContent = 'Highest bankroll at any point';
      }

      // Render Charts
      renderDistributionChart(data.distribution);
      renderTimelineChart(data.timeline_percentiles, data.params.start_capital);
      renderRoundsDistributionChart(data.rounds_distribution);
      renderTrajectoriesChart(data.sample_trajectories, data.params.start_capital, data.params.target_profit, mode);

      // Auto switch chart tab to Timeline
      selectTab(tabTimelineBtn, tabTimeline);
    }

    if (chartsAvailable) {
      Chart.defaults.color = '#94a3b8';
      Chart.defaults.borderColor = '#334155';
      Chart.defaults.font.family = "'Plus Jakarta Sans', sans-serif";

    }

    function renderDistributionChart(distribution) {
      if (!chartsAvailable) return;
      const ctx = document.getElementById('distributionChart').getContext('2d');
      if (distributionChart) distributionChart.destroy();
      if (!distribution || distribution.length === 0) return;

      const labels = distribution.map(d => d.label);
      const counts = distribution.map(d => d.count);
      const percentages = distribution.map(d => d.percentage);

      const backgroundColors = distribution.map(d => {
        if (d.min_val <= 0.01) {
          return 'rgba(244, 63, 94, 0.85)';
        }
        if (d.label.startsWith('>')) {
          return 'rgba(16, 185, 129, 0.85)';
        }
        return 'rgba(99, 102, 241, 0.8)';
      });

      distributionChart = new Chart(ctx, {
        type: 'bar',
        data: {
          labels,
          datasets: [{
            label: 'Estimated runs',
            data: counts,
            backgroundColor: backgroundColors,
            borderRadius: 6,
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            legend: { display: false },
            tooltip: {
              callbacks: {
                label: (ctx) => ` ${counts[ctx.dataIndex].toLocaleString('en-GB')} Runs (${percentages[ctx.dataIndex].toFixed(2)} %)`
              }
            }
          },
          scales: {
            x: { grid: { display: false }, ticks: { maxRotation: 45, font: { size: 10 } } },
            y: { grid: { color: 'rgba(51, 65, 85, 0.5)' }, ticks: { callback: v => Number(v).toLocaleString('en-GB') } }
          }
        }
      });
    }

    // Timeline Fan Chart (Mean, Median, p10, p25, p75, p90)
    function renderTimelineChart(percentiles, startCapital) {
      if (!chartsAvailable) return;
      const ctx = document.getElementById('timelineChart').getContext('2d');
      if (timelineChart) timelineChart.destroy();
      if (!percentiles || percentiles.length === 0) return;

      const rounds = percentiles.map(p => p.round);
      const means = percentiles.map(p => p.mean);
      const medians = percentiles.map(p => p.median);
      const p10s = percentiles.map(p => p.p10);
      const p25s = percentiles.map(p => p.p25);
      const p75s = percentiles.map(p => p.p75);
      const p90s = percentiles.map(p => p.p90);

      timelineChart = new Chart(ctx, {
        type: 'line',
        data: {
          labels: rounds,
          datasets: [
            // Outer Band: 90th percentile top edge
            {
              label: '90th percentile',
              data: p90s,
              borderColor: 'transparent',
              pointRadius: 0,
              fill: false,
            },
            // Outer Band: 10th percentile bottom edge, fills to p90
            {
              label: 'Middle 80% of outcomes (10th–90th)',
              data: p10s,
              borderColor: 'transparent',
              backgroundColor: 'rgba(99, 102, 241, 0.12)',
              pointRadius: 0,
              fill: '-1',
            },
            // Core Band: 75th percentile top edge
            {
              label: '75th percentile',
              data: p75s,
              borderColor: 'transparent',
              pointRadius: 0,
              fill: false,
            },
            // Core Band: 25th percentile bottom edge, fills to p75
            {
              label: 'Middle 50% of outcomes (25th–75th)',
              data: p25s,
              borderColor: 'transparent',
              backgroundColor: 'rgba(99, 102, 241, 0.28)',
              pointRadius: 0,
              fill: '-1',
            },
            // Median Line (p50)
            {
              label: 'Median (50%)',
              data: medians,
              borderColor: '#818cf8',
              borderWidth: 2.5,
              pointRadius: 0,
              fill: false,
              tension: 0.25,
            },
            // Mean Line
            {
              label: 'Mean',
              data: means,
              borderColor: '#34d399',
              borderDash: [5, 5],
              borderWidth: 2,
              pointRadius: 0,
              fill: false,
              tension: 0.25,
            },
            // Start Capital Reference Line
            {
              label: `Starting bankroll (${startCapital} €)`,
              data: rounds.map(() => startCapital),
              borderColor: '#64748b',
              borderDash: [2, 3],
              borderWidth: 1.5,
              pointRadius: 0,
              fill: false,
            }
          ]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          interaction: { mode: 'index', intersect: false },
          plugins: {
            legend: {
              position: 'top',
              labels: {
                boxWidth: 12,
                font: { size: 10 },
                filter: (item) => !['90th percentile', '75th percentile'].includes(item.text)
              }
            },
            tooltip: {
              callbacks: {
                title: (items) => `Spin ${items[0].label}`,
                label: (ctx) => {
                  if (['90th percentile', '75th percentile'].includes(ctx.dataset.label)) return null;
                  return `${ctx.dataset.label}: ${ctx.parsed.y.toLocaleString('en-GB', { minimumFractionDigits: 1, maximumFractionDigits: 1 })} €`;
                }
              }
            }
          },
          scales: {
            x: {
              title: { display: true, text: 'Spin (sampled checkpoints)', color: '#94a3b8' },
              grid: { color: 'rgba(51, 65, 85, 0.3)' },
              ticks: { maxTicksLimit: 16, maxRotation: 0 }
            },
            y: {
              title: { display: true, text: 'Bankroll (€)', color: '#94a3b8' },
              grid: { color: 'rgba(51, 65, 85, 0.5)' },
              ticks: { callback: v => `${v.toLocaleString('en-GB')} €` }
            }
          }
        }
      });
    }

    function renderRoundsDistributionChart(roundsDist) {
      if (!chartsAvailable) return;
      const ctx = document.getElementById('roundsDistributionChart').getContext('2d');
      if (roundsDistributionChart) roundsDistributionChart.destroy();
      if (!roundsDist || roundsDist.length === 0) return;

      const labels = roundsDist.map(d => d.label);
      const counts = roundsDist.map(d => d.count);
      const percentages = roundsDist.map(d => d.percentage);

      const backgroundColors = roundsDist.map(d => {
        if (d.label.includes('Long sessions') || d.label.includes('1 million')) {
          return 'rgba(16, 185, 129, 0.85)';
        }
        return 'rgba(245, 158, 11, 0.8)';
      });

      roundsDistributionChart = new Chart(ctx, {
        type: 'bar',
        data: {
          labels,
          datasets: [{
            label: 'Estimated sessions in this spin range',
            data: counts,
            backgroundColor: backgroundColors,
            borderRadius: 6,
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            legend: { display: false },
            tooltip: {
              callbacks: {
                label: (ctx) => ` ${counts[ctx.dataIndex].toLocaleString('en-GB')} Runs (${percentages[ctx.dataIndex].toFixed(2)} %)`
              }
            }
          },
          scales: {
            x: { grid: { display: false }, ticks: { maxRotation: 45, font: { size: 10 } } },
            y: { grid: { color: 'rgba(51, 65, 85, 0.5)' }, ticks: { callback: v => Number(v).toLocaleString('en-GB') } }
          }
        }
      });
    }

    function renderTrajectoriesChart(trajectories, startCapital, targetProfit, mode) {
      if (!chartsAvailable) return;
      const ctx = document.getElementById('trajectoriesChart').getContext('2d');
      if (trajectoriesChart) trajectoriesChart.destroy();
      if (!trajectories || trajectories.length === 0) return;

      const palette = [
        '#6366f1', '#10b981', '#f59e0b', '#ec4899', '#3b82f6',
        '#8b5cf6', '#14b8a6', '#f97316', '#06b6d4', '#eab308'
      ];

      const datasets = trajectories.map((traj, idx) => {
        const isBust = traj.is_bankrupt;
        const reachedTarget = traj.reached_target;
        let color = palette[idx % palette.length];
        if (isBust) color = 'rgba(244, 63, 94, 0.85)';
        if (reachedTarget) color = 'rgba(16, 185, 129, 0.85)';

        return {
          label: `Session #${traj.id} (${traj.final_capital.toFixed(0)} €, ${traj.rounds_played} spins)`,
          data: traj.points.map(p => ({ x: p.round, y: p.capital })),
          borderColor: color,
          backgroundColor: 'transparent',
          borderWidth: isBust ? 2.5 : 1.8,
          pointRadius: 0,
          tension: 0.1,
        };
      });

      trajectoriesChart = new Chart(ctx, {
        type: 'line',
        data: { datasets },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          interaction: { mode: 'nearest', intersect: false },
          plugins: {
            legend: { position: 'top', labels: { boxWidth: 10, font: { size: 10 } } },
            tooltip: {
              callbacks: {
                label: (ctx) => `${ctx.dataset.label}: ${ctx.parsed.y.toFixed(2)} € (Spin ${ctx.parsed.x})`
              }
            }
          },
          scales: {
            x: {
              type: 'linear',
              title: { display: true, text: 'Spin', color: '#94a3b8' },
              grid: { color: 'rgba(51, 65, 85, 0.3)' }
            },
            y: {
              title: { display: true, text: 'Bankroll (€)', color: '#94a3b8' },
              grid: { color: 'rgba(51, 65, 85, 0.5)' },
              ticks: { callback: v => `${v.toLocaleString('en-GB')} €` }
            }
          }
        }
      });
    }

    function renderTrendsChart() {
      if (!chartsAvailable) return;
      const ctx = document.getElementById('trendsChart').getContext('2d');
      if (trendsChart) trendsChart.destroy();
      if (!historyData || historyData.length === 0) return;

      const chronData = [...historyData].reverse();
      const labels = chronData.map((d, i) => `#${i + 1} (${d.mode || 'fixed'})`);
      const failedPcts = chronData.map(d => d.failed_percentage);
      const avgProfits = chronData.map(d => d.avg_profit);

      trendsChart = new Chart(ctx, {
        type: 'line',
        data: {
          labels,
          datasets: [
            {
              label: 'Sessions ending at a loss (%)',
              data: failedPcts,
              borderColor: '#f43f5e',
              backgroundColor: 'rgba(244, 63, 94, 0.1)',
              yAxisID: 'y',
              tension: 0.2,
              fill: true,
            },
            {
              label: 'Average net profit (€)',
              data: avgProfits,
              borderColor: '#10b981',
              backgroundColor: 'transparent',
              yAxisID: 'y1',
              borderDash: [5, 5],
              tension: 0.2,
            }
          ]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: { legend: { position: 'top', labels: { boxWidth: 12 } } },
          scales: {
            y: {
              type: 'linear',
              position: 'left',
              title: { display: true, text: 'Sessions ending at a loss (%)', color: '#f43f5e' },
              ticks: { callback: v => `${v.toFixed(1)} %` }
            },
            y1: {
              type: 'linear',
              position: 'right',
              title: { display: true, text: 'Net profit (€)', color: '#10b981' },
              grid: { drawOnChartArea: false },
              ticks: { callback: v => `${v.toFixed(0)} €` }
            }
          }
        }
      });
    }

    async function loadHistory() {
      try {
        const res = await fetch('/api/history');
        if (!res.ok) throw new Error(await res.text());
        historyData = await res.json();
        renderHistoryTable();
        if (trendsChart) renderTrendsChart();
      } catch (err) {
        console.error("Failed to load history:", err);
      }
    }

    // History Table Mode Filtering and Dynamic Column Rendering
    document.querySelectorAll('.history-filter-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        currentHistoryFilter = btn.dataset.modeFilter;
        document.querySelectorAll('.history-filter-btn').forEach(b => {
          b.className = 'history-filter-btn px-2.5 py-1 rounded-lg text-xs font-semibold bg-slate-800 text-slate-300 hover:bg-slate-700 transition';
        });
        btn.className = 'history-filter-btn px-2.5 py-1 rounded-lg text-xs font-semibold bg-indigo-600 text-white transition';
        renderHistoryTable();
      });
    });

    function renderHistoryTable() {
      const thead = document.getElementById('historyTableHead');
      const tbody = document.getElementById('historyTableBody');
      tbody.innerHTML = '';

      const filtered = historyData.filter(r => {
        if (currentHistoryFilter === 'all') return true;
        return (r.mode || 'fixed_rounds') === currentHistoryFilter;
      });

      if (!filtered || filtered.length === 0) {
        thead.innerHTML = `<tr><th class="py-3 px-3">History</th></tr>`;
        tbody.innerHTML = `<tr><td class="py-6 text-center text-slate-500 font-sans">No entries for this mode filter.</td></tr>`;
        return;
      }

      // Render Dynamic Header based on Filter
      if (currentHistoryFilter === 'till_bust') {
        thead.innerHTML = `
          <tr>
            <th class="py-3 px-3">Timestamp</th>
            <th class="py-3 px-2.5">Runs</th>
            <th class="py-3 px-2.5">Start</th>
            <th class="py-3 px-2.5">Bet min / table limit</th>
            <th class="py-3 px-2.5">Bust (%)</th>
            <th class="py-3 px-2.5">Safety cap (%)</th>
            <th class="py-3 px-2.5">Average spins survived</th>
            <th class="py-3 px-2.5">Median spins</th>
            <th class="py-3 px-2.5">Range (min / max)</th>
            <th class="py-3 px-2.5">Peak bankroll</th>
            <th class="py-3 px-2.5 text-right">Compute time</th>
          </tr>
        `;
      } else if (currentHistoryFilter === 'target_profit') {
        thead.innerHTML = `
          <tr>
            <th class="py-3 px-3">Timestamp</th>
            <th class="py-3 px-2.5">Runs</th>
            <th class="py-3 px-2.5">Start</th>
            <th class="py-3 px-2.5">Profit target</th>
            <th class="py-3 px-2.5">Target reached (%)</th>
            <th class="py-3 px-2.5">Bust (%)</th>
            <th class="py-3 px-2.5">Ø spins to Target</th>
            <th class="py-3 px-2.5">Median spins</th>
            <th class="py-3 px-2.5">Ø Net profit</th>
            <th class="py-3 px-2.5">Peak Bankroll</th>
            <th class="py-3 px-2.5 text-right">Compute time</th>
          </tr>
        `;
      } else if (currentHistoryFilter === 'combined') {
        thead.innerHTML = `
          <tr>
            <th class="py-3 px-3">Timestamp</th>
            <th class="py-3 px-2.5">Runs</th>
            <th class="py-3 px-2.5">Start</th>
            <th class="py-3 px-2.5">Limit / Target</th>
            <th class="py-3 px-2.5">Target reached (%)</th>
            <th class="py-3 px-2.5">Spin limit reached (%)</th>
            <th class="py-3 px-2.5">Bust (%)</th>
            <th class="py-3 px-2.5">Ø spins</th>
            <th class="py-3 px-2.5">Ø Final bankroll</th>
            <th class="py-3 px-2.5">Median</th>
            <th class="py-3 px-2.5 text-right">Compute time</th>
          </tr>
        `;
      } else {
        // 'all' or 'fixed_rounds'
        thead.innerHTML = `
          <tr>
            <th class="py-3 px-3">Timestamp</th>
            <th class="py-3 px-2.5">Mode</th>
            <th class="py-3 px-2.5">Runs</th>
            <th class="py-3 px-2.5">Start</th>
            <th class="py-3 px-2.5">Stop condition</th>
            <th class="py-3 px-2.5">Outcomes</th>
            <th class="py-3 px-2.5">Ø spins</th>
            <th class="py-3 px-2.5">Mean</th>
            <th class="py-3 px-2.5">Median</th>
            <th class="py-3 px-2.5">Max Bankroll</th>
            <th class="py-3 px-2.5 text-right">Compute time</th>
          </tr>
        `;
      }

      // Render Rows
      filtered.forEach(r => {
        const tr = document.createElement('tr');
        tr.className = 'hover:bg-slate-800/50 transition';

        const durStr = r.duration_ms < 1000 ? `${r.duration_ms.toFixed(1)} ms` : `${(r.duration_ms / 1000).toFixed(2)} s`;
        const maxBetDisplay = r.unlimited_max_bet ? '∞' : `${r.max_bet.toFixed(0)} €`;
        const avgProfitSign = r.avg_profit >= 0 ? '+' : '';
        const avgColor = r.avg_profit >= 0 ? 'text-emerald-400' : 'text-rose-400';

        if (currentHistoryFilter === 'till_bust') {
          const capText = r.hit_safety_cap_runs > 0 ? `${r.max_rounds_played.toLocaleString('en-GB')} (Cap)` : `${r.max_rounds_played.toLocaleString('en-GB')}`;
          const bPct = r.bankrupt_percentage !== null && r.bankrupt_percentage !== undefined ? r.bankrupt_percentage : (100 - (r.hit_safety_cap_percentage || 0));
          const capPct = r.hit_safety_cap_percentage || 0;
          tr.innerHTML = `
            <td class="py-3 px-3 text-slate-400 text-[11px]">${escapeHtml(r.timestamp)}</td>
            <td class="py-3 px-2.5 font-semibold text-white">${r.runs.toLocaleString('en-GB')}</td>
            <td class="py-3 px-2.5 text-slate-300">${r.start_capital.toFixed(0)} €</td>
            <td class="py-3 px-2.5 text-slate-300 text-[11px]">${r.min_bet.toFixed(0)} / <span class="${r.unlimited_max_bet ? 'text-amber-400 font-bold' : ''}">${maxBetDisplay}</span></td>
            <td class="py-3 px-2.5 font-bold text-rose-400 text-sm">${bPct.toFixed(1)} %</td>
            <td class="py-3 px-2.5 font-bold ${capPct > 0 ? 'text-amber-400' : 'text-slate-500'} text-xs">${capPct.toFixed(2)} %</td>
            <td class="py-3 px-2.5 font-bold text-amber-300 text-sm">${r.avg_rounds_played.toFixed(1)}</td>
            <td class="py-3 px-2.5 font-semibold text-indigo-300">${r.median_rounds_played.toFixed(0)} spins</td>
            <td class="py-3 px-2.5 text-slate-300 text-[11px]">${r.min_rounds_played} / ${capText}</td>
            <td class="py-3 px-2.5 text-amber-300 font-semibold">${r.max_capital.toLocaleString('en-GB', { minimumFractionDigits: 0, maximumFractionDigits: 0 })} €</td>
            <td class="py-3 px-2.5 text-right text-slate-400 text-[11px]">${durStr}</td>
          `;
        } else if (currentHistoryFilter === 'target_profit') {
          tr.innerHTML = `
            <td class="py-3 px-3 text-slate-400 text-[11px]">${escapeHtml(r.timestamp)}</td>
            <td class="py-3 px-2.5 font-semibold text-white">${r.runs.toLocaleString('en-GB')}</td>
            <td class="py-3 px-2.5 text-slate-300">${r.start_capital.toFixed(0)} €</td>
            <td class="py-3 px-2.5 font-bold text-emerald-400">+${r.target_profit ? r.target_profit.toFixed(0) : '?'} €</td>
            <td class="py-3 px-2.5 font-bold text-emerald-400 text-sm">${r.target_reached_percentage != null ? r.target_reached_percentage.toFixed(1) : '-'} %</td>
            <td class="py-3 px-2.5 text-rose-400 font-semibold">${r.bankrupt_percentage.toFixed(1)} %</td>
            <td class="py-3 px-2.5 text-indigo-300">${r.avg_rounds_played.toFixed(1)}</td>
            <td class="py-3 px-2.5 text-slate-300">${r.median_rounds_played.toFixed(0)} spins</td>
            <td class="py-3 px-2.5 font-semibold ${avgColor}">${avgProfitSign}${r.avg_profit.toFixed(2)} €</td>
            <td class="py-3 px-2.5 text-amber-300 font-semibold">${r.max_capital.toFixed(0)} €</td>
            <td class="py-3 px-2.5 text-right text-slate-400 text-[11px]">${durStr}</td>
          `;
        } else if (currentHistoryFilter === 'combined') {
          tr.innerHTML = `
            <td class="py-3 px-3 text-slate-400 text-[11px]">${escapeHtml(r.timestamp)}</td>
            <td class="py-3 px-2.5 font-semibold text-white">${r.runs.toLocaleString('en-GB')}</td>
            <td class="py-3 px-2.5 text-slate-300">${r.start_capital.toFixed(0)} €</td>
            <td class="py-3 px-2.5 text-indigo-300 text-[11px]">${r.rounds_per_run} spins / +${r.target_profit ? r.target_profit.toFixed(0) : '?'} €</td>
            <td class="py-3 px-2.5 font-bold text-emerald-400">${r.target_reached_percentage != null ? r.target_reached_percentage.toFixed(1) : '-'} %</td>
            <td class="py-3 px-2.5 text-slate-300">${r.round_limit_percentage ? r.round_limit_percentage.toFixed(1) : '-'} %</td>
            <td class="py-3 px-2.5 text-rose-400 font-semibold">${r.bankrupt_percentage.toFixed(1)} %</td>
            <td class="py-3 px-2.5 text-slate-300">${r.avg_rounds_played.toFixed(1)}</td>
            <td class="py-3 px-2.5 font-semibold ${avgColor}">${r.avg_result.toFixed(2)} €</td>
            <td class="py-3 px-2.5 text-indigo-300">${r.median_result.toFixed(2)} €</td>
            <td class="py-3 px-2.5 text-right text-slate-400 text-[11px]">${durStr}</td>
          `;
        } else {
          // 'all' or 'fixed_rounds'
          let modeBadge = '⏱️ Fixed spins';
          let conditionText = `${r.rounds_per_run} spins`;
          if (r.mode === 'target_profit') {
            modeBadge = '🎯 Target';
            conditionText = `Target: +${r.target_profit ? r.target_profit.toFixed(0) : '?'} €`;
          } else if (r.mode === 'combined') {
            modeBadge = '🔀 Combined';
            conditionText = `${r.rounds_per_run} spins / +${r.target_profit ? r.target_profit.toFixed(0) : '?'} €`;
          } else if (r.mode === 'till_bust') {
            modeBadge = '💀 Bust';
            conditionText = 'Until bust';
          }

          let outcomeCol = `${r.failed_percentage.toFixed(1)} % loss`;
          if (r.mode === 'target_profit' || r.mode === 'combined') {
            outcomeCol = `<span class="text-emerald-400">${r.target_reached_percentage != null ? r.target_reached_percentage.toFixed(1) : '-'}% Target</span> / <span class="text-rose-400">${r.bankrupt_percentage.toFixed(1)}% Bust</span>`;
          } else if (r.mode === 'till_bust') {
            outcomeCol = `<span class="text-amber-400">${r.bankrupt_percentage.toFixed(1)}% bust / ${(r.hit_safety_cap_percentage || 0).toFixed(1)}% capped</span>`;
          }

          tr.innerHTML = `
            <td class="py-3 px-3 text-slate-400 text-[11px]">${escapeHtml(r.timestamp)}</td>
            <td class="py-3 px-2.5 font-semibold text-indigo-300 text-[11px]">${modeBadge}</td>
            <td class="py-3 px-2.5 font-semibold text-white">${r.runs.toLocaleString('en-GB')}</td>
            <td class="py-3 px-2.5 text-slate-300">${r.start_capital.toFixed(0)} €</td>
            <td class="py-3 px-2.5 text-slate-300 text-[11px]">${conditionText}</td>
            <td class="py-3 px-2.5 font-bold">${outcomeCol}</td>
            <td class="py-3 px-2.5 text-amber-300">${r.avg_rounds_played ? r.avg_rounds_played.toFixed(1) : '-'}</td>
            <td class="py-3 px-2.5 font-semibold ${avgColor}">${r.avg_result.toFixed(2)} € <span class="text-[10px] block font-normal">(${avgProfitSign}${r.avg_profit.toFixed(2)} €)</span></td>
            <td class="py-3 px-2.5 text-indigo-300">${r.median_result.toFixed(2)} €</td>
            <td class="py-3 px-2.5 text-amber-300 font-semibold">${r.max_capital.toFixed(0)} €</td>
            <td class="py-3 px-2.5 text-right text-slate-400 text-[11px]">${durStr}</td>
          `;
        }

        tbody.appendChild(tr);
      });
    }

    document.getElementById('exportCsvBtn').addEventListener('click', () => {
      window.location.href = '/api/export_csv';
    });

    document.getElementById('refreshHistoryBtn').addEventListener('click', () => {
      loadHistory();
    });

    document.getElementById('clearHistoryBtn').addEventListener('click', async () => {
      if (!confirm("Delete all saved simulation history from the CSV file?")) {
        return;
      }
      try {
        const res = await fetch('/api/history', { method: 'DELETE' });
        if (!res.ok) throw new Error(await res.text());
        if (res.ok) {
          await loadHistory();
          if (trendsChart) {
            trendsChart.destroy();
            trendsChart = null;
          }
        }
      } catch (err) {
        alert("Could not clear history: " + err.message);
      }
    });

    window.addEventListener('DOMContentLoaded', async () => {
      updateRunsDisplay();
      setSimulationMode('fixed_rounds');
      if (!chartsAvailable) {
        document.getElementById('chartsSubheader').textContent =
          'Charts unavailable: connect to the internet and reload to load Chart.js.';
      }
      await loadHistory();
    });
