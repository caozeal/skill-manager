import { invoke } from "@tauri-apps/api/core";
import type {
  Agent,
  AgentUsage,
  FrontmatterUpdate,
  McpServer,
  McpServerInput,
  McpTarget,
  Skill,
  SkillDetail,
} from "@/types";

export class AppError extends Error {
  code: string;
  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    const err = e as { code?: string; message?: string };
    throw new AppError(err.code ?? "Unknown", err.message ?? String(e));
  }
}

export const api = {
  listAgents: () => call<Agent[]>("list_agents"),
  scanSkills: (agentId: string) => call<Skill[]>("scan_skills", { agentId }),
  scanAllSkills: () => call<Skill[]>("scan_all_skills"),
  getSkillDetail: (agentId: string, folder: string) =>
    call<SkillDetail>("get_skill_detail", { agentId, folder }),
  searchSkills: (query: string) => call<Skill[]>("search_skills", { query }),
  mountSkill: (agentId: string, centralFolder: string) =>
    call<Skill>("mount_skill", { agentId, centralFolder }),
  unmountSkill: (agentId: string, folder: string) =>
    call<void>("unmount_skill", { agentId, folder }),
  updateFrontmatter: (agentId: string, folder: string, update: FrontmatterUpdate) =>
    call<void>("update_frontmatter", { agentId, folder, update }),
  disableSkill: (agentId: string, folder: string) =>
    call<void>("disable_skill", { agentId, folder }),
  enableSkill: (agentId: string, folder: string) =>
    call<void>("enable_skill", { agentId, folder }),
  deleteSkill: (agentId: string, folder: string) =>
    call<void>("delete_skill", { agentId, folder }),
  getAgentUsage: (agentId: string) =>
    call<AgentUsage>("get_agent_usage", { agentId }),
  refreshAgentUsage: (agentId: string) =>
    call<AgentUsage>("refresh_agent_usage", { agentId }),
  getAllUsage: () => call<AgentUsage[]>("get_all_usage"),

  listMcpServers: () => call<McpServer[]>("list_mcp_servers"),
  listMcpTargets: () => call<McpTarget[]>("list_mcp_targets"),
  setMcpEnabled: (id: string, enabled: boolean) =>
    call<void>("set_mcp_enabled", { id, enabled }),
  upsertMcpServer: (agentId: string, name: string, input: McpServerInput) =>
    call<void>("upsert_mcp_server", { agentId, name, input }),
  removeMcpServer: (id: string) => call<void>("remove_mcp_server", { id }),
  syncMcpServer: (id: string, targets: string[]) =>
    call<number>("sync_mcp_server", { id, targets }),
};
