<script lang="ts">
  import type { Component } from "../../bindings/Component";
  import { getListPageSize, setListPageSize } from "../listPreferences";
  import ComponentKindPage, { type KindPageProps } from "./ComponentKindPage.svelte";
  import AgentDetail from "./AgentDetail.svelte";

  type ParentProps = Omit<KindPageProps, "page" | "pageSize" | "onPageChange" | "onPageSizeChange">;

  let props: ParentProps = $props();

  let selectedAgent: Component | null = $state(null);
  let page = $state(1);
  let pageSize = $state<5 | 10 | 15>(getListPageSize("agents") as 5 | 10 | 15);

  function handleSelect(component: Component): void {
    selectedAgent = component;
  }

  function handleBack(): void {
    selectedAgent = null;
  }

  function handlePageSizeChange(nextPageSize: 5 | 10 | 15): void {
    pageSize = nextPageSize;
    setListPageSize("agents", nextPageSize);
  }
</script>

{#if selectedAgent !== null}
  <AgentDetail component={selectedAgent} onBack={handleBack} />
{:else}
  <ComponentKindPage
    kind="agent"
    {...props}
    {page}
    {pageSize}
    onPageChange={(nextPage) => (page = nextPage)}
    onPageSizeChange={handlePageSizeChange}
    onComponentSelect={handleSelect}
  />
{/if}
