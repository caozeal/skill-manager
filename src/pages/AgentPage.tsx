import { useParams } from "react-router-dom";
import { useAgents, useSkills, useMount } from "@/hooks";
import { SkillTable } from "@/components/SkillTable";
import { MountDialog } from "@/components/MountDialog";

export function AgentPage() {
  const { id } = useParams<{ id: string }>();
  const { data: agents } = useAgents();
  const { data: skills, isLoading } = useSkills(id);
  const mount = useMount();

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
                ? `${agent.skillCount} 个技能 · ${agent.symlinkCount} 符号链接 · ${agent.realCount} 本地目录`
                : "目录不存在"
              : ""}
          </div>
          <div className="mt-2 max-w-xl text-[11px] leading-relaxed text-gray-400">{agent.note}</div>
        </div>
        {mountable.length > 0 && (
          <MountDialog
            agentId={agent.id}
            agentName={agent.name}
            candidates={mountable.map((s) => ({ folder: s.folder, name: s.name }))}
            onMount={(folder) =>
              mount.mutate({ agentId: agent.id, centralFolder: folder })
            }
            pending={mount.isPending}
            error={mount.error ? String((mount.error as Error).message) : null}
          />
        )}
      </div>

      {!agent.exists && agent.skillsPath && (
        <div className="mt-4 rounded-lg border border-orange-200 bg-orange-50 p-3 text-[12px] text-orange-700">
          技能目录不存在，可先在对应 agent 中初始化，或通过上方「挂载技能」自动创建目录并挂载。
        </div>
      )}

      <div className="mt-4">
        {isLoading ? (
          <div className="py-14 text-center text-sm text-gray-400">扫描中…</div>
        ) : (
          <SkillTable skills={skills ?? []} canUnmount={canUnmount} />
        )}
      </div>
    </div>
  );
}
