/** Builds the display title for the Vertice window/header from the product name, version, and optional area label. */
export function appTitle(productName: string, version: string, area?: string): string {
  const baseTitle = `${productName} v${version}`;
  return area === undefined ? baseTitle : `${baseTitle} — ${area}`;
}
