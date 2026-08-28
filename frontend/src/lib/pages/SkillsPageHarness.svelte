<script lang="ts">
  import { createI18n, provideI18n } from "../i18n/locale.svelte";
  import type { KindPageProps } from "./ComponentKindPage.svelte";
  import SkillsPage from "./SkillsPage.svelte";

  type Props = Omit<KindPageProps, "page" | "pageSize" | "onPageChange" | "onPageSizeChange">;

  let props: Props = $props();

  provideI18n(createI18n("en"));
</script>

<SkillsPage {...props} />
