<script lang="ts">
  import ComponentKindPage, { type KindPageProps } from "./ComponentKindPage.svelte";

  let props: KindPageProps = $props();
</script>

<ComponentKindPage kind="skill" {...props} />
