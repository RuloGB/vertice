<script lang="ts">
  import { dismiss, getToasts, type ToastKind } from "./toast.svelte";
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  const toasts = $derived(getToasts());

  function kindClasses(kind: ToastKind): string {
    switch (kind) {
      case "success":
        return "border-emerald-300/30 bg-emerald-500/15 text-emerald-100";
      case "error":
        return "border-red-300/30 bg-red-500/15 text-red-100";
      case "warning":
        return "border-amber-300/30 bg-amber-500/15 text-amber-100";
    }
  }

  function kindIcon(kind: ToastKind): string {
    switch (kind) {
      case "success":
        return "M5 13l4 4L19 7";
      case "error":
        return "M6 6l12 12M6 18L18 6";
      case "warning":
        return "M12 9v4m0 4h.01M12 2L2 20h20L12 2z";
    }
  }
</script>

{#if toasts.length > 0}
  <div class="pointer-events-none fixed right-6 top-16 z-50 flex flex-col gap-3" role="region" aria-label={i18n.t("toast.regionLabel")}>
    {#each toasts as toast (toast.id)}
      <div class="pointer-events-auto flex items-start gap-3 rounded-2xl border px-4 py-3 shadow-lg shadow-black/20 backdrop-blur-sm animate-toast-in {kindClasses(toast.kind)}" role="status">
        <svg class="mt-0.5 size-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d={kindIcon(toast.kind)} />
        </svg>
        <p class="min-w-0 flex-1 text-sm leading-5">{toast.message}</p>
        <button
          type="button"
          class="shrink-0 rounded-lg p-1 opacity-60 transition-opacity hover:opacity-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-200"
          aria-label={i18n.t("toast.dismiss")}
          onclick={() => dismiss(toast.id)}
        >
          <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M6 6l12 12M6 18L18 6" />
          </svg>
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateX(1rem);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  :global(.animate-toast-in) {
    animation: toast-in 0.25s ease-out;
  }
</style>
