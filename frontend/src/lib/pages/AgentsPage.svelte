<script lang="ts">
  import type { Component } from "../../bindings/Component";
  import ComponentKindPage, { type KindPageProps } from "./ComponentKindPage.svelte";
  import AgentDetail from "./AgentDetail.svelte";

  let props: KindPageProps = $props();

  let selectedAgent: Component | null = $state(null);

  function handleSelect(component: Component): void {
    selectedAgent = component;
  }

  function handleBack(): void {
    selectedAgent = null;
  }
</script>

{#if selectedAgent !== null}
  <AgentDetail component={selectedAgent} onBack={handleBack} />
{:else}
  <ComponentKindPage kind="agent" {...props} onComponentSelect={handleSelect} />
{/if}
