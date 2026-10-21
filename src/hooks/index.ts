import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import type { FrontmatterUpdate, McpServerInput } from "@/types";

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

/** 某个 agent 的技能使用统计（按 agent 分开统计，读缓存，无缓存时后端现算） */
export function useAgentUsage(agentId: string | undefined) {
  return useQuery({
    queryKey: ["usage", agentId],
    queryFn: () => api.getAgentUsage(agentId!),
    enabled: !!agentId,
    staleTime: 60_000,
  });
}

/** 全部 agent 的使用统计（总览页） */
export function useAllUsage() {
  return useQuery({
    queryKey: ["usage", "all"],
    queryFn: api.getAllUsage,
    staleTime: 60_000,
  });
}

/** 强制重新扫描某个 agent 的会话日志 */
export function useRefreshUsage() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (agentId: string) => api.refreshAgentUsage(agentId),
    onSuccess: (_data, agentId) => {
      qc.invalidateQueries({ queryKey: ["usage", agentId] });
      qc.invalidateQueries({ queryKey: ["usage", "all"] });
    },
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

export function useDisable() {
  const invalidate = useInvalidateAll();
  return useMutation({
    mutationFn: ({ agentId, folder }: { agentId: string; folder: string }) =>
      api.disableSkill(agentId, folder),
    onSuccess: invalidate,
  });
}

export function useEnable() {
  const invalidate = useInvalidateAll();
  return useMutation({
    mutationFn: ({ agentId, folder }: { agentId: string; folder: string }) =>
      api.enableSkill(agentId, folder),
    onSuccess: invalidate,
  });
}

export function useDelete() {
  const invalidate = useInvalidateAll();
  return useMutation({
    mutationFn: ({ agentId, folder }: { agentId: string; folder: string }) =>
      api.deleteSkill(agentId, folder),
    onSuccess: invalidate,
  });
}

// ---------------------------------------------------------------- MCP / 工具

export function useMcpServers() {
  return useQuery({
    queryKey: ["mcp"],
    queryFn: api.listMcpServers,
    staleTime: 10_000,
  });
}

/** MCP 配置目标清单（可写/可管理的 agent）；来源为后端运行时探测，不写死 */
export function useMcpTargets() {
  return useQuery({
    queryKey: ["mcp-targets"],
    queryFn: api.listMcpTargets,
    staleTime: 30_000,
  });
}

/**
 * MCP 相关的 agent 元信息：显示名 + 可作为写入目标（新建/同步下拉）的 agent。
 * 显示名统一取自后端 registry，与侧边栏、技能页保持一致。
 */
export function useMcpAgentMeta() {
  const { data: agents } = useAgents();
  const { data: targets } = useMcpTargets();

  const nameOf = (id: string) =>
    targets?.find((t) => t.id === id)?.name ?? agents?.find((a) => a.id === id)?.name ?? id;

  const writableTargets = (targets ?? []).filter((t) => t.writable && t.available);

  return { nameOf, writableTargets };
}

function useInvalidateMcp() {
  const qc = useQueryClient();
  return () => {
    qc.invalidateQueries({ queryKey: ["mcp"] });
    qc.invalidateQueries({ queryKey: ["mcp-targets"] });
  };
}

export function useSetMcpEnabled() {
  const invalidate = useInvalidateMcp();
  return useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      api.setMcpEnabled(id, enabled),
    onSuccess: invalidate,
  });
}

export function useUpsertMcp() {
  const invalidate = useInvalidateMcp();
  return useMutation({
    mutationFn: ({
      agentId,
      name,
      input,
    }: {
      agentId: string;
      name: string;
      input: McpServerInput;
    }) => api.upsertMcpServer(agentId, name, input),
    onSuccess: invalidate,
  });
}

export function useRemoveMcp() {
  const invalidate = useInvalidateMcp();
  return useMutation({
    mutationFn: (id: string) => api.removeMcpServer(id),
    onSuccess: invalidate,
  });
}

export function useSyncMcp() {
  const invalidate = useInvalidateMcp();
  return useMutation({
    mutationFn: ({ id, targets }: { id: string; targets: string[] }) =>
      api.syncMcpServer(id, targets),
    onSuccess: invalidate,
  });
}
