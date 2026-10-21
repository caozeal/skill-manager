import { Link2, Folder, Link2Off, ShieldAlert, Lock, Globe2, Layers } from "lucide-react";
import type { Skill } from "@/types";
import { cn } from "@/lib/utils";

export function MountTypeBadge({ skill }: { skill: Skill }) {
  if (skill.isGlobal) {
    return (
      <span
        className="inline-flex items-center gap-1 rounded bg-violet-50 px-1.5 py-0.5 text-[11px] text-violet-600"
        title="全局技能：位于中央仓库 ~/.agents/skills，对当前 agent 直接生效"
      >
        <Globe2 size={10} /> 全局
      </span>
    );
  }
  if (skill.isCompat) {
    return (
      <span
        className="inline-flex items-center gap-1 rounded bg-cyan-50 px-1.5 py-0.5 text-[11px] text-cyan-600"
        title="Claude 兼容技能：来自 ~/.claude/skills（OpenCode 可发现此目录）"
      >
        <Layers size={10} /> Claude 兼容
      </span>
    );
  }
  if (skill.isSystem) {
    const label = skill.systemTier ?? "系统";
    return (
      <span
        className="inline-flex items-center gap-1 rounded bg-gray-100 px-1.5 py-0.5 text-[11px] text-gray-500"
        title={
          skill.systemTier
            ? `${skill.systemTier}技能：由 ${skill.agentId} 的 tier 目录（.system/.curated/.experimental）提供，只读`
            : "系统技能：由 agent 内置提供，只读"
        }
      >
        <Lock size={10} /> {label}
      </span>
    );
  }
  switch (skill.mount.mountType) {
    case "symlink":
      return (
        <span
          className={cn(
            "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px]",
            skill.mount.pointsToCentral
              ? "bg-emerald-50 text-emerald-600"
              : "bg-amber-50 text-amber-600",
          )}
          title={
            skill.mount.rawTarget
              ? `链接目标: ${skill.mount.rawTarget}`
              : undefined
          }
        >
          <Link2 size={10} />
          {skill.mount.pointsToCentral ? "符号链接" : "外部链接"}
        </span>
      );
    case "broken_symlink":
      return (
        <span
          className="inline-flex items-center gap-1 rounded bg-red-50 px-1.5 py-0.5 text-[11px] text-red-600"
          title={skill.mount.rawTarget ?? undefined}
        >
          <Link2Off size={10} /> 断链
        </span>
      );
    case "real_dir":
    default:
      return (
        <span className="inline-flex items-center gap-1 rounded bg-blue-50 px-1.5 py-0.5 text-[11px] text-blue-600">
          <Folder size={10} /> 本地目录
        </span>
      );
  }
}

export function NotFoundBadge() {
  return (
    <span className="inline-flex items-center gap-1 rounded bg-orange-50 px-1.5 py-0.5 text-[11px] text-orange-600">
      <ShieldAlert size={10} /> 缺 SKILL.md
    </span>
  );
}
