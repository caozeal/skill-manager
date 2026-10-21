import { useEffect, useState } from "react";
import {
  Plug,
  PlugZap,
  Pause,
  Play,
  Pencil,
  Trash2,
  Copy,
  Server,
  Globe2,
} from "lucide-react";
import type { McpServer, McpServerInput } from "@/types";
import { MCP_MASK } from "@/types";
import {
  useSetMcpEnabled,
  useUpsertMcp,
  useRemoveMcp,
  useSyncMcp,
  useMcpAgentMeta,
} from "@/hooks";
import { useConfirmStore, useToastStore } from "@/stores/ui";
import { cn } from "@/lib/utils";

export function mcpEndpoint(s: McpServer): string {
  if (s.url) return s.url;
  if (s.command) return [s.command, ...s.args].join(" ");
  return "—";
}

/**
 * MCP 列表表格。MCP 页用它显示全量，agent 页用它显示单个 agent 的 MCP。
 */
export function McpTable({
  servers,
  showAgent = true,
  conflicts = [],
  isLoading = false,
  emptyText = "没有找到 MCP 配置",
}: {
  servers: McpServer[];
  showAgent?: boolean;
  conflicts?: string[];
  isLoading?: boolean;
  emptyText?: string;
}) {
  const confirm = useConfirmStore((s) => s.confirm);
  const toast = useToastStore((s) => s.push);
  const setEnabled = useSetMcpEnabled();
  const remove = useRemoveMcp();
  const sync = useSyncMcp();
  const { nameOf } = useMcpAgentMeta();

  const [editing, setEditing] = useState<McpServer | "new" | null>(null);
  const [syncing, setSyncing] = useState<McpServer | null>(null);

  const onError = (action: string) => (e: unknown) =>
    toast(`${action}失败：${e instanceof Error ? e.message : String(e)}`);

  const handleToggle = async (s: McpServer) => {
    if (!s.writable) {
      toast("该配置源只读，不支持启用/禁用");
      return;
    }
    if (s.enabled) {
      const ok = await confirm({
        title: `禁用「${s.name}」`,
        message: s.nativeEnabled
          ? `将在 ${nameOf(s.agentId)} 的配置中标记为 enabled = false。`
          : `该 agent 没有 enabled 字段，将从配置中移除此条目。\n原始配置会留存到 ~/.skill-manager/mcp-disabled.json，可随时恢复。`,
        confirmText: "禁用",
        danger: true,
      });
      if (ok)
        setEnabled.mutate({ id: s.id, enabled: false }, { onError: onError("禁用") });
    } else {
      setEnabled.mutate({ id: s.id, enabled: true }, { onError: onError("启用") });
    }
  };

  const handleRemove = async (s: McpServer) => {
    const ok = await confirm({
      title: `删除「${s.name}」`,
      message: `将从 ${s.sourcePath} 中移除该 MCP 配置（写前会自动备份 .bak-时间戳）。`,
      confirmText: "删除",
      danger: true,
    });
    if (ok) remove.mutate(s.id, { onError: onError("删除") });
  };

  const doSync = (targets: string[]) => {
    const s = syncing;
    if (!s || !targets.length) return;
    sync.mutate(
      { id: s.id, targets },
      {
        onSuccess: (n) => toast(`已同步到 ${n} 个 agent（改完需重启对应 agent）`, "info"),
        onError: onError("同步"),
      },
    );
    setSyncing(null);
  };

  return (
    <>
      <div className="overflow-x-auto rounded-xl border border-black/8 bg-white">
        <table className="w-full min-w-[700px] text-[13px]">
          <thead>
            <tr className="border-b border-black/8 bg-gray-50/60 text-left text-[11px] uppercase tracking-wide text-gray-500">
              <th className="px-4 py-2.5 font-medium">名称</th>
              {showAgent && <th className="px-3 py-2.5 font-medium">来源</th>}
              <th className="px-3 py-2.5 font-medium">类型</th>
              <th className="px-3 py-2.5 font-medium">端点 / 命令</th>
              <th className="px-3 py-2.5 font-medium">状态</th>
              <th className="px-4 py-2.5 text-right font-medium">操作</th>
            </tr>
          </thead>
          <tbody>
            {servers.map((s) => {
              const conflict = conflicts.includes(s.name);
              return (
                <tr
                  key={s.id}
                  className={cn(
                    "border-b border-black/5 last:border-0 hover:bg-blue-50/40",
                    !s.enabled && "opacity-50",
                  )}
                >
                  <td className="px-4 py-2.5">
                    <div className="flex items-center gap-1.5">
                      <span className="font-medium text-gray-900">{s.name}</span>
                      {conflict && (
                        <span
                          className="rounded bg-amber-50 px-1 text-[10px] text-amber-700"
                          title="同名 MCP 在不同 agent 中配置不一致"
                        >
                          不一致
                        </span>
                      )}
                      {s.scope === "project" && (
                        <span
                          className="rounded bg-gray-100 px-1 text-[10px] text-gray-500"
                          title={s.project ?? ""}
                        >
                          项目级
                        </span>
                      )}
                    </div>
                    {(s.env.length > 0 || s.headers.length > 0) && (
                      <div className="mt-0.5 text-[11px] text-gray-400">
                        {s.env.length > 0 && `${s.env.length} 个环境变量`}
                        {s.env.length > 0 && s.headers.length > 0 && " · "}
                        {s.headers.length > 0 && `${s.headers.length} 个请求头`}
                      </div>
                    )}
                  </td>
                  {showAgent && (
                    <td className="whitespace-nowrap px-3 py-2.5 text-gray-600">
                      {nameOf(s.agentId)}
                    </td>
                  )}
                  <td className="whitespace-nowrap px-3 py-2.5">
                    <span
                      className={cn(
                        "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px]",
                        s.transport === "http"
                          ? "bg-emerald-50 text-emerald-600"
                          : s.transport === "stdio"
                            ? "bg-blue-50 text-blue-600"
                            : "bg-gray-100 text-gray-500",
                      )}
                    >
                      {s.transport === "http" ? <Globe2 size={10} /> : <Server size={10} />}
                      {s.transport}
                    </span>
                  </td>
                  <td className="max-w-[320px] px-3 py-2.5">
                    <div
                      className="truncate font-mono text-[11.5px] text-gray-500"
                      title={mcpEndpoint(s)}
                    >
                      {mcpEndpoint(s)}
                    </div>
                  </td>
                  <td className="whitespace-nowrap px-3 py-2.5">
                    {s.enabled ? (
                      <span className="inline-flex items-center gap-1 text-[11.5px] text-emerald-600">
                        <PlugZap size={11} /> 启用
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 text-[11.5px] text-gray-400">
                        <Plug size={11} /> 已禁用
                      </span>
                    )}
                  </td>
                  <td className="whitespace-nowrap px-4 py-2.5 text-right">
                    <button
                      className="rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-gray-600 disabled:opacity-30"
                      title={s.enabled ? "禁用" : "启用"}
                      onClick={() => handleToggle(s)}
                      disabled={!s.writable}
                    >
                      {s.enabled ? <Pause size={14} /> : <Play size={14} />}
                    </button>
                    <button
                      className="ml-0.5 rounded p-1.5 text-gray-400 hover:bg-blue-50 hover:text-blue-600 disabled:opacity-30"
                      title="编辑"
                      onClick={() => setEditing(s)}
                      disabled={!s.writable}
                    >
                      <Pencil size={14} />
                    </button>
                    <button
                      className="ml-0.5 rounded p-1.5 text-gray-400 hover:bg-violet-50 hover:text-violet-600"
                      title="同步到其它 agent"
                      onClick={() => setSyncing(s)}
                    >
                      <Copy size={14} />
                    </button>
                    <button
                      className="ml-0.5 rounded p-1.5 text-gray-400 hover:bg-red-50 hover:text-red-500 disabled:opacity-30"
                      title="删除"
                      onClick={() => handleRemove(s)}
                      disabled={!s.writable}
                    >
                      <Trash2 size={14} />
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        {isLoading && <div className="py-14 text-center text-sm text-gray-400">加载中…</div>}
        {!isLoading && servers.length === 0 && (
          <div className="py-14 text-center text-sm text-gray-400">{emptyText}</div>
        )}
      </div>

      {editing && (
        <McpEditor server={editing === "new" ? null : editing} onClose={() => setEditing(null)} />
      )}
      {syncing && (
        <SyncDialog
          server={syncing}
          onCancel={() => setSyncing(null)}
          onConfirm={doSync}
        />
      )}
    </>
  );
}

/** 新增 / 编辑 MCP。server 为 null 表示新增；defaultAgent 指定新增时预设的 agent */
export function McpEditor({
  server,
  defaultAgent,
  onClose,
}: {
  server: McpServer | null;
  defaultAgent?: string;
  onClose: () => void;
}) {
  const upsert = useUpsertMcp();
  const toast = useToastStore((s) => s.push);
  const { nameOf, writableTargets } = useMcpAgentMeta();
  const isNew = server === null;

  const [agentId, setAgentId] = useState(server?.agentId ?? defaultAgent ?? "");
  const [name, setName] = useState(server?.name ?? "");
  const [transport, setTransport] = useState(server?.transport ?? "stdio");
  const [command, setCommand] = useState(server?.command ?? "");
  const [args, setArgs] = useState((server?.args ?? []).join(" "));
  const [url, setUrl] = useState(server?.url ?? "");
  const [env, setEnv] = useState<[string, string][]>(
    server?.env ?? ([] as [string, string][]),
  );
  const [headers, setHeaders] = useState<[string, string][]>(
    server?.headers ?? ([] as [string, string][]),
  );

  // 可写目标来自后端探测，是异步的：加载完成后补一个默认值
  useEffect(() => {
    if (!agentId && writableTargets.length > 0) setAgentId(writableTargets[0].id);
  }, [agentId, writableTargets]);

  // 编辑已有条目时，其 agent 可能不在可写清单里（如源文件缺失），仍要能显示当前值
  const targetOptions = (() => {
    const list = writableTargets.map((t) => ({ id: t.id, name: t.name }));
    if (agentId && !list.some((t) => t.id === agentId)) {
      list.unshift({ id: agentId, name: nameOf(agentId) });
    }
    return list;
  })();

  const submit = () => {
    const input: McpServerInput = {
      name: name.trim(),
      transport,
      command: transport === "stdio" ? command.trim() || null : null,
      args:
        transport === "stdio"
          ? args
              .split(/\s+/)
              .map((a) => a.trim())
              .filter(Boolean)
          : [],
      url: transport === "http" ? url.trim() || null : null,
      env: env.filter(([k]) => k.trim()),
      headers: headers.filter(([k]) => k.trim()),
      enabled: true,
    };
    if (!input.name) {
      toast("名称不能为空");
      return;
    }
    if (transport === "stdio" && !input.command) {
      toast("stdio 类型必须填命令");
      return;
    }
    if (transport === "http" && !input.url) {
      toast("http 类型必须填 URL");
      return;
    }
    upsert.mutate(
      { agentId, name: isNew ? input.name : server!.name, input },
      {
        onSuccess: () => {
          toast("已保存（重启 agent 后生效）", "info");
          onClose();
        },
        onError: (e: unknown) =>
          toast(`保存失败：${e instanceof Error ? e.message : String(e)}`),
      },
    );
  };

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/30">
      <div className="max-h-[86vh] w-[520px] overflow-y-auto rounded-xl bg-white p-4 shadow-xl">
        <div className="mb-3 text-[14px] font-semibold">
          {isNew ? "新增 MCP" : `编辑「${server!.name}」`}
        </div>

        <div className="space-y-3 text-[12.5px]">
          <Field label="所属 agent">
            <select
              value={agentId}
              onChange={(e) => setAgentId(e.target.value)}
              disabled={!isNew}
              className="w-full rounded-md border border-black/10 px-2 py-1.5"
            >
              {targetOptions.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="名称">
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="lanhu"
              className="w-full rounded-md border border-black/10 px-2 py-1.5 font-mono"
            />
          </Field>
          <Field label="类型">
            <select
              value={transport}
              onChange={(e) => setTransport(e.target.value)}
              className="w-full rounded-md border border-black/10 px-2 py-1.5"
            >
              <option value="stdio">stdio（本地命令）</option>
              <option value="http">http（远程地址）</option>
            </select>
          </Field>
          {transport === "stdio" ? (
            <>
              <Field label="命令">
                <input
                  value={command}
                  onChange={(e) => setCommand(e.target.value)}
                  placeholder="npx"
                  className="w-full rounded-md border border-black/10 px-2 py-1.5 font-mono"
                />
              </Field>
              <Field label="参数（空格分隔）">
                <input
                  value={args}
                  onChange={(e) => setArgs(e.target.value)}
                  placeholder="-y @upstash/context7-mcp@latest"
                  className="w-full rounded-md border border-black/10 px-2 py-1.5 font-mono"
                />
              </Field>
            </>
          ) : (
            <Field label="URL">
              <input
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="http://127.0.0.1:8123/mcp"
                className="w-full rounded-md border border-black/10 px-2 py-1.5 font-mono"
              />
            </Field>
          )}
          <PairsEditor
            label="环境变量"
            hint={`值显示为 ${MCP_MASK} 表示沿用原值；填新值才会覆盖`}
            pairs={env}
            onChange={setEnv}
          />
          <PairsEditor label="请求头" pairs={headers} onChange={setHeaders} />
        </div>

        <div className="mt-4 flex justify-end gap-2">
          <button
            className="rounded-lg border border-black/10 px-3 py-1.5 text-[12.5px] text-gray-600 hover:bg-gray-50"
            onClick={onClose}
          >
            取消
          </button>
          <button
            className="rounded-lg bg-blue-600 px-3 py-1.5 text-[12.5px] font-medium text-white hover:bg-blue-700"
            onClick={submit}
            disabled={upsert.isPending}
          >
            保存
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <div className="mb-1 text-[11px] uppercase tracking-wide text-gray-400">{label}</div>
      {children}
    </div>
  );
}

function PairsEditor({
  label,
  hint,
  pairs,
  onChange,
}: {
  label: string;
  hint?: string;
  pairs: [string, string][];
  onChange: (p: [string, string][]) => void;
}) {
  return (
    <div>
      <div className="mb-1 flex items-baseline gap-2">
        <span className="text-[11px] uppercase tracking-wide text-gray-400">{label}</span>
        {hint && <span className="text-[10.5px] text-gray-400">{hint}</span>}
        <button
          className="ml-auto text-[11px] text-blue-600 hover:underline"
          onClick={() => onChange([...pairs, ["", ""]])}
        >
          + 添加
        </button>
      </div>
      <div className="space-y-1">
        {pairs.map(([k, v], i) => (
          <div key={i} className="flex gap-1.5">
            <input
              value={k}
              onChange={(e) => {
                const next = [...pairs] as [string, string][];
                next[i] = [e.target.value, v];
                onChange(next);
              }}
              placeholder="KEY"
              className="w-2/5 rounded-md border border-black/10 px-2 py-1 font-mono text-[11.5px]"
            />
            <input
              value={v}
              onChange={(e) => {
                const next = [...pairs] as [string, string][];
                next[i] = [k, e.target.value];
                onChange(next);
              }}
              placeholder="VALUE"
              className="flex-1 rounded-md border border-black/10 px-2 py-1 font-mono text-[11.5px]"
            />
            <button
              className="rounded px-1.5 text-gray-400 hover:bg-red-50 hover:text-red-500"
              onClick={() => onChange(pairs.filter((_, j) => j !== i))}
            >
              <Trash2 size={13} />
            </button>
          </div>
        ))}
        {pairs.length === 0 && <div className="text-[11.5px] text-gray-400">无</div>}
      </div>
    </div>
  );
}

function SyncDialog({
  server,
  onCancel,
  onConfirm,
}: {
  server: McpServer;
  onCancel: () => void;
  onConfirm: (targets: string[]) => void;
}) {
  const [targets, setTargets] = useState<string[]>([]);
  const { writableTargets } = useMcpAgentMeta();
  const candidates = writableTargets.filter((t) => t.id !== server.agentId);
  const toggle = (a: string) =>
    setTargets((t) => (t.includes(a) ? t.filter((x) => x !== a) : [...t, a]));

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/30">
      <div className="w-[400px] rounded-xl bg-white p-4 shadow-xl">
        <div className="text-[14px] font-semibold">同步「{server.name}」</div>
        <p className="mt-1.5 text-[12px] text-gray-500">
          把当前这份配置写入目标 agent 的全局 MCP 段（同名会被覆盖，写前自动备份）。
        </p>
        <div className="mt-3 space-y-1.5">
          {candidates.map((t) => (
            <label
              key={t.id}
              className="flex items-center gap-2 rounded-md border border-black/10 px-2 py-1.5 text-[12.5px]"
            >
              <input
                type="checkbox"
                checked={targets.includes(t.id)}
                onChange={() => toggle(t.id)}
              />
              {t.name}
            </label>
          ))}
          {candidates.length === 0 && (
            <div className="text-[12px] text-gray-400">没有其它可写入的 agent</div>
          )}
        </div>
        <div className="mt-4 flex justify-end gap-2">
          <button
            className="rounded-lg border border-black/10 px-3 py-1.5 text-[12.5px] text-gray-600 hover:bg-gray-50"
            onClick={onCancel}
          >
            取消
          </button>
          <button
            className="rounded-lg bg-blue-600 px-3 py-1.5 text-[12.5px] font-medium text-white hover:bg-blue-700 disabled:opacity-40"
            disabled={targets.length === 0}
            onClick={() => onConfirm(targets)}
          >
            同步到 {targets.length} 个 agent
          </button>
        </div>
      </div>
    </div>
  );
}
