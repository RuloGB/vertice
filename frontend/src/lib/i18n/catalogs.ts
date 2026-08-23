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
    scanTitle: string;
    scanHealthy: string;
    scanIssues: string;
    scanFailed: string;
    scanRetry: string;
    scanOpen: string;
    scanPending: string;
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
    reload: string;
    reloading: string;
  };
  kind: {
    skill: string;
    agent: string;
  };
  components: {
    loading: string;
    empty: string;
    duplicate: string;
    duplicateTitle: string;
    embedded: string;
    paginationSummary: string;
    paginationPage: string;
    paginationPageSize: string;
    paginationFirst: string;
    paginationPrevious: string;
    paginationNext: string;
    paginationLast: string;
  };
  diagnostics: {
    title: string;
    recoverableIssues: string;
  };
  scan: {
    verdictHealthy: string;
    verdictIssues: string;
    rootsTitle: string;
    rootFound: string;
    rootNotFound: string;
    clientsTitle: string;
    clientDetected: string;
    clientNotDetected: string;
    clientVersionUnavailable: string;
    clientsUnsupportedPlatform: string;
    durationLabel: string;
    durationValue: string;
  };
  incident: {
    label: string;
    count: string;
    action: string;
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
      scan: "Scan",
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
      scan: "Scan",
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
      ctaTitle: "Browse your components",
      ctaBody: "Agents and Skills are backed by the startup scan.",
      ctaAction: "Open agents",
      scanTitle: "Last scan",
      scanHealthy: "Healthy — no incidents.",
      scanIssues: "Completed with {count} incidents in {ms} ms.",
      scanFailed: "The scan failed.",
      scanRetry: "Retry scan",
      scanOpen: "Open scan report",
      scanPending: "Scanning...",
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
      reload: "Reload",
      reloading: "Reloading...",
    },
    kind: {
      skill: "Skill",
      agent: "Agent",
    },
    components: {
      loading: "Scanning for installed components...",
      empty: "No components to show.",
      duplicate: "Duplicate",
      duplicateTitle: "Found at {count} locations",
      embedded: "Embedded (non-actionable)",
      paginationSummary: "Showing {from}–{to} of {total} components",
      paginationPage: "Page {current} of {total}",
      paginationPageSize: "Components per page",
      paginationFirst: "Go to first page",
      paginationPrevious: "Go to previous page",
      paginationNext: "Go to next page",
      paginationLast: "Go to last page",
    },
    diagnostics: {
      title: "Scan diagnostics",
      recoverableIssues: "Recoverable scan issues",
    },
    scan: {
      verdictHealthy: "Scan completed with no incidents.",
      verdictIssues: "Scan completed with {count} incidents.",
      rootsTitle: "Scan roots",
      rootFound: "Found",
      rootNotFound: "Not found",
      clientsTitle: "Supported clients",
      clientDetected: "Detected",
      clientNotDetected: "Not detected",
      clientVersionUnavailable: "Version unavailable",
      clientsUnsupportedPlatform: "Client installation detection is not supported on this platform.",
      durationLabel: "Duration",
      durationValue: "{ms} ms",
    },
    incident: {
      label: "Scan incidents",
      count: "{count} scan incidents",
      action: "Open the scan report",
    },
    failure: {
      title: "Scan failed.",
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
      scan: "Escaneo",
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
      scan: "Escaneo",
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
      ctaTitle: "Explora tus componentes",
      ctaBody: "Agents y Skills se apoyan en el escaneo de arranque.",
      ctaAction: "Abrir agentes",
      scanTitle: "Último escaneo",
      scanHealthy: "Correcto: sin incidencias.",
      scanIssues: "Terminó con {count} incidencias en {ms} ms.",
      scanFailed: "El escaneo falló.",
      scanRetry: "Reintentar escaneo",
      scanOpen: "Abrir informe del escaneo",
      scanPending: "Escaneando...",
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
      reload: "Recargar",
      reloading: "Recargando...",
    },
    kind: {
      skill: "Skill",
      agent: "Agent",
    },
    components: {
      loading: "Escaneando componentes instalados...",
      empty: "No hay componentes para mostrar.",
      duplicate: "Duplicado",
      duplicateTitle: "Encontrado en {count} ubicaciones",
      embedded: "Integrado (sin acciones disponibles)",
      paginationSummary: "Mostrando {from}–{to} de {total} componentes",
      paginationPage: "Página {current} de {total}",
      paginationPageSize: "Componentes por página",
      paginationFirst: "Ir a la primera página",
      paginationPrevious: "Ir a la página anterior",
      paginationNext: "Ir a la página siguiente",
      paginationLast: "Ir a la última página",
    },
    diagnostics: {
      title: "Diagnósticos del escaneo",
      recoverableIssues: "Problemas recuperables del escaneo",
    },
    scan: {
      verdictHealthy: "El escaneo terminó sin incidencias.",
      verdictIssues: "El escaneo terminó con {count} incidencias.",
      rootsTitle: "Raíces de escaneo",
      rootFound: "Encontrada",
      rootNotFound: "No encontrada",
      clientsTitle: "Clientes compatibles",
      clientDetected: "Detectado",
      clientNotDetected: "No detectado",
      clientVersionUnavailable: "Versión no disponible",
      clientsUnsupportedPlatform: "La detección de instalaciones de clientes no es compatible con esta plataforma.",
      durationLabel: "Duración",
      durationValue: "{ms} ms",
    },
    incident: {
      label: "Incidencias del escaneo",
      count: "{count} incidencias del escaneo",
      action: "Abrir el informe del escaneo",
    },
    failure: {
      title: "Falló el escaneo.",
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
