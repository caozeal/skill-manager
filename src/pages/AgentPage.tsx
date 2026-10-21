import { useState } from "react";
import { useParams } from "react-router-dom";
import { RefreshCw, Plus } from "lucide-react";
import {
  useAgents,
  useSkills,
  useMount,
  useAgentUsage,
  useRefreshUsage,
  useMcpServers,
  useMcpTargets,
} from "@/hooks";
import { SkillTable } from "@/components/SkillTable";
import { MountDialog } from "@/components/MountDialog";
import { McpTable, McpEditor } from "@/components/McpTable";
import { TabBtn } from "@/components/Tabs";
import type { UsageStat } from "@/types";

export function AgentPage() {
  const { id } = useParams<{ id: string }>();
  const { data: agents } = useAgents();
  const { data: skills, isLoading } = useSkills(id);
  const mount = useMount();
  const { data: usage } = useAgentUsage(id);
  const refreshUsage = useRefreshUsage();
  const { data: mcpAll, isLoading: mcpLoading } = useMcpServers();
  const { data: mcpTargets } = useMcpTargets();
  const [tab, setTab] = useState<"skills" | "mcp">("skills");
  const [mcpEditorOpen, setMcpEditorOpen] = useState(false);

  const mcpList = (mcpAll ?? []).filter((s) => s.agentId === id);

  // 该 agent 是否有可写入的本地 MCP 配置源 —— 由后端探测决定，不写死
  const hasMcpSource = !!mcpTargets?.some((t) => t.id === id && t.writable && t.available);

  const usageMap: Record<string, UsageStat> = Object.fromEntries(
    (usage?.stats ?? []).map((s) => [s.folder, s]),
  );

  const agent = agents?.find((a) => a.id === id);
  const centralSkills = useSkills("agents-central");
  const canUnmount =
    !!agent && agent.kind === "standard" && !agent.readOnly && id !== "agents-central";

  if (!agent) {
    return (
      <div className="p-6">
        {isLoading ? (
          <div className="text-sm text-gray-400">加载中…</div>
        ) : (
          <div className="text-sm text-gray-500">未知 agent: {id}</div>
        )}
      </div>
    );
  }

  if (agent.kind === "informational") {
    return (
      <div className="mx-auto max-w-2xl p-6">
        <h1 className="text-lg font-semibold">{agent.name}</h1>
        <div className="mt-4 rounded-xl border border-dashed border-black/10 bg-gray-50 p-5 text-[13px] leading-relaxed text-gray-600">
          {agent.note}
          <div className="mt-3 text-[12px] text-gray-400">
            该 agent 暂无本地 SKILL.md 技能目录，无法进行挂载管理。如需为其提供技能，请参考其官方文档的扩展机制。
          </div>
        </div>
      </div>
    );
  }

  const mounted = new Set(skills?.map((s) => s.folder) ?? []);
  const mountable =
    agent.id !== "agents-central" && agent.exists
      ? (centralSkills.data ?? []).filter(
          (s) => !mounted.has(s.folder) && !s.isSystem,
        )
      : [];

  // 本地/全局/兼容技能统计
  const localCount = (skills ?? []).filter((s) => !s.isGlobal && !s.isCompat).length;
  const globalCount = (skills ?? []).filter((s) => s.isGlobal).length;
  const compatCount = (skills ?? []).filter((s) => s.isCompat).length;

  return (
    <div className="mx-auto max-w-7xl p-6">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h1 className="text-lg font-semibold">{agent.name}</h1>
          <div className="mt-0.5 font-mono text-[11px] text-gray-400">
            {agent.skillsPath
              ? agent.skillsPath +
                (globalCount > 0 ? " · 全局: ~/.agents/skills" : "") +
                (compatCount > 0 ? " · Claude 兼容: ~/.claude/skills" : "")
              : globalCount > 0
                ? "全局技能来源: ~/.agents/skills"
                : ""}
          </div>
          <div className="mt-1 text-[12px] text-gray-500">
            {[
              localCount > 0 && `${localCount} 个本地技能`,
              compatCount > 0 && `${compatCount} 个 Claude 兼容`,
              globalCount > 0 && `${globalCount} 个全局技能（中央仓库直接生效）`,
            ]
              .filter(Boolean)
              .join(" · ")}
            {localCount === 0 && compatCount === 0 && globalCount === 0
              ? agent.exists
                ? `${agent.skillCount} 个技能`
                : "目录不存在"
              : ""}
          </div>
          <div className="mt-2 max-w-xl text-[11px] leading-relaxed text-gray-400">{agent.note}</div>
          {usage && (
            <div className="mt-2 flex items-center gap-2 text-[11px] text-gray-500">
              {usage.supported ? (
                <>
                  <span>
                    技能调用合计{" "}
                    <span className="font-medium text-gray-700 tabular-nums">{usage.total}</span> 次
                  </span>
                  <span className="text-gray-300">·</span>
                  <span>
                    {usage.precision === "exact" ? "精确统计" : "近似统计"}
                  </span>
                  {usage.lastScan && (
                    <>
                      <span className="text-gray-300">·</span>
                      <span>
                        扫描于{" "}
                        {usage.lastScan.replace("T", " ").replace("Z", "").slice(0, 16)}
                      </span>
                    </>
                  )}
                </>
              ) : (
                <span className="text-gray-400">{usage.note}</span>
              )}
              <button
                className="inline-flex items-center gap-1 rounded border border-black/10 px-1.5 py-0.5 text-[11px] text-gray-600 hover:bg-gray-50 disabled:opacity-50"
                disabled={refreshUsage.isPending}
                title={usage.note}
                onClick={() => id && refreshUsage.mutate(id)}
              >
                <RefreshCw
                  size={11}
                  className={refreshUsage.isPending ? "animate-spin" : ""}
                />
                {refreshUsage.isPending ? "统计中…" : "刷新统计"}
              </button>
            </div>
          )}
        </div>
        {mountable.length > 0 && (
          <MountDialog
            agentId={agent.id}
            agentName={agent.name}
            skillsPath={agent.skillsPath}
            candidates={mountable.map((s) => ({ folder: s.folder, name: s.name }))}
            onMount={(folder) =>
              mount.mutate({ agentId: agent.id, centralFolder: folder })
            }
            pending={mount.isPending}
            error={mount.error ? String((mount.error as Error).message) : null}
          />
        )}
      </div>

      {!agent.exists && agent.skillsPath && !agent.readOnly && (
        <div className="mt-4 rounded-lg border border-orange-200 bg-orange-50 p-3 text-[12px] text-orange-700">
          技能目录不存在，可先在对应 agent 中初始化，或通过上方「挂载技能」自动创建目录并挂载。
        </div>
      )}

      <div className="mt-4 flex items-center gap-1 border-b border-black/8">
        <TabBtn active={tab === "skills"} onClick={() => setTab("skills")}>
          技能 {skills ? `(${skills.length})` : ""}
        </TabBtn>
        <TabBtn active={tab === "mcp"} onClick={() => setTab("mcp")}>
          MCP {mcpList.length ? `(${mcpList.length})` : ""}
        </TabBtn>
        <div className="ml-auto pb-1.5">
          {tab === "mcp" && hasMcpSource && (
            <button
              className="inline-flex items-center gap-1 rounded-lg border border-black/10 px-2 py-1 text-[12px] text-gray-600 hover:bg-gray-50"
              onClick={() => setMcpEditorOpen(true)}
            >
              <Plus size={13} /> 新增 MCP
            </button>
          )}
        </div>
      </div>

      <div className="mt-3">
        {tab === "skills" ? (
          isLoading ? (
            <div className="py-14 text-center text-sm text-gray-400">扫描中…</div>
          ) : (
            <SkillTable skills={skills ?? []} canUnmount={canUnmount} usage={usageMap} />
          )
        ) : mcpList.length > 0 ? (
          <McpTable servers={mcpList} showAgent={false} isLoading={mcpLoading} />
        ) : (
          <div className="py-14 text-center text-sm text-gray-400">
            {hasMcpSource
              ? "该 agent 还没有配置任何 MCP，可点右上角「新增 MCP」添加。"
              : "该 agent 没有本地 MCP 配置文件，暂不支持在 Skill Manager 中管理。"}
          </div>
        )}
      </div>

      {mcpEditorOpen && (
        <McpEditor
          server={null}
          defaultAgent={id}
          onClose={() => setMcpEditorOpen(false)}
        />
      )}
    </div>
  );
}

