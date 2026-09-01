import type { ClientInstallSlot } from "../../bindings/ClientInstallSlot";
import type { ClientPresence } from "../../bindings/ClientPresence";

/**
 * Select the record a product's card should render out of a group of
 * `ClientPresence` records, all belonging to the same product's slots.
 *
 * Records arrive in probe-table order and EVERY slot always emits a
 * record, so a plain `Array.prototype.find` over the group returns the
 * first slot's record regardless of its status — the defect this
 * replaces (H1). Prefer the first `detected` record of the group, in
 * record order; fall back to the group's first record so a fully
 * undetected product still renders a card with real probed paths.
 *
 * The rule is stated over N slots, not just two, and is proven over N in
 * `presenceFor.test.ts` (`client-installation-detector` / `inventory-ui`
 * delta, design §6.1).
 */
export const presenceFor = (
  records: readonly ClientPresence[],
  slots: readonly ClientInstallSlot[],
): ClientPresence | undefined => {
  const group = records.filter((record) => slots.includes(record.slot));
  return group.find((record) => record.status === "detected") ?? group[0];
};
