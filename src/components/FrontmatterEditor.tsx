import { useEffect, useState } from "react";
import { Save } from "lucide-react";
import { useSkillDetail, useUpdateFrontmatter } from "@/hooks";
import { AppError } from "@/lib/tauri";

export function FrontmatterEditor({
  agentId,
  folder,
}: {
  agentId: string;
  folder: string;
}) {
  const { data } = useSkillDetail(agentId, folder);
  const mutation = useUpdateFrontmatter();
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [saved, setSaved] = useState<string | null>(null);

  useEffect(() => {
    if (data) {
      setName(data.skill.name);
      setDesc(data.skill.description);
    }
  }, [data?.skill.name, data?.skill.description]);

  if (!data) return null;

  const save = () => {
    mutation.mutate(
      { agentId, folder, update: { name, description: desc } },
      {
        onSuccess: () => setSaved(`已保存（备份: SKILL.md.bak-*）`),
        onError: (e) => setSaved(e instanceof AppError ? `保存失败：${e.message}` : "保存失败"),
      },
    );
  };

  const dirty = name !== data.skill.name || desc !== data.skill.description;

  return (
    <div className="space-y-4">
      <div>
        <label className="mb-1 block text-[12px] font-medium text-gray-600">名称 (name)</label>
        <input
          className="w-full rounded-lg border border-black/10 px-3 py-2 text-[13px] outline-none focus:border-blue-400 focus:ring-2 focus:ring-blue-100"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
      </div>
      <div>
        <label className="mb-1 block text-[12px] font-medium text-gray-600">
          描述 (description)
        </label>
        <textarea
          className="min-h-[120px] w-full resize-y rounded-lg border border-black/10 px-3 py-2 text-[13px] leading-relaxed outline-none focus:border-blue-400 focus:ring-2 focus:ring-blue-100"
          value={desc}
          onChange={(e) => setDesc(e.target.value)}
        />
        <div className="mt-1 text-[11px] text-gray-400">
          description 是 agent 决定何时触发技能的关键，建议保留触发条件描述
        </div>
      </div>
      <div className="flex items-center gap-3">
        <button
          className="inline-flex items-center gap-1.5 rounded-lg bg-blue-600 px-3.5 py-1.5 text-[13px] font-medium text-white hover:bg-blue-700 disabled:opacity-40"
          disabled={!dirty || mutation.isPending}
          onClick={save}
        >
          <Save size={13} /> 保存
        </button>
        {saved && <span className="text-[12px] text-gray-500">{saved}</span>}
        {mutation.isError && !saved && (
          <span className="text-[12px] text-red-500">保存失败</span>
        )}
      </div>
    </div>
  );
}
