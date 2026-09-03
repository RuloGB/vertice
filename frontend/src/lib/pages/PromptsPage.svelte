<script lang="ts">
  import type { Prompt } from "../../bindings/Prompt";
  import type { PromptDraft } from "../../bindings/PromptDraft";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import { useI18n } from "../i18n/locale.svelte";
  import { getListPageSize, setListPageSize } from "../listPreferences";
  import { promptBodyPreview } from "../promptPreview";
  import { createPrompt, deletePrompt, fetchPrompts, isPromptError, updatePrompt } from "../prompts";
  import { filterPrompts } from "../promptSearch";
  import * as toast from "../toast.svelte";

  type LoadStatus = "loading" | "ready" | "failed";
  type RetryableFailure = "load" | "save" | "delete" | null;

  const i18n = useI18n();

  let status = $state<LoadStatus>("loading");
  let prompts = $state<Prompt[]>([]);
  let query = $state("");
  let editing = $state<Prompt | null>(null);
  let formOpen = $state(false);
  let title = $state("");
  let body = $state("");
  let tags = $state("");
  let bestForContext = $state("");
  let errors = $state<string[]>([]);
  let saving = $state(false);
  let retryableFailure = $state<RetryableFailure>(null);
  let deleteDialogOpen = $state(false);
  let pendingDelete = $state<Prompt | null>(null);

  const visiblePrompts = $derived(filterPrompts(prompts, query));
  const PAGE_SIZES = [5, 10, 15] as const;

  let page = $state(1);
  let pageSize = $state<(typeof PAGE_SIZES)[number]>(getListPageSize("prompts") as (typeof PAGE_SIZES)[number]);
  let previousQuery = $state("");

  const pageCount = $derived(Math.max(1, Math.ceil(visiblePrompts.length / pageSize)));
  const pageStart = $derived((page - 1) * pageSize);
  const pagePrompts = $derived(visiblePrompts.slice(pageStart, pageStart + pageSize));
  const rangeStart = $derived(visiblePrompts.length === 0 ? 0 : pageStart + 1);
  const rangeEnd = $derived(Math.min(pageStart + pagePrompts.length, visiblePrompts.length));
  const formTitle = $derived(editing === null ? i18n.t("prompts.createTitle") : i18n.t("prompts.editTitle"));
  const failureTitle = $derived(i18n.t(`prompts.${retryableFailure === "load" || status === "failed" ? "failureTitle" : retryableFailure === "delete" ? "deleteFailed" : "saveFailed"}`));
  const failureBody = $derived(i18n.t(`prompts.${retryableFailure === "load" || status === "failed" ? "failureBody" : "mutationFailureBody"}`));

  void loadPrompts();

  $effect(() => {
    if (query !== previousQuery) {
      page = 1;
      previousQuery = query;
    }
  });

  $effect(() => {
    if (page > pageCount) {
      page = pageCount;
    }
  });

  function onPageSizeChange(event: Event): void {
    pageSize = Number((event.currentTarget as HTMLSelectElement).value) as (typeof PAGE_SIZES)[number];
    setListPageSize("prompts", pageSize);
  }

  async function loadPrompts(): Promise<void> {
    status = "loading";
    try {
      prompts = await fetchPrompts();
      status = "ready";
      retryableFailure = null;
    } catch {
      status = "failed";
      retryableFailure = "load";
    }
  }

  function openCreateForm(): void {
    editing = null;
    title = "";
    body = "";
    tags = "";
    bestForContext = "";
    errors = [];
    retryableFailure = null;
    formOpen = true;
  }

  function openEditForm(prompt: Prompt): void {
    editing = prompt;
    title = prompt.title;
    body = prompt.body;
    tags = prompt.tags.join(", ");
    bestForContext = prompt.bestForContext ?? "";
    errors = [];
    retryableFailure = null;
    formOpen = true;
  }

  function closeForm(): void {
    formOpen = false;
    editing = null;
    errors = [];
  }

  function validate(): boolean {
    const nextErrors = [];
    if (title.trim() === "") nextErrors.push(i18n.t("prompts.titleRequired"));
    if (body.trim() === "") nextErrors.push(i18n.t("prompts.bodyRequired"));
    errors = nextErrors;
    return nextErrors.length === 0;
  }

  function draftFromForm(): PromptDraft {
    return {
      title: title.trim(),
      body: body.trim(),
      tags: tags.split(",").map((tag) => tag.trim()).filter(Boolean),
      bestForContext: bestForContext.trim() === "" ? null : bestForContext.trim(),
    };
  }

  async function savePrompt(): Promise<void> {
    retryableFailure = null;
    if (!validate()) return;
    saving = true;
    try {
      if (editing === null) {
        const created = await createPrompt(draftFromForm());
        prompts = [...prompts, created];
        toast.success(i18n.t("prompts.saved"));
      } else {
        const updated = await updatePrompt({ id: editing.id, ...draftFromForm() });
        prompts = prompts.map((prompt) => (prompt.id === updated.id ? updated : prompt));
        toast.success(i18n.t("prompts.saved"));
      }
      closeForm();
    } catch (error) {
      if (isPromptError(error) && "invalidInput" in error) {
        errors = [error.invalidInput.field === "title" ? i18n.t("prompts.titleRequired") : i18n.t("prompts.bodyRequired")];
      } else {
        errors = [i18n.t("prompts.saveFailed")];
        formOpen = false;
        editing = null;
        retryableFailure = "save";
      }
    } finally {
      saving = false;
    }
  }

  function requestDelete(prompt: Prompt): void {
    pendingDelete = prompt;
    deleteDialogOpen = true;
  }

  function cancelDelete(): void {
    deleteDialogOpen = false;
    pendingDelete = null;
  }

  async function confirmDelete(): Promise<void> {
    if (pendingDelete === null) return;
    const target = pendingDelete;
    deleteDialogOpen = false;
    pendingDelete = null;
    retryableFailure = null;
    try {
      await deletePrompt(target.id);
      prompts = prompts.filter((candidate) => candidate.id !== target.id);
      toast.success(i18n.t("prompts.deleted"));
    } catch {
      retryableFailure = "delete";
    }
  }

  async function copyPrompt(prompt: Prompt): Promise<void> {
    try {
      await navigator.clipboard.writeText(prompt.body);
      toast.success(i18n.t("prompts.copySuccess"));
    } catch {
      toast.error(i18n.t("prompts.copyFailed"));
    }
  }
