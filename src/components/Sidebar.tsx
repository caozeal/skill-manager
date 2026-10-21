import { NavLink, useLocation } from "react-router-dom";
import {
  LayoutDashboard,
  Grid3X3,
  BarChart3,
  Terminal,
  Code2,
  Bot,
  Moon,
  Zap,
  Bird,
  PawPrint,
  Database,
  Waves,
  Blocks,
  type LucideIcon,
} from "lucide-react";
import { useAgents } from "@/hooks";
import { cn } from "@/lib/utils";

const iconMap: Record<string, LucideIcon> = {
  terminal: Terminal,
  code: Code2,
  bot: Bot,
  moon: Moon,
  zap: Zap,
  bird: Bird,
  "paw-print": PawPrint,
  database: Database,
  waves: Waves,
  blocks: Blocks,
};

export function Sidebar() {
  const { data: agents, isLoading } = useAgents();
  const location = useLocation();

  return (
    <aside className="flex w-60 shrink-0 flex-col border-r border-black/8 bg-white">
      <div className="flex items-center gap-2 px-4 py-4">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-600 text-white">
          <Grid3X3 size={16} />
        </div>
        <div>
          <div className="text-sm font-semibold leading-tight">Skill Manager</div>
          <div className="text-[11px] text-gray-500">Agent 技能管理</div>
        </div>
      </div>

      <nav className="flex flex-col gap-0.5 px-2">
        <NavItem to="/" icon={LayoutDashboard} label="总览" active={location.pathname === "/"} />
        <NavItem
          to="/matrix"
          icon={Grid3X3}
          label="挂载矩阵"
          active={location.pathname === "/matrix"}
        />
        <NavItem
          to="/usage"
          icon={BarChart3}
          label="使用统计"
          active={location.pathname === "/usage"}
        />
      </nav>

      <div className="mt-4 px-4 text-[11px] font-medium uppercase tracking-wide text-gray-400">
        Agents
      </div>
      <nav className="mt-1 flex flex-1 flex-col gap-0.5 overflow-y-auto px-2 pb-3">
        {isLoading && <div className="px-2 py-2 text-xs text-gray-400">加载中…</div>}
        {agents?.map((a) => {
          const Icon = iconMap[a.icon] ?? Bot;
          return (
            <NavLink
              key={a.id}
              to={`/agent/${a.id}`}
              className={({ isActive }) =>
                cn(
                  "flex items-center gap-2 rounded-md px-2 py-1.5 text-[13px] transition-colors",
                  isActive
                    ? "bg-blue-50 font-medium text-blue-700"
                    : "text-gray-700 hover:bg-gray-100",
                )
              }
            >
              <Icon size={15} className="shrink-0 opacity-70" />
              <span className="flex-1 truncate">{a.name}</span>
              {a.kind === "informational" ? (
                <span className="rounded bg-gray-100 px-1 text-[10px] text-gray-400">只读</span>
              ) : (
                <span
                  className={cn(
                    "rounded px-1.5 text-[10px] tabular-nums",
                    a.skillCount > 0 ? "bg-blue-50 text-blue-600" : "bg-gray-100 text-gray-400",
                  )}
                >
                  {a.skillCount}
                </span>
              )}
            </NavLink>
          );
        })}
      </nav>
    </aside>
  );
}

function NavItem({
  to,
  icon: Icon,
  label,
  active,
}: {
  to: string;
  icon: LucideIcon;
  label: string;
  active: boolean;
}) {
  return (
    <NavLink
      to={to}
      className={cn(
        "flex items-center gap-2 rounded-md px-2 py-1.5 text-[13px] transition-colors",
        active ? "bg-blue-50 font-medium text-blue-700" : "text-gray-700 hover:bg-gray-100",
      )}
    >
      <Icon size={15} className="shrink-0 opacity-70" />
      {label}
    </NavLink>
  );
}
