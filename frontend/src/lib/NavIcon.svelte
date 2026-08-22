<script lang="ts">
  import type { RouteId } from "./navigation";

  // Decorative only: every nav item ships a visible text label next to the icon.
  let { route, class: className = "size-4" }: { route: RouteId; class?: string } = $props();
</script>

<svg
  class={className}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#if route === "home"}
    <path d="M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z" />
  {:else if route === "agents"}
    <rect x="4" y="8" width="16" height="12" rx="3" />
    <path d="M12 4v4M8.5 13.5h.01M15.5 13.5h.01M9.5 17h5" />
  {:else if route === "skills"}
    <path d="m12 3 1.9 4.8L18.7 9.7l-4.8 1.9L12 16.4l-1.9-4.8L5.3 9.7l4.8-1.9z" />
    <path d="m18 16 .8 2.1 2.2.9-2.2.9L18 22l-.8-2.1-2.2-.9 2.2-.9z" />
  {:else if route === "mcp"}
    <path d="M12 3v6M7 9h10v3a5 5 0 0 1-10 0z" />
    <path d="M12 17v4M9 3v2M15 3v2" />
  {:else if route === "prompts"}
    <path d="M21 14a3 3 0 0 1-3 3H9l-5 4V6a3 3 0 0 1 3-3h11a3 3 0 0 1 3 3z" />
    <path d="M8.5 10h7M8.5 13h4" />
  {:else if route === "inventory"}
    <path d="m12 3 8.5 4.5L12 12 3.5 7.5z" />
    <path d="M3.5 12 12 16.5 20.5 12M3.5 16.5 12 21l8.5-4.5" />
  {:else}
    <rect x="2.5" y="5" width="19" height="14" rx="2.5" />
    <path d="M2.5 10h19M6 14.5h4" />
  {/if}
</svg>
