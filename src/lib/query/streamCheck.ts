import { useQuery } from "@tanstack/react-query";
import type { AppId } from "@/lib/api";
import { getStreamCheckHistory, type StreamCheckResult } from "@/lib/api/model-test";

export function useStreamCheckHistory(
  providerId: string,
  appType: AppId,
  limit: number = 20,
) {
  return useQuery<StreamCheckResult[]>({
    queryKey: ["streamCheckHistory", appType, providerId, limit],
    queryFn: () => getStreamCheckHistory(appType, providerId, limit),
    enabled: !!providerId && !!appType,
    staleTime: 30_000,
    retry: false,
    refetchOnWindowFocus: false,
  });
}

