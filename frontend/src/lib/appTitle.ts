/** Builds the display title for the Vertice window/header from the product name and version. */
export function appTitle(productName: string, version: string): string {
  return `${productName} v${version}`;
}
