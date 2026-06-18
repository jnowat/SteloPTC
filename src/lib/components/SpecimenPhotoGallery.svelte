<script lang="ts">
  import { uploadAttachment, deleteAttachment, getAttachmentData } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let {
    specimenId,
    photos,
    onphotoschanged,
  }: {
    specimenId: string;
    photos: any[];
    onphotoschanged: () => void;
  } = $props();

  let photoCache = $state(new Map<string, string>());
  let uploadingPhoto = $state(false);
  let lightboxSrc = $state<string | null>(null);
  let lightboxMime = $state<string>('image/jpeg');
  let lightboxCloseBtn = $state<HTMLButtonElement | null>(null);
  let fileInputEl = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (lightboxSrc && lightboxCloseBtn) {
      lightboxCloseBtn.focus();
    }
  });

  async function handlePhotoUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    uploadingPhoto = true;
    try {
      const buf = await file.arrayBuffer();
      const bytes = new Uint8Array(buf);
      let binary = '';
      for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
      const b64 = btoa(binary);
      await uploadAttachment('specimen', specimenId, file.name, file.type, b64);
      addNotification('Photo added', 'success');
      onphotoschanged();
    } catch (err: any) {
      addNotification(err.message, 'error');
    } finally {
      uploadingPhoto = false;
      input.value = '';
    }
  }

  async function viewPhoto(id: string, mime: string) {
    let src = photoCache.get(id);
    if (!src) {
      try {
        const b64 = await getAttachmentData(id);
        src = `data:${mime || 'image/jpeg'};base64,${b64}`;
        photoCache.set(id, src);
      } catch (err: any) {
        addNotification(err.message, 'error');
        return;
      }
    }
    lightboxMime = mime || 'image/jpeg';
    lightboxSrc = src;
  }

  async function removePhoto(id: string) {
    if (!confirm('Delete this photo? This cannot be undone.')) return;
    try {
      await deleteAttachment(id);
      photoCache.delete(id);
      addNotification('Photo deleted', 'success');
      onphotoschanged();
    } catch (err: any) {
      addNotification(err.message, 'error');
    }
  }
</script>

<div class="photos-header">
  <h3 style="font-size:15px;">Photos</h3>
  {#if $currentUser?.role !== 'guest'}
    <input
      bind:this={fileInputEl}
      type="file"
      accept="image/*"
      capture="environment"
      style="display:none"
      onchange={handlePhotoUpload}
    />
    <button
      class="btn btn-primary btn-sm"
      onclick={() => fileInputEl?.click()}
      disabled={uploadingPhoto}
      title="Upload a photo or capture from camera (Android)"
    >
      {#if uploadingPhoto}Uploading…{:else}+ Add Photo{/if}
    </button>
  {/if}
</div>

{#if photos.length === 0}
  <div class="empty-state">
    No photos yet.{#if $currentUser?.role !== 'guest'} Use <strong>+ Add Photo</strong> to attach an image.{/if}
  </div>
{:else}
  <div class="photo-grid">
    {#each photos as photo (photo.id)}
      {@const cached = photoCache.get(photo.id)}
      <div class="photo-card" role="button" tabindex="0"
        onclick={() => viewPhoto(photo.id, photo.mime_type)}
        onkeydown={(e) => e.key === 'Enter' && viewPhoto(photo.id, photo.mime_type)}
        title="Click to view full-size — {photo.file_name}"
      >
        <div class="photo-thumb">
          {#if cached}
            <img src={cached} alt={photo.file_name} />
          {:else}
            <span class="photo-icon">&#128247;</span>
          {/if}
        </div>
        <div class="photo-meta">
          <span class="photo-name" title={photo.file_name}>{photo.file_name}</span>
          <span class="photo-date">{photo.created_at?.split(' ')[0] ?? ''}</span>
        </div>
        {#if $currentUser?.role !== 'guest'}
          <button class="photo-delete" title="Delete this photo"
            onclick={(e) => { e.stopPropagation(); removePhoto(photo.id); }}
          >&#10005;</button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<!-- Photo Lightbox -->
{#if lightboxSrc}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="lightbox"
    role="dialog"
    aria-modal="true"
    aria-label="Photo viewer"
    onclick={() => (lightboxSrc = null)}
    onkeydown={(e) => { if (e.key === 'Escape') lightboxSrc = null; }}
    tabindex="-1"
  >
    <button
      class="lightbox-close"
      onclick={() => (lightboxSrc = null)}
      aria-label="Close photo viewer"
      title="Close"
      bind:this={lightboxCloseBtn}
    >&#10005;</button>
    <img src={lightboxSrc} alt="Specimen photo" onclick={(e) => e.stopPropagation()} />
  </div>
{/if}

<style>
  .photos-header {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 16px;
  }
  .photo-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px;
  }
  .photo-card {
    position: relative;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    overflow: hidden;
    cursor: pointer;
    transition: box-shadow 0.15s;
    background: #f8fafc;
  }
  .photo-card:hover { box-shadow: 0 4px 12px rgba(0,0,0,0.12); }
  :global(.dark) .photo-card { border-color: #334155; background: #0f172a; }
  .photo-thumb {
    width: 100%; height: 110px;
    display: flex; align-items: center; justify-content: center;
    background: #f1f5f9; overflow: hidden;
  }
  :global(.dark) .photo-thumb { background: #1e293b; }
  .photo-thumb img { width: 100%; height: 100%; object-fit: cover; }
  .photo-icon { font-size: 40px; opacity: 0.4; }
  .photo-meta {
    padding: 6px 8px;
    display: flex; flex-direction: column; gap: 2px;
  }
  .photo-name {
    font-size: 11px; font-weight: 600; color: #374151;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  :global(.dark) .photo-name { color: #e2e8f0; }
  .photo-date { font-size: 10px; color: #9ca3af; }
  .photo-delete {
    position: absolute; top: 4px; right: 4px;
    width: 22px; height: 22px;
    background: rgba(220,38,38,0.85); color: white;
    border: none; border-radius: 50%; cursor: pointer;
    font-size: 10px; font-weight: 700; line-height: 1;
    display: flex; align-items: center; justify-content: center;
    opacity: 0; transition: opacity 0.15s;
  }
  .photo-card:hover .photo-delete { opacity: 1; }
  .lightbox {
    position: fixed; inset: 0; z-index: 2000;
    background: rgba(0,0,0,0.88);
    display: flex; align-items: center; justify-content: center;
    cursor: zoom-out;
  }
  .lightbox img {
    max-width: 90vw; max-height: 90vh;
    border-radius: 6px; box-shadow: 0 8px 40px rgba(0,0,0,0.6);
    cursor: default;
  }
  .lightbox-close {
    position: absolute; top: 20px; right: 24px;
    background: rgba(255,255,255,0.12); color: white;
    border: 1px solid rgba(255,255,255,0.2);
    width: 40px; height: 40px; border-radius: 50%;
    font-size: 16px; cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    transition: background 0.15s;
  }
  .lightbox-close:hover { background: rgba(255,255,255,0.2); }
</style>
