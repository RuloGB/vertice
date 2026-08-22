/** Product name shown in the window title and the sidebar brand mark. */
export const PRODUCT_NAME = "Vertice";

/** Displayed product version. Kept next to the title builder that consumes it. */
export const APP_VERSION = "0.1.0";

/** Builds the display title for the Vertice window/header from the product name, version, and optional area label. */
export function appTitle(productName: string, version: string, area?: string): string {
  const baseTitle = `${productName} v${version}`;
  return area === undefined ? baseTitle : `${baseTitle} — ${area}`;
}
