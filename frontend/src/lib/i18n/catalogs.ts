import { appTitle } from "../appTitle";

export type Catalog = {
  app: {
    title: string;
    languageLabel: string;
    languageEnglish: string;
    languageSpanish: string;
  };
  toolbar: {
    searchPlaceholder: string;
    searchAriaLabel: string;
    kindAriaLabel: string;
    allKinds: string;
    reload: string;
    reloading: string;
  };
  kind: {
    skill: string;
    agent: string;
  };
  inventory: {
    loading: string;
    empty: string;
    duplicate: string;
    duplicateTitle: string;
  };
  failure: {
    title: string;
    noRootsConfigured: string;
    internalReason: string;
    unexpected: string;
  };
  location: {
    noPath: string;
  };
};

export type SupportedLocale = "en" | "es";

export const catalogs = {
  en: {
    app: {
      title: appTitle("Vertice", "0.1.0", "Inventory"),
      languageLabel: "Language",
      languageEnglish: "English",
      languageSpanish: "Spanish",
    },
    toolbar: {
      searchPlaceholder: "Search by name",
      searchAriaLabel: "Search components by name",
      kindAriaLabel: "Filter by kind",
      allKinds: "All kinds",
      reload: "Reload",
      reloading: "Reloading...",
    },
    kind: {
      skill: "Skill",
      agent: "Agent",
    },
    inventory: {
      loading: "Scanning for installed components...",
      empty: "No components to show.",
      duplicate: "Duplicate",
      duplicateTitle: "Found at {count} locations",
    },
    failure: {
      title: "Inventory scan failed.",
      noRootsConfigured: "No search roots are configured.",
      internalReason: "Internal scan failure: {reason}",
      unexpected: "The scan failed unexpectedly.",
    },
    location: {
      noPath: "(no path on disk)",
    },
  },
  es: {
    app: {
      title: appTitle("Vertice", "0.1.0", "Inventario"),
      languageLabel: "Idioma",
      languageEnglish: "Inglés",
      languageSpanish: "Español",
    },
    toolbar: {
      searchPlaceholder: "Buscar por nombre",
      searchAriaLabel: "Buscar componentes por nombre",
      kindAriaLabel: "Filtrar por tipo",
      allKinds: "Todos los tipos",
      reload: "Recargar",
      reloading: "Recargando...",
    },
    kind: {
      skill: "Skill",
      agent: "Agent",
    },
    inventory: {
      loading: "Escaneando componentes instalados...",
      empty: "No hay componentes para mostrar.",
      duplicate: "Duplicado",
      duplicateTitle: "Encontrado en {count} ubicaciones",
    },
    failure: {
      title: "Falló el escaneo del inventario.",
      noRootsConfigured: "No hay raíces de búsqueda configuradas.",
      internalReason: "Fallo interno del escaneo: {reason}",
      unexpected: "El escaneo falló inesperadamente.",
    },
    location: {
      noPath: "(sin ruta en disco)",
    },
  },
} as const satisfies Record<SupportedLocale, Catalog>;

export type CatalogKey = LeafKeys<Catalog>;

type LeafKeys<T, Prefix extends string = ""> = {
  [K in keyof T & string]: T[K] extends string
    ? `${Prefix}${K}`
    : T[K] extends Record<string, unknown>
      ? LeafKeys<T[K], `${Prefix}${K}.`>
      : never;
}[keyof T & string];