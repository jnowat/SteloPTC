<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Html5Qrcode } from 'html5-qrcode';
  import { invoke } from '@tauri-apps/api/core';
  import { token } from '../stores/auth';
  import { get } from 'svelte/store';
  import { navigateTo, selectedSpecimenId, addNotification } from '../stores/app';
  import { searchSpecimens } from '../api';

  interface Props {
    onclose: () => void;
    onscan?: (data: string) => void;
  }

  let { onclose, onscan }: Props = $props();

  let scannerId = 'qr-scanner-' + Math.random().toString(36).slice(2);
  let scanner: Html5Qrcode | null = null;
  let scanning = $state(false);
  let error = $state('');
  let result = $state('');
  let parsedAccession = $state('');
  let parsedSpecies = $state('');
  let navigating = $state(false);
  let cameras = $state<{ id: string; label: string }[]>([]);
  let selectedCamera = $state('');
  let cameraStarted = $state(false);

  onMount(async () => {
    try {
      const devices = await Html5Qrcode.getCameras();
      if (devices.length === 0) {
        error = 'No camera found. Please connect a camera and try again.';
        return;
      }
      cameras = devices.map((d) => ({ id: d.id, label: d.label || `Camera ${d.id}` }));
      // Prefer back/rear camera on mobile
      const backCam = devices.find((d) => /back|rear|environment/i.test(d.label));
      selectedCamera = (backCam ?? devices[0]).id;
      await startScanner();
    } catch (e: any) {
      error = e.message ?? 'Camera access failed. Please grant camera permission.';
    }
  });

  onDestroy(() => {
    stopScanner();
  });

  async function startScanner() {
    if (!selectedCamera) return;
    try {
      scanner = new Html5Qrcode(scannerId);
      await scanner.start(
        { deviceId: { exact: selectedCamera } },
        { fps: 10, qrbox: { width: 240, height: 240 } },
        onScanSuccess,
        () => {}, // ignore per-frame errors
      );
      cameraStarted = true;
      scanning = true;
      error = '';
    } catch (e: any) {
      error = e.message ?? 'Failed to start camera.';
      scanner = null;
    }
  }

  async function stopScanner() {
    if (scanner && cameraStarted) {
      try {
        await scanner.stop();
      } catch (_) {}
      scanner = null;
      cameraStarted = false;
      scanning = false;
    }
  }

  async function switchCamera(id: string) {
    await stopScanner();
    selectedCamera = id;
    result = '';
    parsedAccession = '';
    parsedSpecies = '';
    await startScanner();
  }

  function onScanSuccess(decodedText: string) {
    if (result === decodedText) return; // debounce duplicate scans
    result = decodedText;

    // Try to parse SteloPTC QR JSON payload
    try {
      const payload = JSON.parse(decodedText);
      if (payload.accession) parsedAccession = payload.accession;
      if (payload.species) parsedSpecies = payload.species;
    } catch {
      // Not JSON, try plain accession
      parsedAccession = decodedText.trim();
    }

    // Store scan event
    storeScan(decodedText);

    // Callback
    onscan?.(decodedText);
  }

  async function storeScan(rawData: string) {
    try {
      await invoke('store_qr_scan', {
        token: get(token),
        rawData,
        accessionNumber: parsedAccession || null,
      });
    } catch (_) {
      // Non-critical — scan still usable without storage
    }
  }

  async function navigateToSpecimen() {
    if (!parsedAccession) return;
    navigating = true;
    try {
      const res = await searchSpecimens({ query: parsedAccession, page: 1, per_page: 1 });
      if (res.items.length > 0) {
        const sp = res.items[0];
        selectedSpecimenId.set(sp.id);
        navigateTo('specimen-detail', sp.id);
        onclose();
      } else {
        addNotification(`No specimen found for accession: ${parsedAccession}`, 'error');
      }
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      navigating = false;
    }
  }

  function clearResult() {
    result = '';
    parsedAccession = '';
    parsedSpecies = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={(e) => { if ((e.target as HTMLElement).classList.contains('modal-backdrop')) onclose(); }}>
  <div class="modal" role="dialog" aria-modal="true" aria-label="QR Scanner">
    <div class="modal-header">
      <h2>&#128247; Scan QR Code</h2>
      <button class="close-btn" onclick={onclose} aria-label="Close">&#10005;</button>
    </div>

    <div class="modal-body">
      {#if error}
        <div class="scan-error">
          <span>&#9888;</span>
          <p>{error}</p>
          <button class="btn btn-sm btn-primary" onclick={startScanner}>Retry</button>
        </div>
      {/if}

      <!-- Camera selector -->
      {#if cameras.length > 1}
        <div class="camera-select">
          <label for="cam-select">Camera</label>
          <select id="cam-select" value={selectedCamera} onchange={(e) => switchCamera((e.target as HTMLSelectElement).value)}>
            {#each cameras as cam}
              <option value={cam.id}>{cam.label}</option>
            {/each}
          </select>
        </div>
      {/if}

      <!-- Scanner viewfinder -->
      <div class="viewfinder-wrapper">
        <div id={scannerId} class="viewfinder"></div>
        {#if scanning && !result}
          <div class="scan-indicator">
            <div class="scan-line"></div>
          </div>
        {/if}
      </div>

      <!-- Result -->
      {#if result}
        <div class="result-card">
          <div class="result-header">
            <span class="result-icon">&#10003;</span>
            <strong>QR Code Detected</strong>
            <button class="btn btn-sm" onclick={clearResult}>Clear</button>
          </div>
          {#if parsedAccession}
            <div class="result-row">
              <span class="result-label">Accession</span>
              <span class="result-value mono">{parsedAccession}</span>
            </div>
          {/if}
          {#if parsedSpecies}
            <div class="result-row">
              <span class="result-label">Species</span>
              <span class="result-value">{parsedSpecies}</span>
            </div>
          {/if}
          <div class="result-row">
            <span class="result-label">Raw data</span>
            <span class="result-value raw">{result.length > 80 ? result.slice(0, 80) + '…' : result}</span>
          </div>
          {#if parsedAccession}
            <button class="btn btn-primary" onclick={navigateToSpecimen} disabled={navigating}>
              {navigating ? 'Searching…' : '&#8594; Open Specimen'}
            </button>
          {/if}
        </div>
      {:else if scanning && !error}
        <p class="hint">Point the camera at a SteloPTC QR code</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    z-index: 2000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    animation: fadeIn 0.15s ease;
  }

  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

  .modal {
    background: #fff;
    border-radius: 12px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.4);
    width: 100%;
    max-width: 420px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: slideUp 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
    max-height: 90dvh;
    overflow-y: auto;
  }

  :global(.dark) .modal { background: #1e293b; border: 1px solid #334155; }

  @keyframes slideUp {
    from { opacity: 0; transform: translateY(20px) scale(0.97); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid #e2e8f0;
    flex-shrink: 0;
  }

  :global(.dark) .modal-header { border-bottom-color: #334155; }

  .modal-header h2 { font-size: 18px; font-weight: 700; color: #0f172a; }
  :global(.dark) .modal-header h2 { color: #f1f5f9; }

  .close-btn {
    background: none; border: none; font-size: 16px; color: #94a3b8;
    cursor: pointer; padding: 4px 8px; border-radius: 4px;
    min-height: 44px; min-width: 44px; display: flex; align-items: center; justify-content: center;
  }
  .close-btn:hover { background: #f1f5f9; color: #1e293b; }
  :global(.dark) .close-btn:hover { background: #334155; color: #e2e8f0; }

  .modal-body {
    padding: 16px 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .scan-error {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    background: #fef2f2;
    border: 1px solid #fca5a5;
    border-radius: 8px;
    padding: 16px;
    text-align: center;
    color: #991b1b;
    font-size: 13px;
  }
  .scan-error span { font-size: 24px; }
  :global(.dark) .scan-error { background: rgba(220,38,38,0.1); border-color: #dc2626; color: #fca5a5; }

  .camera-select {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .camera-select label { font-size: 11px; font-weight: 700; color: #6b7280; text-transform: uppercase; }
  .camera-select select { min-height: 44px; }

  .viewfinder-wrapper {
    position: relative;
    width: 100%;
    border-radius: 12px;
    overflow: hidden;
    background: #0f172a;
    min-height: 260px;
  }

  .viewfinder {
    width: 100%;
  }

  /* html5-qrcode injects video + canvas — override defaults */
  .viewfinder :global(video) {
    width: 100% !important;
    height: auto !important;
    border-radius: 0 !important;
  }
  .viewfinder :global(canvas) { display: none; }

  .scan-indicator {
    position: absolute;
    inset: 0;
    pointer-events: none;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .scan-line {
    width: 220px;
    height: 2px;
    background: linear-gradient(90deg, transparent, #2563eb, transparent);
    animation: scanLine 2s ease-in-out infinite;
  }

  @keyframes scanLine {
    0% { transform: translateY(-80px); opacity: 0; }
    20% { opacity: 1; }
    80% { opacity: 1; }
    100% { transform: translateY(80px); opacity: 0; }
  }

  .result-card {
    background: #f0fdf4;
    border: 1px solid #86efac;
    border-radius: 10px;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  :global(.dark) .result-card { background: rgba(34,197,94,0.1); border-color: #16a34a; }

  .result-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .result-header strong { flex: 1; font-size: 14px; color: #15803d; }
  :global(.dark) .result-header strong { color: #4ade80; }
  .result-icon { color: #16a34a; font-size: 18px; font-weight: 900; }

  .result-row { display: flex; gap: 8px; align-items: baseline; }
  .result-label { font-size: 11px; font-weight: 700; color: #6b7280; min-width: 68px; text-transform: uppercase; }
  .result-value { font-size: 13px; color: #1e293b; font-weight: 500; word-break: break-all; }
  :global(.dark) .result-value { color: #e2e8f0; }
  .result-value.raw { color: #64748b; font-size: 11px; }
  .mono { font-family: 'SF Mono', 'Fira Code', Consolas, monospace; }

  .hint { font-size: 13px; color: #94a3b8; text-align: center; }

  @media (max-width: 480px) {
    .modal { max-width: 100%; border-radius: 16px 16px 0 0; position: fixed; bottom: 0; max-height: 95dvh; }
    .modal-backdrop { align-items: flex-end; padding: 0; }
  }
</style>
