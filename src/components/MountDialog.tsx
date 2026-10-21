import { useMemo, useState } from "react";
import { PackagePlus, X } from "lucide-react";

interface Candidate {
  folder: string;
  name: string;
}

export function MountDialog({
  agentId,
  agentName,
  skillsPath,
  candidates,
  onMount,
  pending,
  error,
}: {
  agentId: string;
  agentName: string;
  skillsPath: string | null;
  candidates: Candidate[];
  onMount: (folder: string) => void;
  pending: boolean;
  error: string | null;
}) {
  const [open, setOpen] = useState(false);
  const [q, setQ] = useState("");
  const [confirmFolder, setConfirmFolder] = useState<Candidate | null>(null);

  const filtered = useMemo(() => {
    const lower = q.trim().toLowerCase();
    return lower
      ? candidates.filter(
          (c) => c.folder.toLowerCase().includes(lower) || c.name.toLowerCase().includes(lower),
        )
      : candidates;
  }, [candidates, q]);

  return (
    <>
      <button
        className="inline-flex items-center gap-1.5 rounded-lg bg-blue-600 px-3.5 py-2 text-[13px] font-medium text-white hover:bg-blue-700"
        onClick={() => setOpen(true)}
      >
        <PackagePlus size={14} /> 挂载技能
      </button>

      {open && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30" onClick={() => setOpen(false)}>
          <div
            className="flex max-h-[70vh] w-[440px] flex-col rounded-2xl bg-white p-4 shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between">
              <h3 className="text-[15px] font-semibold">
                从中央仓库挂载到 {agentName}
              </h3>
              <button className="rounded p-1 text-gray-400 hover:bg-gray-100" onClick={() => setOpen(false)}>
                <X size={15} />
              </button>
            </div>
            <input
              autoFocus
              className="mt-3 w-full rounded-lg border border-black/10 px-3 py-2 text-[13px] outline-none focus:border-blue-400"
              placeholder="筛选技能…"
              value={q}
              onChange={(e) => setQ(e.target.value)}
            />
            {error && <div className="mt-2 rounded bg-red-50 px-3 py-1.5 text-[12px] text-red-600">{error}</div>}
            <div className="mt-2 flex-1 overflow-y-auto">
              {filtered.length === 0 && (
                <div className="py-8 text-center text-[13px] text-gray-400">没有可挂载的技能</div>
              )}
              {filtered.map((c) => (
                <button
                  key={c.folder}
                  disabled={pending}
                  className="flex w-full items-center justify-between rounded-lg px-3 py-2 text-left hover:bg-blue-50 disabled:opacity-50"
                  onClick={() => setConfirmFolder(c)}
                >
                  <div className="min-w-0">
                    <div className="truncate text-[13px] font-medium">{c.name}</div>
                    <div className="truncate text-[11px] text-gray-400">{c.folder}</div>
                  </div>
                  <span className="text-[11px] text-blue-600">挂载 →</span>
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      {confirmFolder && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30">
          <div className="w-[380px] rounded-2xl bg-white p-5 shadow-xl">
            <h3 className="text-[15px] font-semibold">确认挂载</h3>
            <div className="mt-3 rounded-lg bg-gray-50 p-3 text-[12px] leading-relaxed text-gray-600">
              将在 <span className="font-mono">{skillsPath ?? `~/.${agentId}/skills`}</span> 下创建符号链接
              <br />
              <span className="font-mono">{confirmFolder.folder}</span> →{" "}
              <span className="font-mono">~/.agents/skills/{confirmFolder.folder}</span>
            </div>
            <div className="mt-4 flex justify-end gap-2">
              <button
                className="rounded-lg border border-black/10 px-3.5 py-1.5 text-[13px] text-gray-600 hover:bg-gray-50"
                onClick={() => setConfirmFolder(null)}
              >
                取消
              </button>
              <button
                className="rounded-lg bg-blue-600 px-3.5 py-1.5 text-[13px] font-medium text-white hover:bg-blue-700 disabled:opacity-50"
                disabled={pending}
                onClick={() => {
                  onMount(confirmFolder.folder);
                  setConfirmFolder(null);
                }}
              >
                挂载
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
