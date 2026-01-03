import type { ReactNode } from "react";
import { renderHook } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import type { Provider } from "@/types";
import { useAvailabilityMonitor } from "@/hooks/useAvailabilityMonitor";

const streamCheckProviderMock = vi.fn();

vi.mock("@/lib/api/model-test", () => ({
  streamCheckProvider: (...args: unknown[]) => streamCheckProviderMock(...args),
}));

interface WrapperProps {
  children: ReactNode;
}

function createWrapper() {
  const queryClient = new QueryClient();
  const wrapper = ({ children }: WrapperProps) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  return { wrapper, queryClient };
}

describe("useAvailabilityMonitor", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    streamCheckProviderMock.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("does not run when no provider is enabled", async () => {
    const { wrapper } = createWrapper();
    const providers: Record<string, Provider> = {
      a: { id: "a", name: "A", settingsConfig: {}, meta: {} },
    };

    renderHook(() => useAvailabilityMonitor("claude", providers), { wrapper });

    await vi.runOnlyPendingTimersAsync();
    expect(streamCheckProviderMock).not.toHaveBeenCalled();
  });

  it("runs silently for enabled providers and invalidates history cache", async () => {
    vi.spyOn(Math, "random").mockReturnValue(0);
    streamCheckProviderMock.mockResolvedValue({
      status: "operational",
      success: true,
      message: "ok",
      testedAt: Math.floor(Date.now() / 1000),
      retryCount: 0,
      modelUsed: "mock",
    });

    const { wrapper, queryClient } = createWrapper();
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

    const providers: Record<string, Provider> = {
      a: {
        id: "a",
        name: "A",
        settingsConfig: {},
        meta: { availability_monitor_enabled: true },
      },
    };

    renderHook(() => useAvailabilityMonitor("claude", providers), { wrapper });

    await vi.runOnlyPendingTimersAsync();

    expect(streamCheckProviderMock).toHaveBeenCalled();
    expect(streamCheckProviderMock).toHaveBeenCalledWith("claude", "a");
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["streamCheckHistory", "claude", "a", 20],
    });
  });
});
