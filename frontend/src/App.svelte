<script lang="ts">
  import { appTitle } from "./lib/appTitle";

  const title = appTitle("Vertice", "0.1.0");
</script>

<main class="flex min-h-screen items-center justify-center bg-slate-950 text-slate-100">
  <h1 class="text-2xl font-semibold">{title}</h1>
</main>
