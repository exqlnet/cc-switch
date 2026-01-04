import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import type { Provider } from "@/types";
import { ProviderCard } from "@/components/providers/ProviderCard";

vi.mock("@/components/providers/ProviderActions", () => ({
  ProviderActions: () => null,
}));

vi.mock("@/components/ProviderIcon", () => ({
  ProviderIcon: () => null,
}));

vi.mock("@/components/UsageFooter", () => ({
  default: () => null,
}));

vi.mock("@/components/providers/ProviderHealthBadge", () => ({
  ProviderHealthBadge: () => null,
}));

vi.mock("@/components/providers/FailoverPriorityBadge", () => ({
  FailoverPriorityBadge: () => null,
}));

vi.mock("@/lib/query/failover", () => ({
  useProviderHealth: () => ({ data: null }),
}));

vi.mock("@/lib/query", () => ({
  useStreamCheckHistory: () => ({ data: [] }),
}));

vi.mock("@/lib/query/queries", () => ({
  useUsageQuery: () => ({ data: null }),
}));

function createProvider(overrides: Partial<Provider> = {}): Provider {
  return {
    id: overrides.id ?? "provider-1",
    name: overrides.name ?? "Test Provider",
    settingsConfig: overrides.settingsConfig ?? {},
    meta: overrides.meta,
    category: overrides.category,
    websiteUrl: overrides.websiteUrl,
  } as Provider;
}

describe("ProviderCard availability row", () => {
  const baseProps = {
    isCurrent: false,
    appId: "claude" as const,
    onSwitch: vi.fn(),
    onEdit: vi.fn(),
    onDelete: vi.fn(),
    onConfigureUsage: vi.fn(),
    onOpenWebsite: vi.fn(),
    onDuplicate: vi.fn(),
    isProxyRunning: false,
    isProxyTakeover: false,
  };

  it("does not render availability row when monitor is disabled", () => {
    render(
      <ProviderCard
        {...baseProps}
        provider={createProvider({
          meta: { availability_monitor_enabled: false },
        })}
      />,
    );

    expect(screen.queryByText("可用性")).not.toBeInTheDocument();
  });

  it("renders availability row when monitor is enabled", () => {
    render(
      <ProviderCard
        {...baseProps}
        provider={createProvider({
          meta: { availability_monitor_enabled: true },
        })}
      />,
    );

    expect(screen.getByText("可用性")).toBeInTheDocument();
  });
});

