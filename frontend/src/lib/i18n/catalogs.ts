import type { RouteId } from "../navigation";

export type Catalog = {
  app: {
    tagline: string;
    languageLabel: string;
    languageEnglish: string;
    languageSpanish: string;
  };
  nav: Record<RouteId, string>;
  navGroup: {
    overview: string;
    library: string;
    data: string;
  };
  area: Record<RouteId, string>;
  home: {
    greeting: string;
    subtitle: string;
    statComponents: string;
    statSkills: string;
    statAgents: string;
    statRoots: string;
    statsPending: string;
    ctaTitle: string;
    ctaBody: string;
    ctaAction: string;
  };
  placeholder: {
    badge: string;
    title: string;
    body: string;
  };
  subscriptions: {
    sampleBadge: string;
    intro: string;
    summaryActive: string;
    summaryMonthly: string;
    empty: string;
    planLabel: string;
    amountLabel: string;
    renewalLabel: string;
    cycleMonthly: string;
    cycleYearly: string;
    perMonth: string;
    perYear: string;
    renewsToday: string;
    renewsTomorrow: string;
    renewsInDays: string;
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
    embedded: string;
  };
  diagnostics: {
    title: string;
    unavailableRoots: string;
    missingClient: string;
    recoverableIssues: string;
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
      tagline: "AI component inventory",
      languageLabel: "Language",
      languageEnglish: "English",
      languageSpanish: "Spanish",
    },
    nav: {
      home: "Home",
      agents: "Agents",
      skills: "Skills",
      mcp: "MCP",
      prompts: "Prompts",
      inventory: "Inventory",
      subscriptions: "AI Subscriptions",
    },
    navGroup: {
      overview: "Overview",
      library: "Library",
      data: "Data",
    },
    area: {
      home: "Home",
      agents: "Agents",
      skills: "Skills",
      mcp: "MCP",
      prompts: "Prompts",
      inventory: "Inventory",
      subscriptions: "AI Subscriptions",
    },
    home: {
      greeting: "Welcome to Vertice",
      subtitle:
        "Vertice reads your machine and shows every AI component installed across your clients.",
      statComponents: "Components",
      statSkills: "Skills",
      statAgents: "Agents",
      statRoots: "Scan roots",
      statsPending: "—",
      ctaTitle: "Start with the inventory",
      ctaBody: "The inventory is the only section backed by a live scan today.",
      ctaAction: "Open inventory",
    },
    placeholder: {
      badge: "No data source yet",
      title: "Nothing to show here yet",
      body: "This section has no backend source wired up. It will fill in once its scanner lands.",
    },
    subscriptions: {
      sampleBadge: "Sample data",
      intro: "Active AI subscriptions, ordered by the next renewal.",
      summaryActive: "Active subscriptions",
      summaryMonthly: "Monthly spend",
      empty: "No active subscriptions.",
      planLabel: "Plan",
      amountLabel: "Amount",
      renewalLabel: "Renews on",
      cycleMonthly: "Monthly",
      cycleYearly: "Yearly",
      perMonth: "/month",
      perYear: "/year",
      renewsToday: "Renews today",
      renewsTomorrow: "Renews tomorrow",
      renewsInDays: "In {days} days",
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
      embedded: "Embedded (non-actionable)",
    },
    diagnostics: {
      title: "Scan diagnostics",
      unavailableRoots: "Unavailable scan roots",
      missingClient: "Supported client unavailable",
      recoverableIssues: "Recoverable scan issues",
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
      tagline: "Inventario de componentes de IA",
      languageLabel: "Idioma",
      languageEnglish: "Inglés",
      languageSpanish: "Español",
    },
    nav: {
      home: "Inicio",
      agents: "Agentes",
      skills: "Skills",
      mcp: "MCP",
      prompts: "Prompts",
      inventory: "Inventario",
      subscriptions: "Suscripciones de IA",
    },
    navGroup: {
      overview: "General",
      library: "Biblioteca",
      data: "Datos",
    },
    area: {
      home: "Inicio",
      agents: "Agentes",
      skills: "Skills",
      mcp: "MCP",
      prompts: "Prompts",
      inventory: "Inventario",
      subscriptions: "Suscripciones de IA",
    },
    home: {
      greeting: "Bienvenido a Vertice",
      subtitle:
        "Vertice analiza tu equipo y muestra todos los componentes de IA instalados en tus clientes",
      statComponents: "Componentes",
      statSkills: "Skills",
      statAgents: "Agentes",
      statRoots: "Raíces de escaneo",
      statsPending: "—",
      ctaTitle: "Empieza por el inventario",
      ctaBody: "El inventario es la única sección respaldada por un escaneo real hoy.",
      ctaAction: "Abrir inventario",
    },
    placeholder: {
      badge: "Sin fuente de datos",
      title: "Todavía no hay nada que mostrar",
      body: "Esta sección aún no tiene una fuente en el backend. Se rellenará cuando llegue su escáner.",
    },
    subscriptions: {
      sampleBadge: "Datos de ejemplo",
      intro: "Suscripciones de IA activas, ordenadas por próxima renovación.",
      summaryActive: "Suscripciones activas",
      summaryMonthly: "Gasto mensual",
      empty: "No hay suscripciones activas.",
      planLabel: "Plan",
      amountLabel: "Importe",
      renewalLabel: "Renovación",
      cycleMonthly: "Mensual",
      cycleYearly: "Anual",
      perMonth: "/mes",
      perYear: "/año",
      renewsToday: "Renueva hoy",
      renewsTomorrow: "Renueva mañana",
      renewsInDays: "En {days} días",
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
      embedded: "Integrado (sin acciones disponibles)",
    },
    diagnostics: {
      title: "Diagnósticos del escaneo",
      unavailableRoots: "Raíces de escaneo no disponibles",
      missingClient: "Cliente compatible no disponible",
      recoverableIssues: "Problemas recuperables del escaneo",
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
