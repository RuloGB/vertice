<script lang="ts">
  import { onMount } from "svelte";
  import { appTitle } from "./lib/appTitle";
  import { scan } from "./lib/scan";

  const title = appTitle("Vertice", "0.1.0");

  onMount(() => {
    // T10 smoke: exercise the scan IPC round-trip once. The inventory UI is T11.
    void scan().catch((error: unknown) => {
      console.warn("smoke scan invocation failed", error);
    });
  });
</script>

<main class="flex min-h-screen items-center justify-center bg-slate-950 text-slate-100">
  <h1 class="text-2xl font-semibold">{title}</h1>
</main>
