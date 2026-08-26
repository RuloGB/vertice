<script lang="ts">
  import type { Component } from "../../bindings/Component";
  import ComponentKindPage, { type KindPageProps } from "./ComponentKindPage.svelte";
  import McpDetail from "./McpDetail.svelte";

  let props: KindPageProps = $props();

  let selectedMcp: Component | null = $state(null);

  function handleSelect(component: Component): void {
    selectedMcp = component;
  }

  function handleBack(): void {
    selectedMcp = null;
  }
</script>

{#if selectedMcp !== null}
  <McpDetail component={selectedMcp} onBack={handleBack} />
{:else}
  <ComponentKindPage kind="mcp" {...props} onComponentSelect={handleSelect} />
{/if}
