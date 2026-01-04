import { useQuery } from "@tanstack/react-query";
import type { AppId } from "@/lib/api";
import { getStreamCheckHistory, type StreamCheckResult } from "@/lib/api/model-test";

export interface UseStreamCheckHistoryOptions {
  enabled?: boolean;
}

export function useStreamCheckHistory(
  providerId: string,
  appType: AppId,
  limit: number = 20,
  options?: UseStreamCheckHistoryOptions,
) {
  const { enabled = true } = options ?? {};
  return useQuery<StreamCheckResult[]>({
    queryKey: ["streamCheckHistory", appType, providerId, limit],
    queryFn: () => getStreamCheckHistory(appType, providerId, limit),
    enabled: enabled && !!providerId && !!appType,
    staleTime: 30_000,
    retry: false,
    refetchOnWindowFocus: false,
  });
}
