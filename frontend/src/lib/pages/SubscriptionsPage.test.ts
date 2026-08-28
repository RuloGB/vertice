// @vitest-environment jsdom
import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Subscription } from "../../bindings/Subscription";
import { getAllByRole, getByLabel, getByRole, queryByRole } from "../testing/a11y";
import {
  createSubscription,
  deleteSubscription,
  fetchSubscriptions,
  updateSubscription,
} from "../subscriptions";
import SubscriptionsPage from "./SubscriptionsPageHarness.svelte";

vi.mock("../subscriptions", async (original) => ({
  ...(await original<typeof import("../subscriptions")>()),
  fetchSubscriptions: vi.fn(),
  createSubscription: vi.fn(),
  updateSubscription: vi.fn(),
  deleteSubscription: vi.fn(),
}));

const mockedFetch = vi.mocked(fetchSubscriptions);
const mockedCreate = vi.mocked(createSubscription);
const mockedUpdate = vi.mocked(updateSubscription);
const mockedDelete = vi.mocked(deleteSubscription);

const existing: Subscription = {
  id: "sub-1",
  provider: "OpenAI",
  plan: "Plus",
  amount: 20,
  currency: "USD",
  cycle: "monthly",
  renewalDay: 12,
  renewalMonth: null,
  updatedAt: "2026-01-01T00:00:00.000000000Z",
};

async function flush(): Promise<void> {
  await tick();
  await Promise.resolve();
  await tick();
}

function clickByRole(name: string): void {
  getByRole<HTMLButtonElement>("button", { name }).click();
}

