<script lang="ts">
  interface Props {
    text: string;
    position?: 'top' | 'bottom' | 'left' | 'right';
  }

  let { text, position = 'top' }: Props = $props();

  let visible = $state(false);
  let timer: ReturnType<typeof setTimeout>;

  function show() {
    clearTimeout(timer);
    timer = setTimeout(() => (visible = true), 250);
  }

  function hide() {
    clearTimeout(timer);
    visible = false;
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<span
  class="tip-wrap"
  onmouseenter={show}
  onmouseleave={hide}
  onfocus={show}
  onblur={hide}
  tabindex="0"
  role="button"
  aria-label={text}
>
  <span class="tip-badge" aria-hidden="true">?</span>
  {#if visible}
    <div class="tip-popup tip-{position}" role="tooltip">{text}</div>
  {/if}
</span>

<style>
  .tip-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
    vertical-align: middle;
    margin-left: 5px;
    outline: none;
    cursor: default;
  }

  .tip-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 15px;
    height: 15px;
    border-radius: 50%;
    background: #cbd5e1;
    color: #475569;
    font-size: 9px;
    font-weight: 800;
    line-height: 1;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    transition: background 0.15s, color 0.15s;
    flex-shrink: 0;
    user-select: none;
  }

  :global(.dark) .tip-badge {
    background: #334155;
    color: #94a3b8;
  }

  .tip-wrap:hover .tip-badge,
  .tip-wrap:focus .tip-badge {
    background: #2563eb;
    color: #fff;
  }

  /* Popup bubble */
  .tip-popup {
    position: absolute;
    z-index: 9999;
    background: #1e293b;
    color: #f1f5f9;
    font-size: 12px;
    font-weight: 400;
    line-height: 1.5;
    padding: 7px 10px;
    border-radius: 6px;
    white-space: normal;
    max-width: 220px;
    min-width: 120px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
    pointer-events: none;
    animation: tipFade 0.15s ease;
    border: 1px solid #334155;
    word-break: normal;
  }

  @keyframes tipFade {
    from { opacity: 0; transform: translateY(4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  /* Arrow nub */
  .tip-popup::after {
    content: '';
    position: absolute;
    border: 5px solid transparent;
  }

  /* TOP: popup above, arrow points down */
  .tip-top {
    bottom: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
  }
  .tip-top::after {
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    border-top-color: #1e293b;
    border-bottom: none;
  }

  /* BOTTOM: popup below, arrow points up */
  .tip-bottom {
    top: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
  }
  .tip-bottom::after {
    bottom: 100%;
    left: 50%;
    transform: translateX(-50%);
    border-bottom-color: #1e293b;
    border-top: none;
  }

  /* LEFT: popup to the left */
  .tip-left {
    top: 50%;
    right: calc(100% + 8px);
    transform: translateY(-50%);
  }
  .tip-left::after {
    top: 50%;
    left: 100%;
    transform: translateY(-50%);
    border-left-color: #1e293b;
    border-right: none;
  }

  /* RIGHT: popup to the right */
  .tip-right {
    top: 50%;
    left: calc(100% + 8px);
    transform: translateY(-50%);
  }
  .tip-right::after {
    top: 50%;
    right: 100%;
    transform: translateY(-50%);
    border-right-color: #1e293b;
    border-left: none;
  }
</style>
