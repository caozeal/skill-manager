import { useState } from "react";
import { X, BookOpen, FolderTree, Pencil, FolderOpen } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useSkillDetail } from "@/hooks";
import { useDrawerStore } from "@/stores/ui";
import { formatSize, formatTime, cn } from "@/lib/utils";
import { MarkdownView } from "@/components/MarkdownView";
import { FileTreeRoot } from "@/components/FileTree";
import { FrontmatterEditor } from "@/components/FrontmatterEditor";
import { MountTypeBadge } from "@/components/badges/MountTypeBadge";

const tabs = [
  { id: "preview", label: "预览", icon: BookOpen },
  { id: "files", label: "文件树", icon: FolderTree },
  { id: "edit", label: "编辑", icon: Pencil },
] as const;

export function SkillDetailDrawer() {
  const { skill, close } = useDrawerStore();
  const [tab, setTab] = useState<(typeof tabs)[number]["id"]>("preview");
  const { data, isLoading } = useSkillDetail(
    skill?.agentId,
    skill?.folder,
  );

  if (!skill) return null;
  const editable =
    !skill.isSystem &&
    !skill.isGlobal &&
    !skill.isCompat &&
    skill.agentId !== "dsh" &&
    skill.agentId !== "opencode";

  return (
    <div className="flex w-[480px] shrink-0 flex-col border-l border-black/8 bg-white">
      <div className="flex items-start gap-2 border-b border-black/8 px-4 py-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h2 className="truncate text-[15px] font-semibold">{skill.name}</h2>
            <MountTypeBadge skill={skill} />
          </div>
          <button
            className="mt-0.5 max-w-full truncate text-left text-[11px] text-gray-400 hover:text-blue-600"
            title={skill.path}
            onClick={() => revealItemInDir(skill.path)}
          >
            {skill.path}
          </button>
        </div>
        <button
          className="rounded p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-600"
          onClick={close}
        >
          <X size={16} />
        </button>
      </div>

      <div className="flex gap-1 border-b border-black/8 px-3 pt-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            disabled={t.id === "edit" && !editable}
            className={cn(
              "flex items-center gap-1.5 rounded-t-md px-3 py-1.5 text-[12px] transition-colors",
              tab === t.id
                ? "border-b-2 border-blue-600 font-medium text-blue-700"
                : "text-gray-500 hover:text-gray-700",
              t.id === "edit" && !editable && "cursor-not-allowed opacity-40",
            )}
            onClick={() => setTab(t.id)}
          >
            <t.icon size={13} /> {t.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {isLoading && <div className="py-10 text-center text-sm text-gray-400">加载中…</div>}
        {data && (
          <>
            {tab === "preview" && (
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-2 rounded-lg bg-gray-50 p-3 text-[12px]">
                  <Info label="目录" value={data.skill.folder} />
                  <Info label="大小" value={`${formatSize(data.skill.dirSize)} · ${data.skill.fileCount} 文件`} />
                  <Info
                    label="来源"
                    value={data.skill.source?.source ?? "本地"}
                  />
                  <Info
                    label="更新时间"
                    value={formatTime(data.skill.source?.updatedAt ?? data.skill.modifiedAt)}
                  />
                  {data.skill.mount.rawTarget && (
                    <Info className="col-span-2" label="链接目标" value={data.skill.mount.rawTarget} />
                  )}
                </div>
                <MarkdownView md={data.bodyMd} />
              </div>
            )}
            {tab === "files" && <FileTreeRoot node={data.fileTree} />}
            {tab === "edit" && (
              <FrontmatterEditor agentId={skill.agentId} folder={skill.folder} />
            )}
          </>
        )}
      </div>

      <div className="border-t border-black/8 px-4 py-2">
        <button
          className="inline-flex w-full items-center justify-center gap-1.5 rounded-lg border border-black/10 py-1.5 text-[12px] text-gray-600 hover:bg-gray-50"
          onClick={() => revealItemInDir(skill.path)}
        >
          <FolderOpen size={13} /> 在 Finder 中显示
        </button>
      </div>
    </div>
  );
}

function Info({
  label,
  value,
  className,
}: {
  label: string;
  value: string;
  className?: string;
}) {
  return (
    <div className={className}>
      <div className="text-[10px] uppercase tracking-wide text-gray-400">{label}</div>
      <div className="truncate text-gray-700" title={value}>
        {value || "—"}
      </div>
    </div>
  );
}
