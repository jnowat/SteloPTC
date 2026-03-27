<script lang="ts">
  import { onMount } from 'svelte';
  import QRCode from 'qrcode';
  import Tooltip from './Tooltip.svelte';

  interface Props {
    specimen: {
      accession_number: string;
      species_code?: string;
      genus?: string;
      species_name?: string;
      common_name?: string;
      stage: string;
      location?: string;
      health_status?: number | string;
      initiation_date?: string;
      id: string;
    };
    onclose: () => void;
  }

  let { specimen, onclose }: Props = $props();

  let canvas: HTMLCanvasElement;
  let qrDataUrl = $state('');
  let generating = $state(true);
  let error = $state('');

  const qrPayload = $derived(JSON.stringify({
    app: 'SteloPTC',
    accession: specimen.accession_number,
    species: specimen.species_code
      ? specimen.species_code
      : `${specimen.genus ?? ''} ${specimen.species_name ?? ''}`.trim(),
    stage: specimen.stage,
    location: specimen.location ?? '',
    id: specimen.id,
  }));

  onMount(async () => {
    try {
      qrDataUrl = await QRCode.toDataURL(qrPayload, {
        errorCorrectionLevel: 'M',
        margin: 2,
        width: 256,
        color: { dark: '#1e293b', light: '#ffffff' },
      });
    } catch (e: any) {
      error = e.message ?? 'Failed to generate QR code';
    } finally {
      generating = false;
    }
  });

  function printLabel() {
    const win = window.open('', '_blank', 'width=360,height=540');
    if (!win) return;

    const speciesDisplay = specimen.species_code
      ? specimen.species_code
      : (`${specimen.genus ?? ''} ${specimen.species_name ?? ''}`).trim() || '—';
    const commonName = specimen.common_name ? `<div class="label-common">${specimen.common_name}</div>` : '';
    const stageFormatted = specimen.stage.replace(/_/g, ' ').replace(/\b\w/g, (c: string) => c.toUpperCase());
    const initDate = specimen.initiation_date
      ? `<div class="label-row"><span class="lbl">Initiated</span><span class="val">${specimen.initiation_date}</span></div>`
      : '';
    const location = specimen.location
      ? `<div class="label-row"><span class="lbl">Location</span><span class="val">${specimen.location}</span></div>`
      : '';

    win.document.write(`<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>QR Label – ${specimen.accession_number}</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Arial, sans-serif;
      background: #fff;
      display: flex;
      align-items: flex-start;
      justify-content: center;
      padding: 0;
    }
    .label {
      width: 2in;
      min-height: 3in;
      padding: 0.12in 0.14in;
      border: 1.5px solid #1e293b;
      border-radius: 6px;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 5px;
      page-break-inside: avoid;
    }
    .label-header {
      width: 100%;
      display: flex;
      justify-content: space-between;
      align-items: center;
      border-bottom: 1px solid #e2e8f0;
      padding-bottom: 4px;
      margin-bottom: 2px;
    }
    .label-brand {
      font-size: 7.5px;
      font-weight: 800;
      color: #475569;
      letter-spacing: 0.8px;
      text-transform: uppercase;
    }
    .label-acc {
      font-size: 10px;
      font-weight: 900;
      color: #0f172a;
      letter-spacing: -0.2px;
      font-family: 'SF Mono', 'Consolas', monospace;
    }
    .label-qr {
      margin: 2px 0;
    }
    .label-qr img {
      width: 1.35in;
      height: 1.35in;
      display: block;
      image-rendering: pixelated;
    }
    .label-species {
      font-size: 9.5px;
      font-weight: 800;
      color: #0f172a;
      text-align: center;
      font-style: italic;
    }
    .label-common {
      font-size: 8.5px;
      color: #475569;
      text-align: center;
    }
    .label-rows {
      width: 100%;
      display: flex;
      flex-direction: column;
      gap: 2px;
      margin-top: 2px;
      border-top: 1px solid #e2e8f0;
      padding-top: 4px;
    }
    .label-row {
      display: flex;
      gap: 4px;
      font-size: 7.5px;
      line-height: 1.3;
    }
    .lbl {
      font-weight: 700;
      color: #64748b;
      min-width: 0.45in;
      text-transform: uppercase;
      letter-spacing: 0.3px;
      flex-shrink: 0;
    }
    .val {
      color: #1e293b;
      font-weight: 500;
      word-break: break-word;
    }
    .label-footer {
      margin-top: auto;
      padding-top: 4px;
      font-size: 7px;
      color: #94a3b8;
      letter-spacing: 0.5px;
      text-transform: uppercase;
      border-top: 1px solid #f1f5f9;
      width: 100%;
      text-align: center;
    }
    @media print {
      @page {
        size: 2in 3in;
        margin: 0;
      }
      body { padding: 0; background: #fff; }
      .label {
        width: 2in;
        min-height: 3in;
        border-color: #000;
        border-radius: 0;
      }
    }
  </style>
</head>
<body>
  <div class="label">
    <div class="label-header">
      <span class="label-brand">SteloPTC</span>
      <span class="label-acc">${specimen.accession_number}</span>
    </div>
    <div class="label-qr">
      <img src="${qrDataUrl}" alt="QR Code" />
    </div>
    <div class="label-species">${speciesDisplay}</div>
    ${commonName}
    <div class="label-rows">
      <div class="label-row"><span class="lbl">Stage</span><span class="val">${stageFormatted}</span></div>
      ${initDate}
      ${location}
    </div>
    <div class="label-footer">SteloPTC · Tissue Culture Management</div>
  </div>
  <script>window.onload = function() { window.print(); window.close(); };<\/script>
</body>
</html>`);
    win.document.close();
  }

  function downloadQr() {
    const a = document.createElement('a');
    a.href = qrDataUrl;
    a.download = `QR-${specimen.accession_number}.png`;
    a.click();
  }

  function handleBackdropClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('modal-backdrop')) onclose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleBackdropClick}>
  <div class="modal" role="dialog" aria-modal="true" aria-label="QR Code">
    <div class="modal-header">
      <h2>QR Code <Tooltip text="QR code encodes the accession number, species, stage, and location. Scan with any QR reader or use SteloPTC's built-in scanner." position="bottom" /></h2>
      <button title="Close this QR code modal" class="close-btn" onclick={onclose} aria-label="Close">&#10005;</button>
    </div>

    <div class="modal-body">
      {#if generating}
        <div class="qr-placeholder"><div class="spinner"></div></div>
      {:else if error}
        <div class="qr-error">{error}</div>
      {:else}
        <div class="qr-wrapper">
          <img title="QR code encoding accession number, species, stage, location, and specimen ID for {specimen.accession_number}" class="qr-image" src={qrDataUrl} alt="QR code for {specimen.accession_number}" />
        </div>
      {/if}

      <div class="specimen-info">
        <div class="info-row">
          <span title="Unique accession number identifying this specimen" class="info-label">Accession</span>
          <span title="The specimen's unique accession number — also encoded in the QR code" class="info-value mono">{specimen.accession_number}</span>
        </div>
        <div class="info-row">
          <span title="Species identifier: either the species code or the full genus and species name" class="info-label">Species</span>
          <span title="Species code or genus/species name encoded in this QR label" class="info-value">
            {specimen.species_code
              ? specimen.species_code
              : (`${specimen.genus ?? ''} ${specimen.species_name ?? ''}`).trim() || '—'}
          </span>
        </div>
        <div class="info-row">
          <span title="Current growth or lifecycle stage of the specimen" class="info-label">Stage</span>
          <span title="Specimen lifecycle stage encoded in this QR label" class="info-value">{specimen.stage}</span>
        </div>
        {#if specimen.location}
          <div class="info-row">
            <span title="Physical storage location of this specimen" class="info-label">Location</span>
            <span title="Storage location encoded in this QR label" class="info-value">{specimen.location}</span>
          </div>
        {/if}
      </div>
    </div>

    <div class="modal-footer">
      <button title="Print a 2×3-inch adhesive label with the QR code and specimen details" class="btn btn-primary" onclick={printLabel} disabled={!qrDataUrl}>
        &#128424; Print Label
      </button>
      <button title="Save the QR code as a PNG image file to your device" class="btn" onclick={downloadQr} disabled={!qrDataUrl}>
        &#8659; Download PNG
      </button>
      <button title="Close this QR code modal without taking any action" class="btn" onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
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
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.35);
    width: 100%;
    max-width: 380px;
    display: flex;
    flex-direction: column;
    animation: slideUp 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
    overflow: hidden;
  }

  :global(.dark) .modal {
    background: #1e293b;
    border: 1px solid #334155;
  }

  @keyframes slideUp {
    from { opacity: 0; transform: translateY(20px) scale(0.96); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid #e2e8f0;
  }

  :global(.dark) .modal-header { border-bottom-color: #334155; }

  .modal-header h2 {
    font-size: 18px;
    font-weight: 700;
    color: #0f172a;
  }

  :global(.dark) .modal-header h2 { color: #f1f5f9; }

  .close-btn {
    background: none;
    border: none;
    font-size: 16px;
    color: #94a3b8;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
    min-height: 44px;
    min-width: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close-btn:hover { background: #f1f5f9; color: #1e293b; }
  :global(.dark) .close-btn:hover { background: #334155; color: #e2e8f0; }

  .modal-body {
    padding: 24px 20px 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
  }

  .qr-wrapper {
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 12px;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
  }

  .qr-image {
    width: 200px;
    height: 200px;
    display: block;
    image-rendering: pixelated;
  }

  .qr-placeholder {
    width: 200px;
    height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed #d1d5db;
    border-radius: 8px;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #e2e8f0;
    border-top-color: #2563eb;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .qr-error {
    color: #dc2626;
    font-size: 13px;
    padding: 16px;
    text-align: center;
  }

  .specimen-info {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .info-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .info-label {
    font-size: 11px;
    font-weight: 700;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    min-width: 70px;
    flex-shrink: 0;
  }

  .info-value {
    font-size: 13px;
    color: #1e293b;
    font-weight: 500;
  }

  :global(.dark) .info-value { color: #e2e8f0; }

  .mono { font-family: 'SF Mono', 'Fira Code', Consolas, monospace; font-size: 12px; }

  .modal-footer {
    display: flex;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid #e2e8f0;
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  :global(.dark) .modal-footer { border-top-color: #334155; }

  @media (max-width: 480px) {
    .modal { max-width: 100%; border-radius: 16px 16px 0 0; position: fixed; bottom: 0; }
    .modal-backdrop { align-items: flex-end; padding: 0; }
    .modal-footer { justify-content: stretch; }
    .modal-footer :global(.btn) { flex: 1; justify-content: center; min-height: 48px; }
  }
</style>
