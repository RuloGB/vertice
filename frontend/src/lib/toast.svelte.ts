import { SvelteMap } from "svelte/reactivity";

export type ToastKind = "success" | "error" | "warning";

export interface Toast {
  readonly id: number;
  readonly kind: ToastKind;
  readonly message: string;
}

const AUTO_DISMISS_MS = 4000;

let nextId = 0;
let toasts = $state<Toast[]>([]);
const timers = new SvelteMap<number, ReturnType<typeof setTimeout>>();

function scheduleDismiss(id: number): void {
  const timer = setTimeout(() => dismiss(id), AUTO_DISMISS_MS);
  timers.set(id, timer);
}

function addToast(kind: ToastKind, message: string): void {
  const id = nextId++;
  toasts = [...toasts, { id, kind, message }];
  scheduleDismiss(id);
}

export function success(message: string): void {
  addToast("success", message);
}

export function error(message: string): void {
  addToast("error", message);
}

export function warning(message: string): void {
  addToast("warning", message);
}

export function dismiss(id: number): void {
  const timer = timers.get(id);
  if (timer !== undefined) {
    clearTimeout(timer);
    timers.delete(id);
  }
  toasts = toasts.filter((toast) => toast.id !== id);
}

export function getToasts(): Toast[] {
  return toasts;
}

export function clearAll(): void {
  for (const timer of timers.values()) {
    clearTimeout(timer);
  }
  timers.clear();
  toasts = [];
}
