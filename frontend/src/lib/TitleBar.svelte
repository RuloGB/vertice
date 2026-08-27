<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  let appWindow: ReturnType<typeof getCurrentWindow> | null = null;
  try {
    appWindow = getCurrentWindow();
  } catch {
    // Not running inside Tauri (tests, plain browser). Controls are no-ops.
  }

  function minimize(): void {
    appWindow?.minimize();
  }

  function toggleMaximize(): void {
    appWindow?.toggleMaximize();
  }

  function close(): void {
    appWindow?.close();
  }
</script>

<header
  class="titlebar flex h-10 shrink-0 items-center justify-between border-b border-stroke bg-surface"
>
  <div data-tauri-drag-region class="titlebar-drag flex flex-1 items-center gap-2 px-4">
    <span class="text-xs font-semibold text-content-subtle">Vertice</span>
  </div>

  <div class="titlebar-controls flex h-full">
    <button
      type="button"
      class="titlebar-btn"
      aria-label="Minimize"
      onclick={minimize}
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
    </button>

    <button
      type="button"
      class="titlebar-btn"
      aria-label="Maximize"
      onclick={toggleMaximize}
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
      </svg>
    </button>

    <button
      type="button"
      class="titlebar-btn titlebar-btn-close"
      aria-label="Close"
      onclick={close}
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  </div>
</header>
