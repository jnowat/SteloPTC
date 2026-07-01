<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    label,
    fieldId,
    required = false,
    title = undefined,
    children,
  }: {
    label: string;
    fieldId: string;
    required?: boolean;
    title?: string;
    children?: Snippet;
  } = $props();
</script>

<label for={fieldId} {title}>{label}{#if required}<span aria-hidden="true"> *</span>{/if}</label>
{#if children}
  {@render children()}
{/if}
