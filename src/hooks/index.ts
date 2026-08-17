import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import type { FrontmatterUpdate } from "@/types";

export function useAgents() {
  return useQuery({ queryKey: ["agents"], queryFn: api.listAgents });
}

export function useSkills(agentId: string | undefined) {
  return useQuery({
    queryKey: ["skills", agentId],
    queryFn: () => api.scanSkills(agentId!),
    enabled: !!agentId,
  });
}

export function useAllSkills() {
  return useQuery({
    queryKey: ["all-skills"],
    queryFn: api.scanAllSkills,
  });
}

export function useSkillDetail(agentId: string | undefined, folder: string | undefined) {
  return useQuery({
    queryKey: ["skill-detail", agentId, folder],
    queryFn: () => api.getSkillDetail(agentId!, folder!),
    enabled: !!agentId && !!folder,
  });
}

export function useSearch(query: string) {
  return useQuery({
    queryKey: ["search", query],
    queryFn: () => api.searchSkills(query),
    enabled: query.trim().length > 0,
    staleTime: 5_000,
  });
}

function useInvalidateAll() {
  const qc = useQueryClient();
  return () => {
    qc.invalidateQueries({ queryKey: ["agents"] });
    qc.invalidateQueries({ queryKey: ["skills"] });
    qc.invalidateQueries({ queryKey: ["all-skills"] });
    qc.invalidateQueries({ queryKey: ["skill-detail"] });
    qc.invalidateQueries({ queryKey: ["search"] });
  };
}

export function useMount() {
  const invalidate = useInvalidateAll();
  return useMutation({
    mutationFn: ({ agentId, centralFolder }: { agentId: string; centralFolder: string }) =>
      api.mountSkill(agentId, centralFolder),
    onSuccess: invalidate,
  });
}

export function useUnmount() {
  const invalidate = useInvalidateAll();
  return useMutation({
    mutationFn: ({ agentId, folder }: { agentId: string; folder: string }) =>
      api.unmountSkill(agentId, folder),
    onSuccess: invalidate,
  });
}

export function useUpdateFrontmatter() {
  const invalidate = useInvalidateAll();
  return useMutation({
    mutationFn: ({
      agentId,
      folder,
      update,
    }: {
      agentId: string;
      folder: string;
      update: FrontmatterUpdate;
    }) => api.updateFrontmatter(agentId, folder, update),
    onSuccess: invalidate,
  });
}
