<script lang="ts">
  import type { McpTransport } from "../bindings/McpTransport";
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  let { transport }: { transport: McpTransport | null } = $props();
</script>

{#if transport === null}
  <p class="text-xs italic leading-5 text-warning">
    {i18n.t("transport.unavailable")}
  </p>
{:else if "stdio" in transport}
  <dl class="flex flex-col gap-1.5 text-xs">
    <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
      <dt class="label-caps text-content-subtle">{i18n.t("transport.stdio")}</dt>
      <dd class="font-semibold text-content">
        {i18n.t("transport.command")}:
        <span class="font-mono">{transport.stdio.command}</span>
      </dd>
      <dd class="text-content-muted">{i18n.t("transport.argCount", { count: transport.stdio.arg_count })}</dd>
    </div>
    {#if transport.stdio.env_keys.length > 0}
      <div class="flex flex-col gap-1">
        <dt class="font-semibold text-content-muted">{i18n.t("transport.envKeys")}</dt>
        <dd class="flex flex-wrap gap-1.5">
          {#each transport.stdio.env_keys as key (key)}
            <span class="rounded-full border border-stroke bg-canvas/45 px-2 py-0.5 font-mono">{key}</span>
          {/each}
        </dd>
        <dd class="italic text-content-subtle">{i18n.t("transport.keysNote")}</dd>
      </div>
    {/if}
  </dl>
{:else}
  <dl class="flex flex-col gap-1.5 text-xs">
    <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
      <dt class="label-caps text-content-subtle">{i18n.t("transport.remote")}</dt>
      <dd class="font-semibold text-content">
        {i18n.t("transport.endpoint")}:
        <span class="font-mono">{transport.remote.url}</span>
      </dd>
    </div>
    {#if transport.remote.header_keys.length > 0}
      <div class="flex flex-col gap-1">
        <dt class="font-semibold text-content-muted">{i18n.t("transport.headerKeys")}</dt>
        <dd class="flex flex-wrap gap-1.5">
          {#each transport.remote.header_keys as key (key)}
            <span class="rounded-full border border-stroke bg-canvas/45 px-2 py-0.5 font-mono">{key}</span>
          {/each}
        </dd>
        <dd class="italic text-content-subtle">{i18n.t("transport.keysNote")}</dd>
      </div>
    {/if}
  </dl>
{/if}