function inputByLabel(label: string, value: string): void {
  const element = getByLabel<HTMLInputElement>(label);
  element.value = value;
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

function selectByLabel(label: string, value: string): void {
  const element = getByLabel<HTMLSelectElement>(label);
  element.value = value;
  element.dispatchEvent(new Event("change", { bubbles: true }));
}

function submitForm(): void {
  getByRole<HTMLButtonElement>("button", { name: "Save subscription" })
    .form?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
}

function fillValidForm(): void {
  inputByLabel("Provider", "Anthropic");
  inputByLabel("Plan", "Pro");
  inputByLabel("Amount", "25");
  selectByLabel("Renewal day", "8");
}

beforeEach(() => {
  document.body.innerHTML = "";
  vi.resetAllMocks();
});

describe("SubscriptionsPage", () => {
  it("shows English manual recovery guidance for an unavailable subscription store", async () => {
    mockedFetch.mockRejectedValue({ storeCorrupt: { reason: "unsupported schema" } });
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();

    expect(getByRole("alert").textContent).toContain("Subscription storage needs manual recovery");
    expect(getByRole("alert").textContent).toContain("does not have permission to repair this file");
    expect(queryByRole("button", { name: "Retry" })).toBeNull();
    unmount(app);
  });

  it("shows Spanish manual recovery guidance for an unavailable subscription store", async () => {
    mockedFetch.mockRejectedValue({ storeCorrupt: { reason: "invalid data" } });
    const app = mount(SubscriptionsPage, { target: document.body, props: { locale: "es" } });
    await flush();

    expect(getByRole("alert").textContent).toContain("El almacenamiento de suscripciones necesita recuperación manual");
    expect(getByRole("alert").textContent).toContain("no tiene permiso para reparar este archivo");
    expect(queryByRole("button", { name: "Reintentar" })).toBeNull();
    unmount(app);
  });

  it("retains a Retry action for temporary store I/O failures", async () => {
    mockedFetch.mockRejectedValue({ storeUnavailable: { reason: "access denied" } });
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();

    expect(getByRole("alert").textContent).toContain("Could not load subscriptions");
    expect(getByRole("button", { name: "Retry" })).toBeInstanceOf(HTMLButtonElement);
    unmount(app);
  });

  it("requires reload rather than mutation retry after a durability warning", async () => {
    mockedFetch.mockResolvedValue([]);
    mockedCreate.mockRejectedValue({ committedWithDurabilityWarning: { reason: "directory sync failed" } });
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();
    clickByRole("New subscription");
    await flush();
    fillValidForm();
    submitForm();
    await flush();

    expect(getByRole("alert").textContent).toContain("Subscription change needs reconciliation");
    expect(getByRole("button", { name: "Reload subscriptions" })).toBeInstanceOf(HTMLButtonElement);
    expect(queryByRole("button", { name: "Retry" })).toBeNull();
    clickByRole("Reload subscriptions");
    await flush();
    expect(mockedFetch).toHaveBeenCalledTimes(2);
    expect(queryByRole("button", { name: "Save subscription" })).toBeNull();
    unmount(app);
  });

  it("closes a pending delete before reloading after a durability warning", async () => {
    mockedFetch.mockResolvedValue([existing]);
    mockedDelete.mockRejectedValue({ committedWithDurabilityWarning: { reason: "directory sync failed" } });
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();

    clickByRole("Delete OpenAI");
    await flush();
    clickByRole("Delete");
    await flush();
    expect(getByRole("dialog", { name: "Delete subscription" })).toBeInstanceOf(HTMLDivElement);

    clickByRole("Reload subscriptions");
    await flush();
    expect(queryByRole("dialog", { name: "Delete subscription" })).toBeNull();
    expect(mockedFetch).toHaveBeenCalledTimes(2);
    unmount(app);
  });

  it("retries a failed English load and clears the error after success", async () => {
    mockedFetch.mockRejectedValueOnce(new Error("disk")).mockResolvedValueOnce([]);
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();

    expect(getByRole("alert").textContent).toContain("Could not load subscriptions");
    clickByRole("Retry");
    await flush();

    expect(mockedFetch).toHaveBeenCalledTimes(2);
    expect(queryByRole("alert")).toBeNull();
    expect(getByRole("heading", { name: "No subscriptions yet" })).toBeInstanceOf(HTMLHeadingElement);
    unmount(app);
  });

  it("shows the localized Spanish empty state without sample records", async () => {
    mockedFetch.mockResolvedValue([]);
    const app = mount(SubscriptionsPage, { target: document.body, props: { locale: "es" } });
    await flush();

    expect(getByRole("heading", { name: "Aún no hay suscripciones" })).toBeInstanceOf(HTMLHeadingElement);
    expect(getAllByRole("button", { name: /Editar|Eliminar/ })).toHaveLength(0);
    unmount(app);
  });

  it("validates required form values before invoking the backend", async () => {
    mockedFetch.mockResolvedValue([]);
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();
    clickByRole("New subscription");
    await flush();
    submitForm();
    await flush();

    expect(getByRole("alert").textContent).toContain("Provider is required");
    expect(mockedCreate).not.toHaveBeenCalled();
    unmount(app);
  });

  it("shows a backend InvalidInput error in the form", async () => {
    mockedFetch.mockResolvedValue([]);
    mockedCreate.mockRejectedValue({ invalidInput: { field: "amount" } });
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();
    clickByRole("New subscription");
    await flush();
    fillValidForm();
    submitForm();
    await flush();

    expect(getByRole("alert").textContent).toContain("Amount must be greater than zero");
    expect(mockedCreate).toHaveBeenCalledWith(expect.objectContaining({ amount: 25 }));
    unmount(app);
  });

  it("creates and edits a subscription through generated IPC contracts", async () => {
    const updated = { ...existing, provider: "Anthropic", plan: "Pro", amount: 25 };
    mockedFetch.mockResolvedValueOnce([]).mockResolvedValueOnce([updated]);
    mockedCreate.mockResolvedValue(updated);
    mockedUpdate.mockResolvedValue({ ...updated, plan: "Team" });
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();
    clickByRole("New subscription");
    await flush();
    fillValidForm();
    submitForm();
    await flush();

    expect(getByRole("heading", { name: "Anthropic" })).toBeInstanceOf(HTMLHeadingElement);
    clickByRole("Edit Anthropic");
    await flush();
    inputByLabel("Plan", "Team");
    submitForm();
    await flush();

    expect(mockedUpdate).toHaveBeenCalledWith(expect.objectContaining({ id: "sub-1", plan: "Team" }));
    expect(getByRole("heading", { name: "Anthropic" }).parentElement?.textContent).toContain("Team");
    unmount(app);
  });

  it("retries a failed save and removes its retry alert after success", async () => {
    mockedFetch.mockResolvedValue([]);
    mockedCreate.mockRejectedValueOnce(new Error("offline")).mockResolvedValueOnce(existing);
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();
    clickByRole("New subscription");
    await flush();
    fillValidForm();
    submitForm();
    await flush();
    expect(getByRole("alert").textContent).toContain("Could not save subscription");

    clickByRole("Retry");
    await flush();
    expect(mockedCreate).toHaveBeenCalledTimes(2);
    expect(queryByRole("alert")).toBeNull();
    expect(getByRole("heading", { name: "OpenAI" })).toBeInstanceOf(HTMLHeadingElement);
    unmount(app);
  });

  it("cancels then retries delete, clearing the retry alert after success", async () => {
    mockedFetch.mockResolvedValue([existing]);
    mockedDelete.mockRejectedValueOnce(new Error("offline")).mockResolvedValueOnce(undefined);
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();

    clickByRole("Delete OpenAI");
    await flush();
    expect(getByRole("dialog", { name: "Delete subscription" })).toBeInstanceOf(HTMLDivElement);
    clickByRole("Cancel");
    await flush();
    expect(queryByRole("dialog", { name: "Delete subscription" })).toBeNull();

    clickByRole("Delete OpenAI");
    await flush();
    getByRole<HTMLButtonElement>("button", { name: "Delete" }).click();
    await flush();
    expect(getByRole("alert").textContent).toContain("Could not delete subscription");
    clickByRole("Retry");
    await flush();

    expect(mockedDelete).toHaveBeenCalledTimes(2);
    expect(queryByRole("alert")).toBeNull();
    expect(queryByRole("heading", { name: "OpenAI" })).toBeNull();
    unmount(app);
  });

  it("reveals a labelled renewal-month select only for yearly billing", async () => {
    mockedFetch.mockResolvedValue([]);
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();
    clickByRole("New subscription");
    await flush();

    expect(getAllByRole("combobox", { name: "Renewal month" })).toHaveLength(0);
    selectByLabel("Billing cycle", "yearly");
    await flush();
    selectByLabel("Renewal month", "6");
    expect(getByLabel<HTMLSelectElement>("Renewal month").value).toBe("6");
    unmount(app);
  });

  it("disables Save while a mutation is in flight", async () => {
    mockedFetch.mockResolvedValue([]);
    let resolveCreate: (subscription: Subscription) => void = () => {};
    mockedCreate.mockImplementationOnce(() => new Promise((resolve) => { resolveCreate = resolve; }));
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();
    clickByRole("New subscription");
    await flush();
    fillValidForm();
    submitForm();
    await flush();

    const saveButton = getByRole<HTMLButtonElement>("button", { name: "Save subscription" });
    expect(saveButton.disabled).toBe(true);
    submitForm();
    expect(mockedCreate).toHaveBeenCalledTimes(1);

    resolveCreate(existing);
    await flush();
    expect(queryByRole("button", { name: "Save subscription" })).toBeNull();
    expect(getByRole("heading", { name: "OpenAI" })).toBeInstanceOf(HTMLHeadingElement);
    unmount(app);
  });

  it("shows localized billing cycle and renewal countdown on cards", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-06-10T12:00:00Z"));
    mockedFetch.mockResolvedValue([{ ...existing, cycle: "yearly", renewalDay: 10, renewalMonth: 6 }]);
    const app = mount(SubscriptionsPage, { target: document.body });
    await flush();

    const cardHeading = getByRole("heading", { name: "OpenAI" });
    expect(cardHeading.parentElement?.textContent).toContain("Billing cycle: Yearly");
    expect(cardHeading.parentElement?.textContent).toContain("Renews today");
    unmount(app);
    vi.useRealTimers();
  });

  it("shows Spanish deletion errors and contextual action names", async () => {
    mockedFetch.mockResolvedValue([existing]);
    mockedDelete.mockRejectedValue(new Error("offline"));
    const app = mount(SubscriptionsPage, { target: document.body, props: { locale: "es" } });
    await flush();

    clickByRole("Eliminar OpenAI");
    await flush();
    getByRole<HTMLButtonElement>("button", { name: "Eliminar" }).click();
    await flush();

    expect(getByRole("alert").textContent).toContain("No se pudo eliminar la suscripción");
    unmount(app);
  });
});
