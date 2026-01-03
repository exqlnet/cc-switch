import { useEffect, useMemo, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { Provider } from "@/types";
import type { AppId } from "@/lib/api";
import { streamCheckProvider } from "@/lib/api/model-test";

const MONITOR_INTERVAL_MS = 60_000;
const HISTORY_LIMIT = 20;
const MAX_CONCURRENCY = 3;

export function useAvailabilityMonitor(
  appId: AppId,
  providers: Record<string, Provider>,
) {
  const queryClient = useQueryClient();
  const cancelRef = useRef(false);
  const inFlightRef = useRef<Set<string>>(new Set());
  const lastRunAtRef = useRef<Record<string, number>>({});

  const enabledProviderIds = useMemo(() => {
    return Object.values(providers)
      .filter((provider) => provider.meta?.availability_monitor_enabled === true)
      .map((provider) => provider.id);
  }, [providers]);

  useEffect(() => {
    cancelRef.current = false;
    return () => {
      cancelRef.current = true;
    };
  }, []);

  useEffect(() => {
    if (enabledProviderIds.length === 0) {
      return;
    }

    const runChecks = async () => {
      const now = Date.now();
      const queue = enabledProviderIds.filter((providerId) => {
        if (inFlightRef.current.has(providerId)) return false;
        const lastRunAt = lastRunAtRef.current[providerId] ?? 0;
        return now - lastRunAt >= MONITOR_INTERVAL_MS;
      });

      if (queue.length === 0) return;

      const worker = async () => {
        while (!cancelRef.current) {
          const providerId = queue.shift();
          if (!providerId) return;
          if (inFlightRef.current.has(providerId)) continue;

          inFlightRef.current.add(providerId);
          lastRunAtRef.current[providerId] = Date.now();

          try {
            await streamCheckProvider(appId, providerId);
          } catch (e) {
            // 监控过程完全静默：不弹 Toast，仅记录到控制台，避免打扰用户
            console.warn("[availability-monitor] stream check failed", {
              appId,
              providerId,
              error: String(e),
            });
          } finally {
            inFlightRef.current.delete(providerId);
            queryClient.invalidateQueries({
              queryKey: ["streamCheckHistory", appId, providerId, HISTORY_LIMIT],
            });
          }
        }
      };

      await Promise.all(
        Array.from({ length: Math.min(MAX_CONCURRENCY, queue.length) }).map(
          () => worker(),
        ),
      );
    };

    const jitterMs = Math.floor(Math.random() * 3000);
    const initialTimer = window.setTimeout(() => {
      void runChecks();
    }, jitterMs);

    const interval = window.setInterval(() => {
      void runChecks();
    }, MONITOR_INTERVAL_MS);

    return () => {
      window.clearTimeout(initialTimer);
      window.clearInterval(interval);
    };
  }, [appId, enabledProviderIds, queryClient]);
}

