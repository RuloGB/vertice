<script lang="ts">
  type BrandMarkVariant = "hero" | "sidebar";

  let {
    compact = false,
    decorative = true,
    variant = "hero",
    class: className = "",
  }: {
    compact?: boolean;
    decorative?: boolean;
    variant?: BrandMarkVariant;
    class?: string;
  } = $props();
</script>

<div
  data-testid="brand-mark"
  aria-hidden={decorative ? "true" : undefined}
  class={["brand-mark", compact && "brand-mark--compact", `brand-mark--${variant}`, className]}
>
  <span class="brand-mark__left"></span>
  <span class="brand-mark__right"></span>
</div>

<style>
  .brand-mark {
    position: relative;
    width: 9.5rem;
    height: 8.5rem;
    place-items: center;
    filter: drop-shadow(var(--shadow-mark));
  }

  .brand-mark::before {
    position: absolute;
    inset: 0.4rem;
    border: 1px solid var(--stroke-mark);
    border-radius: 1.25rem;
    content: "";
    transform: rotate(45deg);
  }

  .brand-mark__left,
  .brand-mark__right {
    position: absolute;
    left: 50%;
    top: 2.55rem;
    width: 1.2rem;
    height: 4.75rem;
    border-radius: 999px;
    background: var(--gradient-mark);
  }

  .brand-mark__left {
    transform: translateX(calc(-50% - 1.55rem)) rotate(-27deg);
  }

  .brand-mark__right {
    transform: translateX(calc(-50% + 1.55rem)) rotate(27deg);
  }

  .brand-mark--compact {
    width: 2.5rem;
    height: 2.5rem;
  }

  .brand-mark--compact::before {
    inset: 0.26rem;
    border-radius: 0.55rem;
  }

  .brand-mark--compact .brand-mark__left,
  .brand-mark--compact .brand-mark__right {
    top: 0.75rem;
    width: 0.38rem;
    height: 1.2rem;
  }

  .brand-mark--compact .brand-mark__left {
    transform: translateX(calc(-50% - 0.42rem)) rotate(-27deg);
  }

  .brand-mark--compact .brand-mark__right {
    transform: translateX(calc(-50% + 0.42rem)) rotate(27deg);
  }

  .brand-mark--sidebar {
    border: 1px solid color-mix(in srgb, var(--color-stroke-strong) 72%, transparent);
    border-radius: 0.85rem;
    background: var(--color-surface-raised);
    box-shadow: 0 8px 18px -15px rgb(0 0 0 / 0.95);
    filter: none;
  }

  .brand-mark--sidebar::before {
    border-color: color-mix(in srgb, var(--color-interactive-hover) 25%, var(--color-stroke));
  }

  .brand-mark--sidebar .brand-mark__left,
  .brand-mark--sidebar .brand-mark__right {
    filter: saturate(0.88) brightness(0.9);
  }

  @media (prefers-reduced-motion: reduce) {
    .brand-mark {
      filter: none;
    }
  }
</style>
