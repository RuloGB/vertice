<script lang="ts">
  import type { BillingCycle } from "../../bindings/BillingCycle";
  import type { Currency } from "../../bindings/Currency";
  import type { Subscription } from "../../bindings/Subscription";
  import type { SubscriptionDraft } from "../../bindings/SubscriptionDraft";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import SubscriptionCard from "../SubscriptionCard.svelte";
  import { useI18n } from "../i18n/locale.svelte";
  import {
    createSubscription,
    deleteSubscription,
    fetchSubscriptions,
    isSubscriptionError,
    MAX_RENEWAL_DAY,
    MONTHS_PER_YEAR,
    monthlyTotalsByCurrency,
    sortByRenewal,
    updateSubscription,
  } from "../subscriptions";

  type UiOutcome =
    | { kind: "none" }
    | { kind: "retry"; operation: "load" | "save" | "delete" }
    | { kind: "manual-recovery" }
    | { kind: "reconciliation" };
  type PageState =
    | { kind: "loading" }
    | { kind: "ready"; outcome: UiOutcome }
    | { kind: "failed"; outcome: Exclude<UiOutcome, { kind: "none" }> };

  const RENEWAL_DAYS = Array.from({ length: MAX_RENEWAL_DAY }, (_, index) => index + 1);
  const RENEWAL_MONTHS = Array.from({ length: MONTHS_PER_YEAR }, (_, index) => index + 1);
  const i18n = useI18n();
  const today = new Date();

  let page = $state<PageState>({ kind: "loading" });
  let subscriptions = $state<Subscription[]>([]);
  let formOpen = $state(false);
  let editing = $state<Subscription | null>(null);
  let provider = $state("");
  let plan = $state("");
  let amount = $state("");
  let currency = $state<Currency>("EUR");
  let cycle = $state<BillingCycle>("monthly");
  let renewalDay = $state(1);
  let renewalMonth = $state("");
  let errors = $state<string[]>([]);
  let saving = $state(false);
  let pendingDelete = $state<Subscription | null>(null);

  const orderedSubscriptions = $derived(sortByRenewal(subscriptions, today));
  const monthlyTotal = $derived(
    [...monthlyTotalsByCurrency(subscriptions)]
      .map(([itemCurrency, value]) =>
        new Intl.NumberFormat(i18n.locale === "es" ? "es-ES" : "en-US", {
          style: "currency",
          currency: itemCurrency,
        }).format(value),
      )
      .join(" + "),
  );
  const formTitle = $derived(
    editing === null ? i18n.t("subscriptions.createTitle") : i18n.t("subscriptions.editTitle"),
  );
  const outcome = $derived(page.kind === "loading" ? { kind: "none" } as UiOutcome : page.outcome);
  const failureMessage = $derived(
    i18n.t(
      `subscriptions.${outcome.kind === "retry" && outcome.operation === "delete" ? "deleteFailed" : outcome.kind === "retry" && outcome.operation === "save" ? "saveFailed" : "failureTitle"}`,
    ),
  );

  void loadSubscriptions();

  async function loadSubscriptions(): Promise<void> {
    page = { kind: "loading" };
    try {
      subscriptions = await fetchSubscriptions();
      page = { kind: "ready", outcome: { kind: "none" } };
    } catch (error) {
      page = { kind: "failed", outcome: outcomeForFailure(error, "load") };
    }
  }

  function openForm(subscription: Subscription | null): void {
    editing = subscription;
    provider = subscription?.provider ?? "";
    plan = subscription?.plan ?? "";
    amount = subscription?.amount.toString() ?? "";
    currency = subscription?.currency ?? "EUR";
    cycle = subscription?.cycle ?? "monthly";
    renewalDay = subscription?.renewalDay ?? 1;
    renewalMonth = subscription?.renewalMonth?.toString() ?? "";
    errors = [];
    if (page.kind === "ready") page = { ...page, outcome: { kind: "none" } };
    formOpen = true;
  }

  function closeForm(): void {
    formOpen = false;
    editing = null;
    errors = [];
  }

  function resetMutationContext(): void {
    closeForm();
    pendingDelete = null;
  }

  function draftFromForm(): SubscriptionDraft {
    return {
      provider: provider.trim(),
      plan: plan.trim(),
      amount: Number(amount),
      currency,
      cycle,
      renewalDay,
      renewalMonth: cycle === "yearly" && renewalMonth !== "" ? Number(renewalMonth) : null,
    };
  }

  function validateForm(): boolean {
    const nextErrors: string[] = [];
    if (provider.trim() === "") nextErrors.push(i18n.t("subscriptions.providerRequired"));
    if (plan.trim() === "") nextErrors.push(i18n.t("subscriptions.planRequired"));
    if (!(Number(amount) > 0)) nextErrors.push(i18n.t("subscriptions.amountInvalid"));
    if (!(renewalDay >= 1 && renewalDay <= MAX_RENEWAL_DAY)) nextErrors.push(i18n.t("subscriptions.dayInvalid"));
    if (cycle === "yearly" && !(Number(renewalMonth) >= 1 && Number(renewalMonth) <= MONTHS_PER_YEAR)) {
      nextErrors.push(i18n.t("subscriptions.monthInvalid"));
    }
    errors = nextErrors;
    return nextErrors.length === 0;
  }

  function invalidInputMessage(error: unknown): string {
    if (!isSubscriptionError(error) || !("invalidInput" in error)) {
      return i18n.t("subscriptions.saveFailed");
    }
    const messages: Record<string, string> = {
      provider: i18n.t("subscriptions.providerRequired"),
      plan: i18n.t("subscriptions.planRequired"),
      amount: i18n.t("subscriptions.amountInvalid"),
      renewalDay: i18n.t("subscriptions.dayInvalid"),
      renewalMonth: i18n.t("subscriptions.monthInvalid"),
    };
    return messages[error.invalidInput.field] ?? i18n.t("subscriptions.invalidInput");
  }

  function isStoreCorrupt(error: unknown): boolean {
    return isSubscriptionError(error) && "storeCorrupt" in error;
  }

  function hasDurabilityWarning(error: unknown): boolean {
    return isSubscriptionError(error) && "committedWithDurabilityWarning" in error;
  }

  function outcomeForFailure(
    error: unknown,
    operation: "load" | "save" | "delete",
  ): Exclude<UiOutcome, { kind: "none" }> {
    if (isStoreCorrupt(error)) return { kind: "manual-recovery" };
    if (hasDurabilityWarning(error)) return { kind: "reconciliation" };
    return { kind: "retry", operation };
  }

  async function saveSubscription(): Promise<void> {
    if (saving) return;
    if (page.kind !== "ready") return;
    page = { ...page, outcome: { kind: "none" } };
    if (!validateForm()) return;
    saving = true;
    try {
      const saved = editing === null
        ? await createSubscription(draftFromForm())
        : await updateSubscription({ id: editing.id, ...draftFromForm() });
      subscriptions = editing === null
        ? [...subscriptions, saved]
        : subscriptions.map((item) => (item.id === saved.id ? saved : item));
      closeForm();
      page = { kind: "ready", outcome: { kind: "none" } };
    } catch (error) {
      const invalidInput = isSubscriptionError(error) && "invalidInput" in error;
      errors = invalidInput ? [invalidInputMessage(error)] : [];
      page = { kind: "ready", outcome: invalidInput ? { kind: "none" } : outcomeForFailure(error, "save") };
    } finally {
      saving = false;
    }
  }

  function requestDelete(subscription: Subscription): void {
    pendingDelete = subscription;
  }

  function cancelDelete(): void {
    pendingDelete = null;
  }

  async function confirmDelete(): Promise<void> {
    if (pendingDelete === null) return;
    const target = pendingDelete;
    pendingDelete = null;
    if (page.kind !== "ready") return;
    page = { ...page, outcome: { kind: "none" } };
    try {
      await deleteSubscription(target.id);
      subscriptions = subscriptions.filter((item) => item.id !== target.id);
      page = { kind: "ready", outcome: { kind: "none" } };
    } catch (error) {
      pendingDelete = target;
      page = { kind: "ready", outcome: outcomeForFailure(error, "delete") };
    }
  }

  async function retryLastOperation(): Promise<void> {
    if (outcome.kind !== "retry") return;
    if (outcome.operation === "load") await loadSubscriptions();
    if (outcome.operation === "save") await saveSubscription();
    if (outcome.operation === "delete") await confirmDelete();
  }

  async function reloadSubscriptions(): Promise<void> {
    resetMutationContext();
    await loadSubscriptions();
  }
