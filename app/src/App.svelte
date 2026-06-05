<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import PEKGauge from './lib/PEKGauge.svelte';
  import ProviderSwitcher from './lib/ProviderSwitcher.svelte';

  let admitted = $state(0);
  let rejected = $state(0);
  let warnings = $state(0);
  let config = $state({
    default_policy: 'Enforced',
    local_proxy_port: 8080,
    upstream_provider: 'Ollama',
  });

  async function refreshStats() {
    try {
      const result = await invoke('get_pek_stats');
      admitted = result.admitted;
      rejected = result.rejected;
      warnings = result.advisory_warnings;
      if (result.config) config = result.config;
    } catch (e) {
      console.error('IPC get_pek_stats error:', e);
    }
  }

  async function handleConfigChange(key: string, value: string | number) {
    const updated = { ...config, [key]: value };
    config = updated;
    try {
      await invoke('update_config', { newConfig: updated });
    } catch (e) {
      console.error('IPC update_config error:', e);
    }
  }

  onMount(() => {
    refreshStats();
    const interval = setInterval(refreshStats, 2000);
    return () => clearInterval(interval);
  });
</script>

<header>
  <div style="display:flex;align-items:center;gap:8px">
    <span class="status-dot online"></span>
    <h1>Vantage</h1>
    <span class="subtitle">Knowledge Runtime</span>
  </div>
  <div style="display:flex;align-items:center;gap:16px">
    <span class="time">localhost:{config.local_proxy_port}</span>
    <span class="time">Policy: {config.default_policy}</span>
  </div>
</header>

<main>
  <div class="card" style="grid-column:1/-1">
    <h2>PEK-1 Gate Statistics</h2>
    <PEKGauge {admitted} {rejected} {warnings} policy={config.default_policy} />
  </div>

  <div class="card">
    <h2>Upstream Provider</h2>
    <ProviderSwitcher
      provider={config.upstream_provider}
      onswitch={(p) => handleConfigChange('upstream_provider', p)}
    />
  </div>

  <div class="card">
    <h2>Configuration</h2>
    <div class="provider-controls">
      <div class="config-row">
        <label>Default Policy</label>
        <select
          value={config.default_policy}
          onchange={(e) => handleConfigChange('default_policy', e.currentTarget.value)}
        >
          <option value="Disabled">Disabled</option>
          <option value="Advisory">Advisory</option>
          <option value="Enforced">Enforced</option>
          <option value="StrictCanonical">StrictCanonical</option>
        </select>
      </div>
      <div class="config-row">
        <label>Proxy Port</label>
        <input
          type="number"
          value={config.local_proxy_port}
          onchange={(e) => handleConfigChange('local_proxy_port', parseInt(e.currentTarget.value))}
        />
      </div>
    </div>
  </div>

  <div class="card" style="grid-column:1/-1">
    <h2>Recent Transactions</h2>
    <table class="ledger-table">
      <thead>
        <tr><th>Time</th><th>ID</th><th>Status</th></tr>
      </thead>
      <tbody>
        <tr><td colspan="3" style="text-align:center;color:var(--text-secondary);padding:24px">No transactions yet</td></tr>
      </tbody>
    </table>
  </div>
</main>
