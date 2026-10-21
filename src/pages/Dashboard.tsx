import { Link } from "react-router-dom";
import { Link2, Folder, Database, Grid3X3, Search } from "lucide-react";
import { useAgents } from "@/hooks";
import { useSearchStore } from "@/stores/ui";
import { useSearch } from "@/hooks";
import { formatTime } from "@/lib/utils";

export function Dashboard() {
  const { data: agents, isLoading } = useAgents();
  const { query, setQuery } = useSearchStore();
  const { data: results } = useSearch(query);

  const standard = agents?.filter((a) => a.kind === "standard") ?? [];
  const informational = agents?.filter((a) => a.kind === "informational") ?? [];
  const central = standard.find((a) => a.id === "agents-central");
  const totalSkills = standard.reduce((n, a) => n + a.skillCount, 0);

  return (
    <div className="mx-auto max-w-4xl p-6">
      <h1 className="text-lg font-semibold">总览</h1>
      <p className="mt-1 text-[13px] text-gray-500">
        管理各 AI coding agent 挂载的技能 · 中央仓库位于 ~/.agents/skills
      </p>

      {/* 搜索 */}
      <div className="relative mt-5">
        <Search size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
        <input
          className="w-full rounded-xl border border-black/10 bg-white py-2.5 pl-9 pr-3 text-[13px] outline-none focus:border-blue-400 focus:ring-2 focus:ring-blue-100"
          placeholder="搜索全部技能（名称 / 描述 / 目录）…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>
      {query.trim() && (
        <div className="mt-3 rounded-xl border border-black/8 bg-white p-3">
          {results?.length === 0 && (
            <div className="py-4 text-center text-[13px] text-gray-400">没有匹配的技能</div>
          )}
          {results?.slice(0, 10).map((s) => (
            <div key={`${s.agentId}/${s.folder}`} className="border-b border-black/5 py-2 last:border-0">
              <Link
                to={`/agent/${s.agentId}`}
                className="text-[13px] font-medium text-blue-700 hover:underline"
              >
                {s.name}
              </Link>
              <span className="ml-2 text-[11px] text-gray-400">
                {agents?.find((a) => a.id === s.agentId)?.name}
              </span>
              <div className="truncate text-[12px] text-gray-500">{s.description}</div>
            </div>
          ))}
          {results && results.length > 10 && (
            <div className="pt-2 text-[11px] text-gray-400">共 {results.length} 条，仅显示前 10 条</div>
          )}
        </div>
      )}

      {/* 统计卡片 */}
      <div className="mt-6 grid grid-cols-3 gap-3">
        <StatCard icon={<Grid3X3 size={15} />} label="Agent 数" value={`${standard.length + informational.length}`} sub={`${standard.length} 个可管理 · ${informational.length} 个只读展示`} />
        <StatCard icon={<Database size={15} />} label="中央仓库技能" value={`${central?.skillCount ?? 0}`} sub={central?.exists ? "正常可用" : "未找到"} />
        <StatCard icon={<Link2 size={15} />} label="技能总数" value={`${totalSkills}`} sub={`${standard.reduce((n, a) => n + a.symlinkCount, 0)} 个符号链接 · ${standard.reduce((n, a) => n + a.realCount, 0)} 个本地目录`} />
      </div>

      {/* Agent 卡片 */}
      <h2 className="mt-8 text-[14px] font-semibold">Agents</h2>
      <div className="mt-3 grid grid-cols-2 gap-3">
        {isLoading && <div className="col-span-2 py-6 text-center text-sm text-gray-400">加载中…</div>}
        {standard.map((a) => (
          <Link
            key={a.id}
            to={`/agent/${a.id}`}
            className="group rounded-xl border border-black/8 bg-white p-4 transition-shadow hover:shadow-md"
          >
            <div className="flex items-center justify-between">
              <span className="text-[14px] font-medium group-hover:text-blue-700">{a.name}</span>
              <span className="text-[20px] font-semibold tabular-nums text-gray-800">
                {a.skillCount}
              </span>
            </div>
            <div className="mt-1.5 flex items-center gap-3 text-[11px] text-gray-500">
              <span className="inline-flex items-center gap-1">
                <Link2 size={11} className="text-emerald-500" />
                {a.symlinkCount} 链接
              </span>
              <span className="inline-flex items-center gap-1">
                <Folder size={11} className="text-blue-500" />
                {a.realCount} 本地
              </span>
              {!a.exists && <span className="text-orange-500">目录不存在</span>}
            </div>
            <div className="mt-2 truncate text-[11px] text-gray-400" title={a.note}>
              {a.note}
            </div>
          </Link>
        ))}
      </div>

      {informational.length > 0 && (
        <>
          <h2 className="mt-6 text-[14px] font-semibold">只读展示</h2>
          <div className="mt-3 grid grid-cols-2 gap-3">
            {informational.map((a) => (
              <div key={a.id} className="rounded-xl border border-dashed border-black/10 bg-gray-50 p-4">
                <div className="text-[14px] font-medium text-gray-600">{a.name}</div>
                <div className="mt-1.5 text-[11px] leading-relaxed text-gray-500">{a.note}</div>
              </div>
            ))}
          </div>
        </>
      )}

      <div className="mt-8 text-center text-[11px] text-gray-400">
        数据来自实时扫描 · 更新于 {formatTime(new Date().toISOString())}
      </div>
    </div>
  );
}

function StatCard({
  icon,
  label,
  value,
  sub,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  sub: string;
}) {
  return (
    <div className="rounded-xl border border-black/8 bg-white p-4">
      <div className="flex items-center gap-1.5 text-[11px] uppercase tracking-wide text-gray-400">
        {icon} {label}
      </div>
      <div className="mt-1 text-[24px] font-semibold tabular-nums">{value}</div>
      <div className="text-[11px] text-gray-400">{sub}</div>
    </div>
  );
}
