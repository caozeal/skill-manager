import { useMemo, useState } from "react";
import { ExternalLink, FolderOpen, Unplug, Pause, Play, Trash2, Search, ArrowDownUp } from "lucide-react";
import type { Skill, UsageStat } from "@/types";
import { useUnmount, useDisable, useEnable, useDelete } from "@/hooks";
import { formatSize, formatTime, cn, revealSkillInFinder } from "@/lib/utils";
import { MountTypeBadge, NotFoundBadge } from "@/components/badges/MountTypeBadge";
import { useDrawerStore, useConfirmStore, useToastStore } from "@/stores/ui";

/** skills/<folder> → skills 同级的 disabled-skills/<folder> */
function disabledDest(path: string): string {
  const marker = "/skills/";
  const i = path.lastIndexOf(marker);
  if (i < 0) return path;
  return path.slice(0, i) + "/disabled-skills/" + path.slice(i + marker.length);
}

/** 仅显示日期（YYYY-MM-DD），完整时间放 title */
function shortDate(iso: string | null): string {
  if (!iso) return "—";
  return iso.slice(0, 10);
}

export function SkillTable({
  skills,
  canUnmount,
  usage,
}: {
  skills: Skill[];
  canUnmount: boolean;
  /** folder -> 使用统计，未提供时不显示「使用」列 */
  usage?: Record<string, UsageStat>;
}) {
  const openDrawer = useDrawerStore((s) => s.open);
  const confirm = useConfirmStore((s) => s.confirm);
  const toast = useToastStore((s) => s.push);
  const unmount = useUnmount();
  const disable = useDisable();
  const enable = useEnable();
  const del = useDelete();

  /** mutation 失败必须有可见反馈，否则看起来就是「点击不生效」 */
  const onError = (action: string) => (e: unknown) =>
    toast(`${action}失败：${e instanceof Error ? e.message : String(e)}`);

  // 搜索 + 排序（有使用数据时默认按使用次数降序；用户手动切换后以手动为准）
  const [query, setQuery] = useState("");
  const [manualSort, setManualSort] = useState<"usage" | "name" | null>(null);
  const sortBy = manualSort ?? (usage ? "usage" : "name");

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    let list = skills;
    if (q) {
      list = list.filter(
        (s) =>
          s.name.toLowerCase().includes(q) ||
          s.folder.toLowerCase().includes(q) ||
          (s.description ?? "").toLowerCase().includes(q),
      );
    }
    const byName = (a: Skill, b: Skill) =>
      a.name.localeCompare(b.name, "zh-Hans-CN");
    if (sortBy === "usage" && usage) {
      list = [...list].sort((a, b) => {
        const ca = usage[a.folder]?.count ?? 0;
        const cb = usage[b.folder]?.count ?? 0;
        return cb - ca || byName(a, b);
      });
    } else {
      list = [...list].sort(byName);
    }
    return list;
  }, [skills, usage, query, sortBy]);

  const isManaged = (s: Skill) => canUnmount && !s.isSystem && !s.isGlobal && !s.isCompat;

  const handleUnmount = async (s: Skill) => {
    const ok = await confirm({
      title: `卸载「${s.name}」`,
      message: `将删除符号链接：\n${s.path}\n\n仅删除链接，不影响技能本体。`,
      confirmText: "卸载",
      danger: true,
    });
    if (ok) unmount.mutate({ agentId: s.agentId, folder: s.folder }, { onError: onError("卸载") });
  };
  const handleDisable = async (s: Skill) => {
    const ok = await confirm({
      title: `禁用「${s.name}」`,
      message: `将把目录移到 skills 目录之外：\n${disabledDest(s.path)}\n\n彻底脱离 agent 的扫描范围，可随时启用（不会删除任何文件）。`,
      confirmText: "禁用",
      danger: true,
    });
    if (ok) disable.mutate({ agentId: s.agentId, folder: s.folder }, { onError: onError("禁用") });
  };
  const handleDelete = async (s: Skill) => {
    const ok = await confirm({
      title: `删除「${s.name}」`,
      message: `⚠️ 将移动整个目录到回收站：\n${s.path}\n→ ~/.skill-manager-trash/\n\n可在回收站中手动恢复，但该 agent 将立即失去此技能。`,
      confirmText: "删除",
      danger: true,
    });
    if (ok) del.mutate({ agentId: s.agentId, folder: s.folder }, { onError: onError("删除") });
  };

  return (
    <div className="overflow-x-auto rounded-xl border border-black/8 bg-white">
      <div className="flex items-center gap-2 border-b border-black/8 px-3 py-2">
        <div className="relative">
          <Search
            size={13}
            className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-gray-400"
          />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索技能名称 / 描述…"
            className="w-56 rounded-md border border-black/10 bg-white py-1 pl-7 pr-2 text-[12px] text-gray-700 placeholder:text-gray-300 focus:border-blue-300 focus:outline-none"
          />
        </div>
        <button
          className="inline-flex items-center gap-1 rounded-md border border-black/10 px-2 py-1 text-[11px] text-gray-600 hover:bg-gray-50"
          title="切换排序方式"
          onClick={() => setManualSort(sortBy === "usage" ? "name" : "usage")}
        >
          <ArrowDownUp size={12} />
          {sortBy === "usage" ? "按使用次数" : "按名称"}
        </button>
        {query && (
          <span className="text-[11px] text-gray-400">
            {visible.length}/{skills.length} 个匹配
          </span>
        )}
      </div>
      <table className="w-full min-w-[860px] text-[13px]">
        <thead>
          <tr className="border-b border-black/8 bg-gray-50/60 text-left text-[11px] uppercase tracking-wide text-gray-500">
            <th className="px-4 py-2.5 font-medium">类型</th>
            <th className="px-3 py-2.5 font-medium">名称</th>
            <th className="px-3 py-2.5 font-medium">描述</th>
            <th className="px-3 py-2.5 font-medium">来源</th>
            <th className="px-3 py-2.5 font-medium">更新时间</th>
            <th className="px-3 py-2.5 text-right font-medium">大小</th>
            {usage && (
              <th className="px-3 py-2.5 text-right font-medium" title="该 agent 会话日志中的技能调用次数">
                使用
              </th>
            )}
            <th className="px-4 py-2.5 text-right font-medium">操作</th>
          </tr>
        </thead>
        <tbody>
          {visible.map((s) => (
            <tr
              key={s.path}
              className={cn(
                "cursor-pointer border-b border-black/5 last:border-0 hover:bg-blue-50/40",
                s.disabled && "opacity-45",
              )}
              onClick={() => openDrawer(s)}
            >
              <td className="px-4 py-2.5">
                <MountTypeBadge skill={s} />
                {s.disabled && (
                  <span className="ml-1 rounded bg-gray-100 px-1.5 py-0.5 text-[11px] text-gray-500">
                    已禁用
                  </span>
                )}
              </td>
              <td className="max-w-[180px] px-3 py-2.5">
                <div className="truncate font-medium text-gray-900" title={s.name}>
                  {s.name}
                </div>
                <div className="truncate text-[11px] text-gray-400" title={s.folder}>
                  {s.folder}
                </div>
              </td>
              <td className="max-w-[240px] px-3 py-2.5">
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
              <td
                className="whitespace-nowrap px-3 py-2.5 text-gray-500"
                title={formatTime(s.source?.updatedAt ?? s.modifiedAt)}
              >
                {shortDate(s.source?.updatedAt ?? s.modifiedAt)}
              </td>
              <td className="whitespace-nowrap px-3 py-2.5 text-right tabular-nums text-gray-500">
                {formatSize(s.dirSize)}
                <span className="ml-1 text-[11px] text-gray-400">({s.fileCount} 文件)</span>
              </td>
              {usage && (
                <td className="whitespace-nowrap px-3 py-2.5 text-right tabular-nums">
                  {usage[s.folder]?.count ? (
                    <span
                      className="text-gray-700"
                      title={
                        usage[s.folder].lastUsed
                          ? `最近使用：${usage[s.folder].lastUsed!.replace("T", " ").replace("Z", "")}`
                          : "无精确时间（近似统计）"
                      }
                    >
                      {usage[s.folder].count}
                    </span>
                  ) : (
                    <span className="text-gray-300">—</span>
                  )}
                </td>
              )}
              <td className="whitespace-nowrap px-4 py-2.5 text-right" onClick={(e) => e.stopPropagation()}>
                <button
                  className="rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-gray-600"
                  title="在 Finder 中显示"
                  onClick={() => revealSkillInFinder(s.path, s.mount.mountType)}
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
                {isManaged(s) && s.disabled && (
                  <button
                    className="ml-0.5 rounded p-1.5 text-gray-400 hover:bg-emerald-50 hover:text-emerald-600"
                    title="启用（移回 skills 目录）"
                    disabled={enable.isPending}
                    onClick={() =>
                      enable.mutate(
                        { agentId: s.agentId, folder: s.folder },
                        { onError: onError("启用") },
                      )
                    }
                  >
                    <Play size={14} />
                  </button>
                )}
                {isManaged(s) && !s.disabled && (
                  <>
                    <button
                      className="ml-0.5 rounded p-1.5 text-gray-400 hover:bg-amber-50 hover:text-amber-600"
                      title="禁用（agent 将不再加载此技能，可恢复）"
                      disabled={disable.isPending}
                      onClick={() => handleDisable(s)}
                    >
                      <Pause size={14} />
                    </button>
                    {s.mount.mountType === "real_dir" && (
                      <button
                        className="ml-0.5 rounded p-1.5 text-gray-400 hover:bg-red-50 hover:text-red-500"
                        title="删除（移动到 ~/.skill-manager-trash）"
                        disabled={del.isPending}
                        onClick={() => handleDelete(s)}
                      >
                        <Trash2 size={14} />
                      </button>
                    )}
                    {(s.mount.mountType === "symlink" || s.mount.mountType === "broken_symlink") && (
                      <button
                        className="ml-0.5 rounded p-1.5 text-gray-400 hover:bg-red-50 hover:text-red-500"
                        title="卸载（删除符号链接）"
                        disabled={unmount.isPending}
                        onClick={() => handleUnmount(s)}
                      >
                        <Unplug size={14} />
                      </button>
                    )}
                  </>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {skills.length === 0 && (
        <div className="py-14 text-center text-sm text-gray-400">暂无技能</div>
      )}
      {skills.length > 0 && visible.length === 0 && (
        <div className="py-14 text-center text-sm text-gray-400">
          没有匹配「{query}」的技能
        </div>
      )}
    </div>
  );
}
