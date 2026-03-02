<script lang="ts">
  import { onDestroy } from 'svelte';
  import { getSpecimenByAccession } from '../api';
  import { navigateTo, addNotification, selectedSpecimenId } from '../stores/app';

  let {
    mode = 'generate',
    accessionNumber = '',
    speciesName = '',
    onclose,
  }: {
    mode: 'generate' | 'scan';
    accessionNumber?: string;
    speciesName?: string;
    onclose: () => void;
  } = $props();

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let qrError = $state('');
  let scanning = $state(false);
  let scanResult = $state('');
  let printReady = $state(false);
  let scannerInstance: any = null;

  const qrData = accessionNumber ? `STELO:${accessionNumber}` : '';

  $effect(() => {
    if (mode === 'generate' && canvasEl && qrData) {
      generateQR();
    }
  });

  $effect(() => {
    if (mode === 'scan') {
      startScan();
    }
  });

  async function generateQR() {
    qrError = '';
    try {
      const QRCode = (await import('qrcode')).default;
      await QRCode.toCanvas(canvasEl, qrData, {
        width: 280,
        errorCorrectionLevel: 'M',
        color: { dark: '#1e293b', light: '#ffffff' },
      });
      printReady = true;
    } catch (e: any) {
      qrError = e.message || 'Failed to generate QR code';
    }
  }

  async function startScan() {
    if (scanning || scannerInstance) return;
    scanning = true;
    qrError = '';
    try {
      const { Html5Qrcode } = await import('html5-qrcode');
      scannerInstance = new Html5Qrcode('qr-reader');
      await scannerInstance.start(
        { facingMode: 'environment' },
        { fps: 10, qrbox: { width: 250, height: 250 } },
        onScanSuccess,
        () => {} // ignore scan frame errors
      );
    } catch (e: any) {
      qrError = e.message || 'Camera access denied or unavailable';
      scanning = false;
    }
  }

  async function onScanSuccess(decodedText: string) {
    await stopScan();
    scanResult = decodedText;
    if (decodedText.startsWith('STELO:')) {
      const accession = decodedText.slice(6);
      try {
        const specimen = await getSpecimenByAccession(accession);
        if (specimen) {
          selectedSpecimenId.set(specimen.id);
          navigateTo('specimen-detail', specimen.id);
          onclose();
        } else {
          addNotification(`No active specimen found for: ${accession}`, 'warning');
        }
      } catch (e: any) {
        addNotification(e.message, 'error');
      }
    } else {
      addNotification(`Unrecognized QR code: ${decodedText}`, 'warning');
    }
  }

  async function stopScan() {
    if (scannerInstance) {
      try {
        await scannerInstance.stop();
        scannerInstance.clear();
      } catch {}
      scannerInstance = null;
    }
    scanning = false;
  }

  function handlePrint() {
    window.print();
  }

  function downloadQR() {
    if (!canvasEl) return;
    const link = document.createElement('a');
    link.download = `QR-${accessionNumber}.png`;
    link.href = canvasEl.toDataURL('image/png');
    link.click();
  }

  function handleClose() {
    stopScan();
    onclose();
  }

  onDestroy(() => {
    stopScan();
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleClose}>
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <h2>{mode === 'generate' ? 'QR Code' : 'Scan QR Code'}</h2>
      <button class="close-btn" onclick={handleClose} aria-label="Close">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
          <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>

    <div class="modal-body">
      {#if mode === 'generate'}
        <div class="qr-generate">
          {#if qrError}
            <p class="error-msg">{qrError}</p>
          {:else}
            <!-- Print-visible label -->
            <div class="print-label">
              <canvas bind:this={canvasEl}></canvas>
              <div class="label-info">
                <div class="label-accession">{accessionNumber}</div>
                {#if speciesName}
                  <div class="label-species">{speciesName}</div>
                {/if}
                <div class="label-data">{qrData}</div>
              </div>
            </div>
          {/if}

          {#if printReady}
            <div class="modal-actions">
              <button class="btn btn-primary" onclick={handlePrint}>&#128438; Print Label</button>
              <button class="btn" onclick={downloadQR}>&#8681; Download PNG</button>
              <button class="btn" onclick={handleClose}>Close</button>
            </div>
          {/if}
        </div>

      {:else}
        <!-- Scan mode -->
        <div class="qr-scan">
          <p class="scan-hint">Point the camera at a SteloPTC QR label.</p>
          <div id="qr-reader" class="scan-viewport"></div>
          {#if qrError}
            <p class="error-msg">{qrError}</p>
          {/if}
          {#if scanResult && !scanResult.startsWith('STELO:')}
            <p class="scan-result">Scanned: {scanResult}</p>
          {/if}
          <div class="modal-actions">
            <button class="btn" onclick={handleClose}>Cancel</button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 2000;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(3px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    animation: fadeIn 0.15s ease;
  }
  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .modal {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    width: 100%;
    max-width: 360px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    animation: slideUp 0.18s cubic-bezier(0.4, 0, 0.2, 1);
  }
  @keyframes slideUp {
    from { transform: translateY(20px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid #334155;
  }
  .modal-header h2 {
    font-size: 16px;
    font-weight: 700;
    color: #f1f5f9;
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    color: #64748b;
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 36px;
    min-height: 36px;
  }
  .close-btn:hover { background: #334155; color: #e2e8f0; }

  .modal-body {
    padding: 20px;
  }

  /* ── Generate mode ── */
  .qr-generate {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .print-label {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    background: white;
    padding: 16px;
    border-radius: 8px;
  }
  .print-label canvas {
    display: block;
  }
  .label-info {
    text-align: center;
  }
  .label-accession {
    font-size: 15px;
    font-weight: 700;
    color: #1e293b;
    letter-spacing: 0.5px;
  }
  .label-species {
    font-size: 12px;
    color: #64748b;
    margin-top: 2px;
    font-style: italic;
  }
  .label-data {
    font-size: 10px;
    color: #94a3b8;
    margin-top: 4px;
    font-family: monospace;
  }

  /* ── Scan mode ── */
  .qr-scan {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }
  .scan-hint {
    font-size: 13px;
    color: #94a3b8;
    text-align: center;
  }
  .scan-viewport {
    width: 100%;
    min-height: 280px;
    border-radius: 8px;
    overflow: hidden;
    background: #0f172a;
  }
  .scan-result {
    font-size: 12px;
    color: #94a3b8;
    word-break: break-all;
  }

  /* ── Shared ── */
  .modal-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: center;
    width: 100%;
  }
  .error-msg {
    color: #f87171;
    font-size: 13px;
    text-align: center;
  }

  /* ── Print styles ── */
  @media print {
    :global(body > *:not(.modal-backdrop)) {
      display: none !important;
    }
    .modal-backdrop {
      position: static;
      background: none;
      backdrop-filter: none;
      padding: 0;
      animation: none;
    }
    .modal {
      border: none;
      box-shadow: none;
      background: white;
      max-width: none;
      animation: none;
    }
    .modal-header,
    .modal-actions,
    .scan-hint,
    .close-btn {
      display: none !important;
    }
    .modal-body {
      padding: 0;
    }
    .print-label {
      background: white;
      page-break-inside: avoid;
    }
    .label-accession { color: #000 !important; }
    .label-species { color: #333 !important; }
    .label-data { color: #666 !important; }
  }
</style>
