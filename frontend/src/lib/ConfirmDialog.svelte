<script lang="ts">
  let {
    open,
    title,
    body,
    confirmLabel,
    cancelLabel,
    onConfirm,
    onCancel,
  }: {
    open: boolean;
    title: string;
    body: string;
    confirmLabel: string;
    cancelLabel: string;
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
    }
  }
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    role="dialog"
    aria-modal="true"
    aria-labelledby="confirm-dialog-title"
    tabindex="-1"
    onkeydown={handleKeydown}
  >
    <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" onclick={onCancel} aria-hidden="true"></div>
    <div class="relative w-full max-w-sm rounded-3xl border border-white/10 bg-slate-900/95 p-6 shadow-2xl shadow-black/40">
      <h2 id="confirm-dialog-title" class="text-lg font-semibold text-white">{title}</h2>
      <p class="mt-2 text-sm leading-6 text-mist-300">{body}</p>
      <div class="mt-6 flex justify-end gap-3">
        <button
          type="button"
          class="rounded-xl border border-white/15 px-4 py-2.5 text-sm font-semibold text-mist-100 transition-colors hover:bg-white/10 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-200"
          onclick={onCancel}
        >
          {cancelLabel}
        </button>
        <button
          type="button"
          class="rounded-xl border border-red-300/40 bg-red-500/15 px-4 py-2.5 text-sm font-semibold text-red-100 transition-colors hover:border-red-200/70 hover:bg-red-500/25 hover:text-red-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-200"
          onclick={onConfirm}
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}
