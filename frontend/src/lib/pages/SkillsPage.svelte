<script lang="ts">
  import type { Component } from "../../bindings/Component";
  import { getListPageSize, setListPageSize } from "../listPreferences";
  import ComponentKindPage, { type KindPageProps } from "./ComponentKindPage.svelte";
  import SkillDetail from "./SkillDetail.svelte";

  type ParentProps = Omit<KindPageProps, "page" | "pageSize" | "onPageChange" | "onPageSizeChange">;

  let props: ParentProps = $props();

  let selectedSkill: Component | null = $state(null);
  let page = $state(1);
  let pageSize = $state<5 | 10 | 15>(getListPageSize("skills") as 5 | 10 | 15);

  function handleSelect(component: Component): void {
    selectedSkill = component;
  }

  function handleBack(): void {
    selectedSkill = null;
  }

  function handlePageSizeChange(nextPageSize: 5 | 10 | 15): void {
    pageSize = nextPageSize;
    setListPageSize("skills", nextPageSize);
  }
</script>

{#if selectedSkill !== null}
  <SkillDetail component={selectedSkill} onBack={handleBack} />
{:else}
  <ComponentKindPage
    kind="skill"
    {...props}
    {page}
    {pageSize}
    onPageChange={(nextPage) => (page = nextPage)}
    onPageSizeChange={handlePageSizeChange}
    onComponentSelect={handleSelect}
  />
{/if}
