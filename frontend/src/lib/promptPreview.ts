/** Maximum number of code points shown for a prompt body in the list view. */
export const PROMPT_PREVIEW_LIMIT = 280;

/**
 * Bounds a prompt body for list rendering.
 *
 * The full body is always reachable through the edit form and the copy action;
 * this only limits what a prompt card shows so a long body cannot flood the list.
 * Counting is done by code point so surrogate pairs (emoji) are never split.
 */
export function promptBodyPreview(body: string, limit: number = PROMPT_PREVIEW_LIMIT): string {
  const codePoints = Array.from(body);
  if (codePoints.length <= limit) return body;
  return codePoints.slice(0, limit).join("").trimEnd() + "…";
}
