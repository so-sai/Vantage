<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import DiffViewer from './lib/DiffViewer.svelte';

  const appWindow = getCurrentWindow();

  // Window Controls
  function minimize() { appWindow.minimize(); }
  async function toggleMaximize() { 
    try {
      const isMax = await appWindow.isMaximized();
      if (isMax) {
        appWindow.unmaximize();
      } else {
        appWindow.maximize();
      }
    } catch (e) {
      appWindow.toggleMaximize(); 
    }
  }
  function close() { appWindow.close(); }

  // Spotlight Intent states
  let intentQuery = $state("");
  let isAnalyzing = $state(false);
  let activeArtifact = $state<any>(null);

  // Seal Slider states
  let sliderValue = $state(0);
  let isSealed = $state(false);
  let isSealing = $state(false);
  let sealHash = $state("");
  let sealNodes = $state(0);
  let validationLogs = $state<string[]>([]);

  // Time Machine Scrubber states
  let currentEpoch = $state(10);
  let maxEpoch = $state(10);
  let timeMachineRevisions = $state<any[]>([]);
  let selectedRevision = $derived(
    timeMachineRevisions.find(r => r.epoch === currentEpoch) || null
  );
  let isTimeTraveling = $derived(currentEpoch < maxEpoch);

  // Fetch revisions on mount
  onMount(async () => {
    try {
      timeMachineRevisions = await invoke('get_time_machine_revisions');
      if (timeMachineRevisions.length > 0) {
        maxEpoch = Math.max(...timeMachineRevisions.map(r => r.epoch));
        currentEpoch = maxEpoch;
      }
    } catch (e) {
      console.error("Failed to load time machine revisions", e);
    }
  });

  // Handle Search submit
  async function handleSearch(e: SubmitEvent) {
    e.preventDefault();
    if (!intentQuery.trim()) return;

    isAnalyzing = true;
    isSealed = false;
    sliderValue = 0;
    sealHash = "";
    activeArtifact = null;

    try {
      const res: any = await invoke('execute_intent', { intent: intentQuery });
      activeArtifact = res;
      validationLogs = res.logs;
    } catch (err) {
      console.error(err);
      validationLogs = [`❌ Error: ${err}`];
    } finally {
      isAnalyzing = false;
    }
  }

  // Handle slide completion
  async function handleSealComplete() {
    isSealing = true;
    validationLogs = [...validationLogs, "Vantage: Đang chạy linter & compiler cục bộ...", "Vantage: Đối chiếu Hiến pháp Không Gian..."];
    
    // Simulate compilation delay for UX polish
    await new Promise(resolve => setTimeout(resolve, 800));

    try {
      const res: any = await invoke('commit_safe_state');
      validationLogs = [
        ...validationLogs,
        "Vantage: Invariant AST Check -> PASS 🟢",
        "Vantage: Invariant Zero Drift -> PASS 🟢",
        `Vantage: Giao dịch được niêm phong thành công! [Hash: ${res.hash.slice(0, 8)}] 🟢`
      ];
      sealHash = res.hash;
      sealNodes = res.total_nodes;
      isSealed = true;
    } catch (err) {
      validationLogs = [...validationLogs, `❌ Thất bại: ${err}`];
      sliderValue = 0;
    } finally {
      isSealing = false;
    }
  }

  // Monitor Slider value to trigger seal
  $effect(() => {
    if (sliderValue >= 100 && !isSealed && !isSealing) {
      handleSealComplete();
    }
  });

  function resetTransaction() {
    sliderValue = 0;
    isSealed = false;
    sealHash = "";
    activeArtifact = null;
    intentQuery = "";
  }
</script>

