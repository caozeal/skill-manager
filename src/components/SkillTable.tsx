import { ExternalLink, FolderOpen, Unplug } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { Skill } from "@/types";
import { useUnmount } from "@/hooks";
import { formatSize, formatTime } from "@/lib/utils";
import { MountTypeBadge, NotFoundBadge } from "@/components/badges/MountTypeBadge";
import { useDrawerStore } from "@/stores/ui";

export function SkillTable({ skills, canUnmount }: { skills: Skill[]; canUnmount: boolean }) {
  const openDrawer = useDrawerStore((s) => s.open);
  const unmount = useUnmount();
  const handleUnmount = (skill: Skill) => {
    if (
      window.confirm(
        `确定卸载「${skill.name}」吗？\n\n将删除符号链接：\n${skill.path}\n\n（仅删除链接，不影响技能本体）`,
      )
    ) {
      unmount.mutate({ agentId: skill.agentId, folder: skill.folder });
    }
  };

  return (
    <div className="overflow-x-auto rounded-xl border border-black/8 bg-white">
      <table className="w-full min-w-[860px] text-[13px]">
        <thead>
          <tr className="border-b border-black/8 bg-gray-50/60 text-left text-[11px] uppercase tracking-wide text-gray-500">
            <th className="px-4 py-2.5 font-medium">类型</th>
            <th className="px-3 py-2.5 font-medium">名称</th>
            <th className="px-3 py-2.5 font-medium">描述</th>
            <th className="px-3 py-2.5 font-medium">来源</th>
            <th className="px-3 py-2.5 font-medium">更新时间</th>
            <th className="px-3 py-2.5 text-right font-medium">大小</th>
            <th className="px-4 py-2.5 text-right font-medium">操作</th>
          </tr>
        </thead>
        <tbody>
          {skills.map((s) => (
            <tr
              key={s.folder}
              className="cursor-pointer border-b border-black/5 last:border-0 hover:bg-blue-50/40"
              onClick={() => openDrawer(s)}
            >
              <td className="px-4 py-2.5">
                <MountTypeBadge skill={s} />
              </td>
              <td className="max-w-[180px] px-3 py-2.5">
                <div className="truncate font-medium text-gray-900" title={s.name}>
                  {s.name}
                </div>
                <div className="truncate text-[11px] text-gray-400" title={s.folder}>
                  {s.folder}
                </div>
              </td>
              <td className="max-w-[420px] px-3 py-2.5">
                {!s.skillMdFound && <NotFoundBadge />}
                <div className="truncate text-gray-500" title={s.description}>
                  {s.description || "—"}
                </div>
              </td>
              <td className="max-w-[150px] px-3 py-2.5">
                {s.source ? (
                  <span className="text-gray-500" title={s.source.sourceUrl}>
                    {s.source.source || s.source.sourceType}
                  </span>
                ) : (
                  <span className="text-gray-400">本地</span>
                )}
              </td>
              <td className="whitespace-nowrap px-3 py-2.5 text-gray-500">
                {formatTime(s.source?.updatedAt ?? s.modifiedAt)}
              </td>
              <td className="whitespace-nowrap px-3 py-2.5 text-right tabular-nums text-gray-500">
                {formatSize(s.dirSize)}
                <span className="ml-1 text-[11px] text-gray-400">({s.fileCount} 文件)</span>
              </td>
              <td className="whitespace-nowrap px-4 py-2.5 text-right" onClick={(e) => e.stopPropagation()}>
                <button
                  className="rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-gray-600"
                  title="在 Finder 中显示"
                  onClick={() => revealItemInDir(s.path)}
                >
                  <FolderOpen size={14} />
                </button>
                {s.source?.sourceUrl && (
                  <a
                    className="ml-0.5 inline-block rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-gray-600"
                    title={s.source.sourceUrl}
                    href={s.source.sourceUrl}
                    target="_blank"
                    rel="noreferrer"
                  >
                    <ExternalLink size={14} />
                  </a>
                )}
                {canUnmount &&
                  !s.isSystem &&
                  !s.isGlobal &&
                  !s.isCompat &&
                  (s.mount.mountType === "symlink" || s.mount.mountType === "broken_symlink") && (
                    <button
                      className="ml-0.5 rounded p-1.5 text-gray-400 hover:bg-red-50 hover:text-red-500"
                      title="卸载（删除符号链接）"
                      disabled={unmount.isPending}
                      onClick={() => handleUnmount(s)}
                    >
                      <Unplug size={14} />
                    </button>
                  )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {skills.length === 0 && (
        <div className="py-14 text-center text-sm text-gray-400">暂无技能</div>
      )}
    </div>
  );
}