</script>

<section class="space-y-6" aria-labelledby="subscriptions-title">
  <header class="flex flex-col gap-4 rounded-3xl border border-white/10 bg-white/5 p-6 shadow-2xl shadow-black/20 md:flex-row md:items-end md:justify-between">
    <div class="space-y-2">
      <p class="text-sm font-semibold uppercase tracking-[0.3em] text-cyan-200">{i18n.t("subscriptions.badge")}</p>
      <h1 id="subscriptions-title" class="text-3xl font-semibold text-white">{i18n.t("area.subscriptions")}</h1>
      <p class="max-w-2xl text-sm leading-6 text-mist-300">{i18n.t("subscriptions.intro")}</p>
    </div>
    <button
      type="button"
      class="shadow-action rounded-control bg-action px-4 py-2.5 text-sm font-bold text-canvas transition-[background,transform] hover:bg-action/85 active:scale-[.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-action disabled:cursor-not-allowed disabled:opacity-50"
      disabled={page.kind !== "ready"}
      onclick={() => openForm(null)}
    >
      {i18n.t("subscriptions.createAction")}
    </button>
  </header>

  {#if page.kind === "loading"}
    <div role="status" class="surface-card flex min-h-44 items-center justify-center p-12 text-center text-sm text-content-subtle">
      {i18n.t("subscriptions.loading")}
    </div>
  {:else}
    {#if outcome.kind === "manual-recovery"}
      <div role="alert" class="rounded-panel border border-warning/45 bg-warning/10 p-6 text-sm text-warning">
        <h2 class="text-base font-semibold text-content">{i18n.t("subscriptions.recoveryTitle")}</h2>
        <p class="mt-2 text-content-muted">{i18n.t("subscriptions.recoveryBody")}</p>
        <ol class="mt-4 list-decimal space-y-2 pl-5 text-content-muted">
          <li>{i18n.t("subscriptions.recoveryBackup")}</li>
          <li>{i18n.t("subscriptions.recoveryReplace")}</li>
          <li>{i18n.t("subscriptions.recoveryReopen")}</li>
        </ol>
      </div>
    {:else if outcome.kind === "reconciliation"}
      <div role="alert" class="rounded-panel flex flex-wrap items-end justify-between gap-4 border border-warning/45 bg-warning/10 p-6 text-sm text-warning">
        <div>
          <h2 class="text-base font-semibold text-content">{i18n.t("subscriptions.durabilityWarningTitle")}</h2>
          <p class="mt-2 text-content-muted">{i18n.t("subscriptions.durabilityWarningBody")}</p>
        </div>
        <button type="button" class="rounded-control border border-warning/45 px-4 py-2 text-sm font-bold text-warning transition-colors hover:bg-warning/15 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-warning" onclick={() => void reloadSubscriptions()}>{i18n.t("subscriptions.reloadAction")}</button>
      </div>
    {:else if page.kind === "failed" || outcome.kind === "retry"}
      <div role="alert" class="rounded-panel flex flex-wrap items-center justify-between gap-4 border border-danger/45 bg-danger/10 p-6 text-sm text-danger">
        <p class="font-semibold">{failureMessage}</p>
        <button type="button" class="rounded-control border border-danger/45 px-4 py-2 text-sm font-bold text-danger transition-colors hover:bg-danger/15 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-danger" onclick={() => void retryLastOperation()}>{i18n.t("subscriptions.retry")}</button>
      </div>
    {/if}

    {#if page.kind === "ready"}
      <div class="grid gap-4 md:grid-cols-2">
        <section class="surface-card relative overflow-hidden px-5 py-4" aria-label={i18n.t("area.subscriptions")}>
          <span class="label-caps">{i18n.t("area.subscriptions")}</span>
          <span class="mt-2 block text-3xl font-semibold tabular-nums text-content">{orderedSubscriptions.length}</span>
          <span class="absolute bottom-0 left-0 h-1 w-16 bg-action"></span>
        </section>
        <section class="surface-card relative overflow-hidden px-5 py-4" aria-label={i18n.t("subscriptions.summaryMonthly")}>
          <span class="label-caps">{i18n.t("subscriptions.summaryMonthly")}</span>
          <span class="mt-2 block text-3xl font-semibold tabular-nums text-content">{monthlyTotal || "—"}</span>
          <span class="absolute bottom-0 left-0 h-1 w-16 bg-interactive-hover"></span>
        </section>
      </div>

      {#if subscriptions.length === 0 && outcome.kind === "none"}
        <section class="rounded-panel border border-dashed border-stroke-strong bg-surface/60 px-6 py-14 text-center" aria-labelledby="subscriptions-empty-title">
          <div class="mx-auto flex h-14 w-14 items-center justify-center rounded-full border border-interactive/35 bg-interactive/10 text-interactive-hover" aria-hidden="true">
            <svg viewBox="0 0 24 24" class="h-7 w-7" fill="none" stroke="currentColor" stroke-width="1.7">
              <path d="M4 7.5A2.5 2.5 0 0 1 6.5 5h11A2.5 2.5 0 0 1 20 7.5v9a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 4 16.5z" />
              <path d="M7 9h10M7 13h4" stroke-linecap="round" />
            </svg>
          </div>
          <h2 id="subscriptions-empty-title" class="mt-5 text-lg font-semibold text-content">{i18n.t("subscriptions.emptyTitle")}</h2>
          <p class="mx-auto mt-2 max-w-md text-sm leading-6 text-content-muted">{i18n.t("subscriptions.emptyBody")}</p>
        </section>
      {:else if subscriptions.length > 0}
        <ul class="grid gap-4 lg:grid-cols-2 2xl:grid-cols-3" aria-label={i18n.t("area.subscriptions")}>
          {#each orderedSubscriptions as subscription (subscription.id)}
            <li><SubscriptionCard {subscription} {today} onEdit={openForm} onDelete={requestDelete} /></li>
          {/each}
        </ul>
      {/if}
    {/if}
  {/if}

  {#if formOpen}
    <div class="fixed inset-0 z-40 flex items-center justify-center p-4" role="presentation">
      <button class="absolute inset-0 cursor-default bg-canvas/75 backdrop-blur-sm" type="button" aria-label={i18n.t("subscriptions.cancelAction")} onclick={closeForm}></button>
      <form class="relative z-10 w-full max-w-2xl rounded-panel border border-stroke bg-surface p-6 shadow-panel" aria-labelledby="subscription-form-title" onsubmit={(event) => { event.preventDefault(); void saveSubscription(); }}>
        <div class="flex items-start justify-between gap-4 border-b border-stroke pb-4">
          <div>
            <h2 id="subscription-form-title" class="text-xl font-semibold text-content">{formTitle}</h2>
          </div>
        </div>
        {#if errors.length > 0}
          <div role="alert" class="mt-5 rounded-control border border-danger/45 bg-danger/10 px-4 py-3 text-sm text-danger">
            {#each errors as error (error)}<p>{error}</p>{/each}
          </div>
        {/if}
        <div class="mt-6 grid gap-5 sm:grid-cols-2">
          <div class="flex flex-col gap-2 text-sm font-semibold text-content"><label for="subscription-provider">{i18n.t("subscriptions.providerLabel")}</label><input class="rounded-control border border-stroke bg-canvas/35 px-3 py-2.5 font-normal text-content outline-none transition-colors placeholder:text-content-subtle focus:border-action focus:ring-2 focus:ring-action/25" id="subscription-provider" bind:value={provider} /></div>
          <div class="flex flex-col gap-2 text-sm font-semibold text-content"><label for="subscription-plan">{i18n.t("subscriptions.planLabel")}</label><input class="rounded-control border border-stroke bg-canvas/35 px-3 py-2.5 font-normal text-content outline-none transition-colors placeholder:text-content-subtle focus:border-action focus:ring-2 focus:ring-action/25" id="subscription-plan" bind:value={plan} /></div>
          <div class="flex flex-col gap-2 text-sm font-semibold text-content"><label for="subscription-amount">{i18n.t("subscriptions.amountLabel")}</label><input class="rounded-control border border-stroke bg-canvas/35 px-3 py-2.5 font-normal text-content outline-none transition-colors placeholder:text-content-subtle focus:border-action focus:ring-2 focus:ring-action/25" id="subscription-amount" type="number" min="0" step="0.01" bind:value={amount} /></div>
          <div class="flex flex-col gap-2 text-sm font-semibold text-content"><label for="subscription-currency">{i18n.t("subscriptions.currencyLabel")}</label><select class="rounded-control border border-stroke bg-canvas/35 px-3 py-2.5 font-normal text-content outline-none transition-colors focus:border-action focus:ring-2 focus:ring-action/25" id="subscription-currency" bind:value={currency}><option value="EUR">EUR</option><option value="USD">USD</option></select></div>
          <div class="flex flex-col gap-2 text-sm font-semibold text-content"><label for="subscription-cycle">{i18n.t("subscriptions.cycleLabel")}</label><select class="rounded-control border border-stroke bg-canvas/35 px-3 py-2.5 font-normal text-content outline-none transition-colors focus:border-action focus:ring-2 focus:ring-action/25" id="subscription-cycle" bind:value={cycle}><option value="monthly">{i18n.t("subscriptions.cycleMonthly")}</option><option value="yearly">{i18n.t("subscriptions.cycleYearly")}</option></select></div>
          <div class="flex flex-col gap-2 text-sm font-semibold text-content"><label for="subscription-renewal-day">{i18n.t("subscriptions.renewalDayLabel")}</label><select class="rounded-control border border-stroke bg-canvas/35 px-3 py-2.5 font-normal text-content outline-none transition-colors focus:border-action focus:ring-2 focus:ring-action/25" id="subscription-renewal-day" bind:value={renewalDay}>{#each RENEWAL_DAYS as day (day)}<option value={day}>{day}</option>{/each}</select></div>
          {#if cycle === "yearly"}
            <div class="flex flex-col gap-2 text-sm font-semibold text-content"><label for="subscription-renewal-month">{i18n.t("subscriptions.renewalMonthLabel")}</label><select class="rounded-control border border-stroke bg-canvas/35 px-3 py-2.5 font-normal text-content outline-none transition-colors focus:border-action focus:ring-2 focus:ring-action/25" id="subscription-renewal-month" bind:value={renewalMonth}><option value="">{i18n.t("subscriptions.renewalMonthLabel")}</option>{#each RENEWAL_MONTHS as month (month)}<option value={month}>{month}</option>{/each}</select></div>
          {/if}
        </div>
        <div class="mt-7 flex flex-wrap justify-end gap-3 border-t border-stroke pt-5">
          <button type="button" class="rounded-control border border-stroke px-4 py-2.5 text-sm font-bold text-content-muted transition-colors hover:bg-canvas/45 hover:text-content focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-action" onclick={closeForm}>{i18n.t("subscriptions.cancelAction")}</button>
          <button type="submit" class="shadow-action rounded-control bg-action px-4 py-2.5 text-sm font-bold text-canvas transition-[background,transform] hover:bg-action/85 active:scale-[.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-action disabled:cursor-not-allowed disabled:opacity-50" disabled={saving}>{i18n.t("subscriptions.saveAction")}</button>
        </div>
      </form>
    </div>
  {/if}

  <ConfirmDialog
    open={pendingDelete !== null}
    title={i18n.t("subscriptions.deleteConfirmTitle")}
    body={i18n.t("subscriptions.deleteConfirmBody")}
    confirmLabel={i18n.t("subscriptions.deleteAction")}
    cancelLabel={i18n.t("subscriptions.cancelAction")}
    onConfirm={() => void confirmDelete()}
    onCancel={cancelDelete}
  />
</section>