</script>

<section class="space-y-6" aria-labelledby="prompts-title">
  <div class="flex flex-col gap-4 rounded-3xl border border-white/10 bg-white/5 p-6 shadow-2xl shadow-black/20 md:flex-row md:items-end md:justify-between">
    <div class="space-y-2">
      <p class="text-sm font-semibold uppercase tracking-[0.3em] text-cyan-200">{i18n.t("prompts.badge")}</p>
      <h1 id="prompts-title" class="text-3xl font-semibold text-white">{i18n.t("prompts.title")}</h1>
      <p class="max-w-2xl text-sm leading-6 text-mist-300">{i18n.t("prompts.intro")}</p>
    </div>
    <button data-testid="new-prompt" class="shadow-action rounded-control bg-action px-5 py-3 text-sm font-bold text-canvas transition-[background,transform] hover:bg-action/85 active:scale-[.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-action" type="button" onclick={openCreateForm}>
      {i18n.t("prompts.createAction")}
    </button>
  </div>

  {#if status === "failed" || retryableFailure !== null}
    <div class="rounded-2xl border border-red-300/30 bg-red-500/10 p-5 text-red-100" role="alert">
      <p class="font-semibold">{failureTitle}</p>
      <p class="mt-1 text-sm">{failureBody}</p>
      <button class="mt-4 rounded-xl border border-red-200/40 px-4 py-2 text-sm font-semibold" type="button" onclick={() => void loadPrompts()}>{i18n.t("prompts.retry")}</button>
    </div>
  {/if}

  {#if status === "loading"}
    <p class="rounded-2xl border border-white/10 bg-slate-950/50 px-4 py-8 text-center text-mist-300" role="status">{i18n.t("prompts.loading")}</p>
  {:else if status !== "failed" || prompts.length > 0}
    <div class="rounded-3xl border border-white/10 bg-slate-950/40 p-5">
      <label class="block text-sm font-medium text-mist-200" for="prompt-search">{i18n.t("prompts.searchLabel")}</label>
      <input id="prompt-search" type="search" class="mt-2 w-full rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-mist-100 outline-none focus:ring-2 focus:ring-cyan-300" placeholder={i18n.t("prompts.searchPlaceholder")} bind:value={query} />
    </div>

    {#if formOpen}
      <form class="space-y-4 rounded-3xl border border-white/10 bg-white/5 p-5" onsubmit={(event) => { event.preventDefault(); void savePrompt(); }}>
        <h2 class="text-xl font-semibold text-white">{formTitle}</h2>
        {#if errors.length > 0}
          <div class="rounded-2xl border border-red-300/30 bg-red-500/10 px-4 py-3 text-sm text-red-100" role="alert">
            {#each errors as error (error)}
              <p>{error}</p>
            {/each}
          </div>
        {/if}
        <div>
          <label class="block text-sm font-medium text-mist-200" for="prompt-title">{i18n.t("prompts.titleLabel")}</label>
          <input id="prompt-title" class="mt-2 w-full rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-mist-100 outline-none focus:ring-2 focus:ring-cyan-300" bind:value={title} />
        </div>
        <div>
          <label class="block text-sm font-medium text-mist-200" for="prompt-body">{i18n.t("prompts.bodyLabel")}</label>
          <textarea id="prompt-body" class="mt-2 min-h-36 w-full rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-mist-100 outline-none focus:ring-2 focus:ring-cyan-300" bind:value={body}></textarea>
        </div>
        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label class="block text-sm font-medium text-mist-200" for="prompt-tags">{i18n.t("prompts.tagsLabel")}</label>
            <input id="prompt-tags" class="mt-2 w-full rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-mist-100 outline-none focus:ring-2 focus:ring-cyan-300" bind:value={tags} />
          </div>
          <div>
            <label class="block text-sm font-medium text-mist-200" for="prompt-context">{i18n.t("prompts.contextLabel")}</label>
            <input id="prompt-context" class="mt-2 w-full rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-mist-100 outline-none focus:ring-2 focus:ring-cyan-300" bind:value={bestForContext} />
          </div>
        </div>
        <div class="flex flex-wrap gap-3">
          <button class="shadow-action rounded-control bg-action px-5 py-3 text-sm font-bold text-canvas transition-[background,transform] hover:bg-action/85 active:scale-[.98] disabled:cursor-not-allowed disabled:opacity-50" disabled={saving} type="submit">{saving ? i18n.t("prompts.saving") : i18n.t("prompts.saveAction")}</button>
          <button class="rounded-control border border-stroke px-5 py-3 text-sm font-semibold text-content transition-colors hover:bg-surface-raised hover:border-interactive-hover/55" type="button" onclick={closeForm}>{i18n.t("prompts.cancelAction")}</button>
        </div>
      </form>
    {/if}

    {#if prompts.length === 0 && retryableFailure === null}
      <div class="rounded-3xl border border-dashed border-white/15 bg-white/5 p-8 text-center">
        <h2 class="text-xl font-semibold text-white">{i18n.t("prompts.emptyTitle")}</h2>
        <p class="mt-2 text-sm text-mist-300">{i18n.t("prompts.emptyBody")}</p>
        <button class="mt-5 shadow-action rounded-control bg-action px-5 py-3 text-sm font-bold text-canvas transition-[background,transform] hover:bg-action/85 active:scale-[.98]" type="button" onclick={openCreateForm}>{i18n.t("prompts.emptyAction")}</button>
      </div>
    {:else if visiblePrompts.length === 0}
      <p class="rounded-2xl border border-white/10 bg-slate-950/50 px-4 py-8 text-center text-mist-300">{i18n.t("prompts.noSearchResults")}</p>
    {:else}
      <div class="grid gap-4">
        {#each pagePrompts as prompt (prompt.id)}
          <article class="rounded-3xl border border-white/10 bg-white/5 p-5 shadow-xl shadow-black/10" data-testid="prompt-card">
            <div class="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
              <div class="min-w-0 space-y-3">
                <h2 class="text-xl font-semibold text-white">{prompt.title}</h2>
                <p class="whitespace-pre-wrap break-words text-sm leading-6 text-mist-200">{promptBodyPreview(prompt.body)}</p>
                {#if prompt.tags.length > 0}
                  <div class="flex flex-wrap gap-2" aria-label={i18n.t("prompts.tagsLabel")}>
                    {#each prompt.tags as tag (tag)}
                      <span class="rounded-full border border-cyan-300/20 bg-cyan-300/10 px-3 py-1 text-xs text-cyan-100">{tag}</span>
                    {/each}
                  </div>
                {/if}
                {#if prompt.bestForContext !== null}
                  <p class="text-xs text-mist-400">{i18n.t("prompts.contextPrefix", { context: prompt.bestForContext })}</p>
                {/if}
              </div>
              <div class="flex shrink-0 flex-wrap gap-2">
                <button data-testid="copy-prompt" class="min-h-11 rounded-xl border border-white/15 bg-white/[0.03] px-3 py-2 text-sm font-semibold text-mist-100 transition-colors hover:border-cyan-200/60 hover:bg-cyan-300/10 hover:text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-200" type="button" onclick={() => void copyPrompt(prompt)}>{i18n.t("prompts.copyAction")}</button>
                <button data-testid="edit-prompt" class="min-h-11 rounded-xl border border-white/15 bg-white/[0.03] px-3 py-2 text-sm font-semibold text-mist-100 transition-colors hover:border-cyan-200/60 hover:bg-cyan-300/10 hover:text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-200" type="button" onclick={() => openEditForm(prompt)}>{i18n.t("prompts.editAction")}</button>
                <button data-testid="delete-prompt" class="min-h-11 rounded-xl border border-red-300/40 bg-red-500/10 px-3 py-2 text-sm font-semibold text-red-100 transition-colors hover:border-red-200/70 hover:bg-red-500/20 hover:text-red-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-200" type="button" onclick={() => requestDelete(prompt)}>{i18n.t("prompts.deleteAction")}</button>
              </div>
            </div>
          </article>
        {/each}
      </div>
      {#if visiblePrompts.length > 0}
        <nav
          aria-label={i18n.t("prompts.paginationPage", { current: page, total: pageCount })}
          class="mt-4 flex flex-wrap items-center justify-between gap-4 rounded-3xl border border-white/10 bg-white/5 p-4"
        >
          <p class="text-sm text-mist-300" aria-live="polite">
            {i18n.t("prompts.paginationSummary", {
              from: rangeStart,
              to: rangeEnd,
              total: visiblePrompts.length,
            })}
          </p>

          <div class="flex flex-wrap items-center gap-2">
            <label class="flex items-center gap-2 text-sm text-mist-300">
              <span>{i18n.t("prompts.paginationPageSize")}</span>
              <select
                aria-label={i18n.t("prompts.paginationPageSize")}
                value={pageSize}
                onchange={onPageSizeChange}
                class="rounded-xl border border-white/15 bg-slate-950/70 px-2.5 py-2 text-sm text-mist-100 focus:border-cyan-200 focus:outline-none focus:ring-2 focus:ring-cyan-300/60"
              >
                {#each PAGE_SIZES as size (size)}
                  <option value={size}>{size}</option>
                {/each}
              </select>
            </label>

            <div class="flex items-center gap-1" aria-label={i18n.t("prompts.paginationPage", { current: page, total: pageCount })}>
              <button type="button" aria-label={i18n.t("prompts.paginationFirst")} disabled={page === 1} onclick={() => (page = 1)} class="rounded-xl border border-white/15 px-2.5 py-2 text-sm text-mist-100 transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-45 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-200">«</button>
              <button type="button" aria-label={i18n.t("prompts.paginationPrevious")} disabled={page === 1} onclick={() => (page -= 1)} class="rounded-xl border border-white/15 px-2.5 py-2 text-sm text-mist-100 transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-45 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-200">‹</button>
              <span class="min-w-20 px-2 text-center text-sm tabular-nums text-mist-300">
                {i18n.t("prompts.paginationPage", { current: page, total: pageCount })}
              </span>
              <button type="button" aria-label={i18n.t("prompts.paginationNext")} disabled={page === pageCount} onclick={() => (page += 1)} class="rounded-xl border border-white/15 px-2.5 py-2 text-sm text-mist-100 transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-45 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-200">›</button>
              <button type="button" aria-label={i18n.t("prompts.paginationLast")} disabled={page === pageCount} onclick={() => (page = pageCount)} class="rounded-xl border border-white/15 px-2.5 py-2 text-sm text-mist-100 transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-45 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-200">»</button>
            </div>
          </div>
        </nav>
      {/if}
    {/if}
  {/if}

  <ConfirmDialog
    open={deleteDialogOpen}
    title={i18n.t("prompts.deleteConfirmTitle")}
    body={i18n.t("prompts.deleteConfirmBody")}
    confirmLabel={i18n.t("prompts.deleteAction")}
    cancelLabel={i18n.t("prompts.cancelAction")}
    onConfirm={() => void confirmDelete()}
    onCancel={cancelDelete}
  />
</section>
