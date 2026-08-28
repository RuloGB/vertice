<script lang="ts">
  import { createI18n, provideI18n } from "../i18n/locale.svelte";
  import type { KindPageProps } from "./ComponentKindPage.svelte";
  import AgentsPage from "./AgentsPage.svelte";
  import McpsPage from "./McpsPage.svelte";
  import SkillsPage from "./SkillsPage.svelte";

  type ListRoute = "agents" | "skills" | "mcp";
  type Props = Omit<KindPageProps, "page" | "pageSize" | "onPageChange" | "onPageSizeChange"> & {
    route: ListRoute;
  };

  let { route, ...props }: Props = $props();

  provideI18n(createI18n("en"));
</script>

{#if route === "agents"}
  <AgentsPage {...props} />
{:else if route === "skills"}
  <SkillsPage {...props} />
{:else}
  <McpsPage {...props} />
{/if}
