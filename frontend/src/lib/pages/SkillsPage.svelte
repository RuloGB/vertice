<script lang="ts">
  import type { Component } from "../../bindings/Component";
  import ComponentKindPage, { type KindPageProps } from "./ComponentKindPage.svelte";
  import SkillDetail from "./SkillDetail.svelte";

  let props: KindPageProps = $props();

  let selectedSkill: Component | null = $state(null);

  function handleSelect(component: Component): void {
    selectedSkill = component;
  }

  function handleBack(): void {
    selectedSkill = null;
  }
</script>

{#if selectedSkill !== null}
  <SkillDetail component={selectedSkill} onBack={handleBack} />
{:else}
  <ComponentKindPage kind="skill" {...props} onComponentSelect={handleSelect} />
{/if}
