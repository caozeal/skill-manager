import { useMemo, useState } from "react";
import {
  Link2,
  Folder,
  Circle,
  Minus,
  Globe2,
  Layers,
  TriangleAlert,
  Plus,
} from "lucide-react";
import { useAgents, useAllSkills, useMcpServers } from "@/hooks";
import { TabBtn } from "@/components/Tabs";
import { McpEditor } from "@/components/McpTable";
import type { Agent, McpServer, Skill } from "@/types";
import { cn } from "@/lib/utils";

type CellStatus = "symlink" | "local" | "global" | "compat" | "disabled" | "none" | "na";

export function MatrixPage() {
  const { data: agents } = useAgents();
  const { data: allSkills, isLoading } = useAllSkills();
  const [tab, setTab] = useState<"skills" | "mcp">("skills");
  const [mcpEditorOpen, setMcpEditorOpen] = useState(false);

  const mountableAgents = useMemo(
    () =>
      (agents ?? []).filter(
        (a) =>
          a.kind === "standard" &&
          a.id !== "agents-central" &&
          (a.exists || a.skillCount > 0),
      ),
    [agents],
  );

  const { rows } = useMemo(() => {
    const skillsByAgent = new Map<string, Skill[]>();
    for (const s of allSkills ?? []) {
      if (!skillsByAgent.has(s.agentId)) skillsByAgent.set(s.agentId, []);
      skillsByAgent.get(s.agentId)!.push(s);
    }
    const centralFolders = new Set(
      (skillsByAgent.get("agents-central") ?? []).map((s) => s.folder),
    );

    const folderMap = new Map<string, Record<string, { status: CellStatus; skill: Skill }>>();
    for (const a of mountableAgents) {
      for (const s of skillsByAgent.get(a.id) ?? []) {
        if (s.isSystem) continue;
        const status: CellStatus = s.disabled
          ? "disabled"
          : s.isGlobal
            ? "global"
            : s.isCompat
              ? "compat"
              : s.mount.mountType === "symlink" || s.mount.mountType === "broken_symlink"
                ? "symlink"
                : "local";
        if (!folderMap.has(s.folder)) folderMap.set(s.folder, {});
        folderMap.get(s.folder)![a.id] = { status, skill: s };
      }
    }
    // 中央仓库中未被任何 agent 挂载的技能也显示为行
    for (const f of centralFolders) {
      if (!folderMap.has(f)) folderMap.set(f, {});
    }

    const rows = [...folderMap.entries()]
      .map(([folder, cells]) => ({
        folder,
        inCentral: centralFolders.has(folder),
        cells,
        name: firstNonEmptyName(cells) ?? folder,
      }))
      .sort((a, b) => a.name.localeCompare(b.name));
    return { rows };
  }, [allSkills, mountableAgents]);

  if (!agents || isLoading) {
    return <div className="p-6 text-sm text-gray-400">加载中…</div>;
  }

  return (
    <div className="mx-auto max-w-5xl p-6">
      <h1 className="text-lg font-semibold">挂载矩阵</h1>
      <p className="mt-1 text-[13px] text-gray-500">
        技能 × agent 挂载关系一览 · 每行一个技能，每列一个 agent
      </p>

      <div className="mt-3 flex items-center gap-1 border-b border-black/8">
        <TabBtn active={tab === "skills"} onClick={() => setTab("skills")}>
          技能
        </TabBtn>
        <TabBtn active={tab === "mcp"} onClick={() => setTab("mcp")}>
          MCP
        </TabBtn>
        <div className="ml-auto pb-1.5">
          {tab === "mcp" && (
            <button
              className="inline-flex items-center gap-1 rounded-lg border border-black/10 px-2 py-1 text-[12px] text-gray-600 hover:bg-gray-50"
              onClick={() => setMcpEditorOpen(true)}
            >
              <Plus size={13} /> 新增 MCP
            </button>
          )}
        </div>
      </div>

      {tab === "mcp" ? (
        <McpMatrix />
      ) : (
        <>
      <div className="mt-2 flex flex-wrap gap-4 text-[11px] text-gray-500">
        <Legend icon={<Link2 size={12} className="text-emerald-500" />} label="符号链接挂载" />
        <Legend
          icon={<Folder size={12} className="text-blue-500" />}
          label="本地真实目录（可能与中央仓库不同步）"
        />
        <Legend icon={<Globe2 size={12} className="text-violet-500" />} label="全局技能（中央仓库直接生效）" />
        <Legend icon={<Layers size={12} className="text-cyan-500" />} label="Claude 兼容（~/.claude/skills）" />
        <Legend icon={<Circle size={12} className="text-gray-300" />} label="未挂载" />
        <Legend icon={<Minus size={12} className="text-gray-300" />} label="不适用" />
      </div>

      <div className="mt-4 overflow-x-auto rounded-xl border border-black/8 bg-white">
        <table className="w-full text-[12px]">
          <thead>
            <tr className="border-b border-black/8 bg-gray-50/60">
              <th className="sticky left-0 z-10 bg-gray-50 px-4 py-2.5 text-left font-medium">
                技能
              </th>
              <th className="px-2 py-2.5 text-center font-medium">中央仓库</th>
              {mountableAgents.map((a) => (
                <th key={a.id} className="px-2 py-2.5 text-center font-medium" title={a.note}>
                  <div className="flex flex-col items-center gap-0.5">
                    <AgentInitial agent={a} />
                    <span className="text-[10px] text-gray-500">{a.name}</span>
                  </div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr
                key={row.folder}
                className="border-b border-black/5 last:border-0 hover:bg-blue-50/30"
              >
                <td className="sticky left-0 z-10 max-w-[220px] bg-white px-4 py-2">
                  <div className="truncate font-medium text-gray-800" title={row.name}>
                    {row.name}
                  </div>
                  <div className="truncate text-[10px] text-gray-400">{row.folder}</div>
                </td>
                <td className="px-2 py-2 text-center">
                  {row.inCentral ? (
                    <span className="text-emerald-600" title="存在于中央仓库">
                      ●
                    </span>
                  ) : (
                    <span className="text-gray-300" title="仅存在于个别 agent 本地">
                      ○
                    </span>
                  )}
                </td>
                {mountableAgents.map((a) => {
                  const cell = row.cells[a.id];
                  return (
                    <td key={a.id} className="px-2 py-2 text-center">
                      <Cell status={cell?.status ?? (row.inCentral ? "none" : "na")} />
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {rows.length === 0 && (
        <div className="py-14 text-center text-sm text-gray-400">没有扫描到技能</div>
      )}
        </>
      )}

      {mcpEditorOpen && (
        <McpEditor server={null} onClose={() => setMcpEditorOpen(false)} />
      )}
    </div>
  );
}

/** MCP × agent 矩阵：行是 MCP 名称，列是配置了该 agent 的来源 */
function McpMatrix() {
  const { data: agents } = useAgents();
  const { data: servers, isLoading } = useMcpServers();

  const cols = useMemo(() => {
    const ids = new Set((servers ?? []).map((s) => s.agentId));
    return (agents ?? []).filter((a) => ids.has(a.id));
  }, [agents, servers]);

  const rows = useMemo(() => {
    const map = new Map<string, { cells: Record<string, McpServer>; conflict: boolean }>();
    const fps = new Map<string, Set<string>>();
    for (const s of servers ?? []) {
      // 矩阵只统计全局 MCP；项目级属于单个工程，横向对比没有意义
      if (s.scope !== "global") continue;
      if (!map.has(s.name)) map.set(s.name, { cells: {}, conflict: false });
      map.get(s.name)!.cells[s.agentId] = s;
      const fp = s.url ?? [s.command ?? "", ...s.args].join(" ");
      if (!fps.has(s.name)) fps.set(s.name, new Set());
      fps.get(s.name)!.add(fp);
    }
    for (const [name, set] of fps) {
      if (set.size > 1 && map.has(name)) map.get(name)!.conflict = true;
    }
    return [...map.entries()]
      .map(([name, v]) => ({ name, ...v }))
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [servers]);

  if (isLoading) return <div className="py-14 text-center text-sm text-gray-400">加载中…</div>;

  return (
    <>
      <div className="mt-3 flex flex-wrap gap-4 text-[11px] text-gray-500">
        <Legend
          icon={<span className="inline-block h-2.5 w-2.5 rounded-full bg-emerald-500" />}
          label="已配置并启用"
        />
        <Legend
          icon={
            <span className="inline-block h-2.5 w-2.5 rounded-full border border-gray-400 bg-white" />
          }
          label="已禁用"
        />
        <Legend
          icon={<span className="inline-block h-2.5 w-2.5 rounded-full border border-gray-300 bg-white" />}
          label="未配置"
        />
        <Legend
          icon={<TriangleAlert size={12} className="text-amber-500" />}
          label="跨 agent 配置不一致"
        />
      </div>

      <div className="mt-4 overflow-x-auto rounded-xl border border-black/8 bg-white">
        <table className="w-full text-[12px]">
          <thead>
            <tr className="border-b border-black/8 bg-gray-50/60">
              <th className="sticky left-0 z-10 bg-gray-50 px-4 py-2.5 text-left font-medium">
                MCP
              </th>
              {cols.map((a) => (
                <th key={a.id} className="px-2 py-2.5 text-center font-medium" title={a.note}>
                  <div className="flex flex-col items-center gap-0.5">
                    <AgentInitial agent={a} />
                    <span className="text-[10px] text-gray-500">{a.name}</span>
                  </div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr
                key={row.name}
                className="border-b border-black/5 last:border-0 hover:bg-blue-50/30"
              >
                <td className="sticky left-0 z-10 bg-white px-4 py-2">
                  <div className="flex items-center gap-1.5">
                    <span className="font-medium text-gray-800">{row.name}</span>
                    {row.conflict && (
                      <span
                        className="rounded bg-amber-50 px-1 text-[10px] text-amber-700"
                        title="同名 MCP 在不同 agent 中配置不一致"
                      >
                        不一致
                      </span>
                    )}
                  </div>
                </td>
                {cols.map((a) => (
                  <td key={a.id} className="px-2 py-2 text-center">
                    <McpCell server={row.cells[a.id]} />
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {rows.length === 0 && (
          <div className="py-14 text-center text-sm text-gray-400">
            没有扫描到全局 MCP 配置
          </div>
        )}
      </div>
      <p className="mt-2 text-[11px] text-gray-400">
        只统计全局 MCP；项目级（Claude 的 projects 段）不参与横向对比。启用/禁用、编辑、同步请到对应
        agent 页的「MCP」标签页操作。
      </p>
    </>
  );
}

function McpCell({ server }: { server?: McpServer }) {
  if (!server)
    return (
      <span
        className="inline-block h-2.5 w-2.5 rounded-full border border-gray-300 bg-white"
        title="未配置"
      />
    );
  const ep = server.url ?? [server.command ?? "", ...server.args].join(" ");
  if (!server.enabled)
    return (
      <span
        className="inline-block h-2.5 w-2.5 rounded-full border border-gray-400 bg-white"
        title={`已禁用 · ${ep}`}
      />
    );
  return (
    <span className="inline-block h-2.5 w-2.5 rounded-full bg-emerald-500" title={ep} />
  );
}

function firstNonEmptyName(cells: Record<string, { status: CellStatus; skill: Skill }>) {
  for (const v of Object.values(cells)) {
    if (v.skill.name) return v.skill.name;
  }
  return null;
}

function Cell({ status }: { status: CellStatus }) {
  if (status === "symlink")
    return (
      <span className="inline-block h-2.5 w-2.5 rounded-full bg-emerald-500" title="符号链接挂载" />
    );
  if (status === "local")
    return (
      <span className="inline-block h-2.5 w-2.5 rounded-full bg-blue-500" title="本地真实目录" />
    );
  if (status === "global")
    return (
      <span
        className="inline-block h-2.5 w-2.5 rounded-full bg-violet-500"
        title="全局技能：位于中央仓库 ~/.agents/skills，直接生效"
      />
    );
  if (status === "compat")
    return (
      <span
        className="inline-block h-2.5 w-2.5 rounded-full bg-cyan-500"
        title="Claude 兼容技能：来自 ~/.claude/skills"
      />
    );
  if (status === "disabled")
    return (
      <span
        className="inline-flex items-center gap-0.5 text-gray-400"
        title="已禁用（已移出 skills 目录，agent 不再加载）"
      >
        <span className="inline-block h-2.5 w-2.5 rounded-full border border-gray-400 bg-white" />
        <span className="text-[9px]">✕</span>
      </span>
    );
  if (status === "none")
    return (
      <span
        className="inline-block h-2.5 w-2.5 rounded-full border border-gray-300 bg-white"
        title="未挂载"
      />
    );
  return (
    <span className="text-gray-300" title="不适用">
      —
    </span>
  );
}

function Legend({ icon, label }: { icon: React.ReactNode; label: string }) {
  return (
    <span className="inline-flex items-center gap-1">
      {icon} {label}
    </span>
  );
}

function AgentInitial({ agent }: { agent: Agent }) {
  return (
    <span
      className={cn(
        "flex h-6 w-6 items-center justify-center rounded-md text-[10px] font-semibold text-white",
        hashColor(agent.id),
      )}
    >
      {agent.name.slice(0, 2)}
    </span>
  );
}

function hashColor(id: string): string {
  const colors = [
    "bg-blue-500",
    "bg-emerald-500",
    "bg-violet-500",
    "bg-orange-500",
    "bg-rose-500",
    "bg-cyan-600",
    "bg-indigo-500",
  ];
  let h = 0;
  for (const c of id) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return colors[h % colors.length];
}