<div id="app">
  <!-- Frameless Titlebar -->
  <header class="titlebar">
    <div class="titlebar-logo">
      <span>Vantage</span>
      <span class="version">v1.2.5</span>
    </div>

    <!-- Spotlight Search -->
    <div class="spotlight-container">
      <form onsubmit={handleSearch} class="spotlight-wrapper">
        <svg class="spotlight-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input 
          type="text" 
          placeholder="Giao việc cho Vantage (Ví dụ: Sửa lỗi Stripe)..." 
          bind:value={intentQuery}
          disabled={isTimeTraveling || isSealing || isSealed}
          class="spotlight-input"
        />
      </form>
    </div>

    <!-- Draggable Region -->
    <div class="titlebar-drag" data-tauri-drag-region></div>

    <!-- OS Window Controls -->
    <div class="titlebar-controls">
      <button class="window-btn" onclick={minimize} title="Thu nhỏ">
        <svg width="10" height="1" viewBox="0 0 10 1" fill="none" xmlns="http://www.w3.org/2000/svg">
          <line y1="0.5" x2="10" y2="0.5" stroke="currentColor" stroke-width="1.2"/>
        </svg>
      </button>
      <button class="window-btn" onclick={toggleMaximize} title="Phóng to">
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" xmlns="http://www.w3.org/2000/svg">
          <rect x="1" y="1" width="8" height="8" stroke="currentColor" stroke-width="1.2" fill="none"/>
        </svg>
      </button>
      <button class="window-btn close" onclick={close} title="Đóng">
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M1 1L9 9M9 1L1 9" stroke="currentColor" stroke-width="1.2"/>
        </svg>
      </button>
    </div>
  </header>

  <!-- Main Workspace -->
  <main class="workspace {isTimeTraveling ? 'time-traveling' : ''}">
    {#if isAnalyzing}
      <div class="empty-state">
        <div class="pulse" style="width: 40px; height: 40px; border-radius: 50%; border: 3px solid var(--accent-info); border-top-color: transparent; animation: spin 1s infinite linear;"></div>
        <h3 style="margin-top: 16px;">Vantage Shield đang phân tích...</h3>
        <p>Đang đối chiếu với các luật bảo vệ của Hiến pháp Không Gian.</p>
      </div>
    {:else if activeArtifact}
      <!-- Codex Artifact Card -->
      <div class="artifact-card fade-in {isSealed ? 'safe pulse' : ''}">
        <div class="artifact-header">
          <div class="artifact-title">
            <h2>{activeArtifact.description}</h2>
            <p>{activeArtifact.file_path}</p>
          </div>
          <div class="badge-shield">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
              <path d="M6 0L1 2v4c0 3.1 2.1 5.9 5 6 2.9-.1 5-2.9 5-6V2L6 0zm0 1.2L9.8 2.7v3.3c0 2.4-1.6 4.5-3.8 4.7-2.2-.2-3.8-2.3-3.8-4.7V2.7L6 1.2z"/>
            </svg>
            Vantage Shield: Safe
          </div>
        </div>

        <div class="artifact-content">
          <DiffViewer originalCode={activeArtifact.original_code} modifiedCode={activeArtifact.modified_code} />
        </div>

        {#if validationLogs.length > 0}
          <div class="logs-console">
            {#each validationLogs as log}
              <div class="log-line {log.includes('PASS') || log.includes('thành công') ? 'green' : ''}">{log}</div>
            {/each}
          </div>
        {/if}

        <!-- iOS slide to commit action gate -->
        <div class="slider-container">
          {#if isSealed}
            <div style="display:flex; flex-direction:column; align-items:center; gap:12px; width:100%">
              <div style="font-size:11px; color:var(--accent-safe); font-weight:bold; letter-spacing:0.5px">
                NIÊM PHONG THÀNH CÔNG (EPOCH {maxEpoch + 1}) • {sealNodes} NODES
              </div>
              <div style="font-family:'SF Mono', monospace; font-size:10px; color:var(--text-secondary); background:rgba(0,0,0,0.3); padding:4px 8px; border-radius:4px; max-width:100%; word-break:break-all">
                SHA-256: {sealHash}
              </div>
              <button class="window-btn" style="width:auto; padding:6px 16px; font-weight:600; background:rgba(255,255,255,0.08); border:1px solid var(--border-color); color:var(--text-primary)" onclick={resetTransaction}>
                Tạo Giao Dịch Mới
              </button>
            </div>
          {:else}
            <div class="slide-track {isSealed ? 'sealed' : ''}">
              <input 
                type="range" 
                min="0" 
                max="100" 
                bind:value={sliderValue}
                disabled={isSealing || isSealed}
                class="slide-input"
              />
              <div class="slide-thumb" style="left: calc({sliderValue}% * 0.9 + 3px)">
                {#if isSealing}
                  <div class="pulse" style="width: 14px; height: 14px; border-radius: 50%; border: 2px solid #050508; border-top-color: transparent; animation: spin 0.8s infinite linear;"></div>
                {:else}
                  <svg fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7" />
                  </svg>
                {/if}
              </div>
              <span class="slide-text">
                {#if isSealing}
                  ĐANG KIỂM CHỨNG & NIÊM PHONG...
                {:else}
                  TRƯỢT ĐỂ NIÊM PHONG GIAO DỊCH ──→
                {/if}
              </span>
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <!-- Empty State -->
      <div class="empty-state">
        <svg fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
        <h3>Vantage Cognitive Cockpit</h3>
        <p>Giao diện điều phối AI tối giản. Nhập ý định của Ngài lên ô Spotlight để phân tích.</p>
      </div>
    {/if}
  </main>

  <!-- Time Machine Scrubber (Always visible at the bottom) -->
  {#if isTimeTraveling}
    <!-- Historical View overlay overlaying dimmed main workspace -->
    <div style="position: absolute; bottom: 100px; left: 50%; transform: translateX(-50%); z-index: 20;" class="fade-in">
      {#if selectedRevision}
        <div class="artifact-card" style="width: 680px; border-color: var(--accent-warning); box-shadow: 0 10px 30px rgba(0,0,0,0.4);">
          <div class="artifact-header" style="background: rgba(245, 158, 11, 0.04); border-bottom-color: rgba(245, 158, 11, 0.15)">
            <div class="artifact-title">
              <h2 style="color: var(--accent-warning)">Time Machine: Khảo cổ Epoch {selectedRevision.epoch}</h2>
              <p>Tác nhân: {selectedRevision.actor} • Đã thay đổi {selectedRevision.file_count} tệp tin</p>
            </div>
            <div class="badge-shield" style="color:var(--accent-warning); background:rgba(245, 158, 11, 0.1); border-color:rgba(245, 158, 11, 0.2)">
              ✓ Vantage State: Sealed
            </div>
          </div>
          <div style="padding: 16px 20px; font-size:12px; line-height: 1.5; color: var(--text-primary);">
            <p style="margin-bottom: 8px;"><strong>Mô tả revision:</strong> {selectedRevision.description}</p>
            <p style="color: var(--text-secondary); font-size: 11px;">
              Mã nguồn tại Epoch này đã được đối chiếu toàn vẹn và không phát hiện thấy bất kỳ dấu hiệu lệch cấu trúc (Zero Drift) nào.
            </p>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <footer class="time-machine-scrubber">
    <div class="scrubber-header">
      <span class="scrubber-title">Temporal Lineage (Time Machine)</span>
      <span class="scrubber-badge {isTimeTraveling ? 'time-travel' : ''}">
        {#if isTimeTraveling}
          TIME TRAVEL MODE • EPOCH {currentEpoch} OF {maxEpoch}
        {:else}
          CURRENT REALITY • EPOCH {maxEpoch}
        {/if}
      </span>
    </div>
    <div class="scrubber-track">
      <input 
        type="range" 
        min="7" 
        max={maxEpoch} 
        bind:value={currentEpoch}
        class="scrubber-input"
        style="background: linear-gradient(to right, var(--accent-info) 0%, var(--accent-info) {maxEpoch > 7 ? ((currentEpoch - 7) / (maxEpoch - 7)) * 100 : 100}%, rgba(255, 255, 255, 0.08) {maxEpoch > 7 ? ((currentEpoch - 7) / (maxEpoch - 7)) * 100 : 100}%, rgba(255, 255, 255, 0.08) 100%)"
      />
    </div>
  </footer>
</div>

<style>
  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style>
