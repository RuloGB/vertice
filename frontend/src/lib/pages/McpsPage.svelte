<script lang="ts">
  import type { Component } from "../../bindings/Component";
  import { getListPageSize, setListPageSize } from "../listPreferences";
  import ComponentKindPage, { type KindPageProps } from "./ComponentKindPage.svelte";
  import McpDetail from "./McpDetail.svelte";

  type ParentProps = Omit<KindPageProps, "page" | "pageSize" | "onPageChange" | "onPageSizeChange">;

  let props: ParentProps = $props();

  let selectedMcp: Component | null = $state(null);
  let page = $state(1);
  let pageSize = $state<5 | 10 | 15>(getListPageSize("mcp") as 5 | 10 | 15);

  function handleSelect(component: Component): void {
    selectedMcp = component;
  }

  function handleBack(): void {
    selectedMcp = null;
  }

  function handlePageSizeChange(nextPageSize: 5 | 10 | 15): void {
    pageSize = nextPageSize;
    setListPageSize("mcp", nextPageSize);
  }
</script>

{#if selectedMcp !== null}
  <McpDetail component={selectedMcp} onBack={handleBack} />
{:else}
  <ComponentKindPage
    kind="mcp"
    {...props}
    {page}
    {pageSize}
    onPageChange={(nextPage) => (page = nextPage)}
    onPageSizeChange={handlePageSizeChange}
    onComponentSelect={handleSelect}
  />
{/if}
