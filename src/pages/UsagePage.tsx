import { RefreshCw } from "lucide-react";
import { useAgents, useAllUsage, useRefreshUsage } from "@/hooks";
import { formatTime } from "@/lib/utils";
import type { AgentUsage, UsagePoint } from "@/types";

/** 最近 n 天的连续日期序列（YYYY-MM-DD，升序） */
function lastNDays(n: number): string[] {
  const out: string[] = [];
  const now = new Date();
  for (let i = n - 1; i >= 0; i--) {
    const d = new Date(now.getTime() - i * 86_400_000);
    out.push(
      `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
        d.getDate(),
      ).padStart(2, "0")}`,
    );
  }
  return out;
}

/** 把稀疏的 trend 补齐成最近 days 天的连续序列，缺失补 0 */
function toSeries(trend: UsagePoint[], days: number): { date: string; count: number }[] {
  const map = new Map((trend ?? []).map((p) => [p.date, p.count]));
  return lastNDays(days).map((date) => ({ date, count: map.get(date) ?? 0 }));
}

function sumSeries(series: { count: number }[]) {
  return series.reduce((s, p) => s + p.count, 0);
}

function TrendBars({
  series,
  height = 44,
  className = "bg-blue-400",
}: {
  series: { date: string; count: number }[];
  height?: number;
  className?: string;
}) {
  const max = Math.max(1, ...series.map((p) => p.count));
  return (
    <div className="flex items-end gap-[2px]" style={{ height }}>
      {series.map((p) => (
        <div
          key={p.date}
          title={`${p.date} · ${p.count} 次`}
          className={`flex-1 rounded-[2px] ${p.count > 0 ? className : "bg-gray-100"}`}
          style={{
            height: p.count > 0 ? `${Math.max(10, (p.count / max) * 100)}%` : "2px",
          }}
        />
      ))}
    </div>
  );
}

export function UsagePage() {
  const { data: agents } = useAgents();
  const { data: usage, isLoading } = useAllUsage();
  const refresh = useRefreshUsage();

  const nameOf = (id: string) => agents?.find((a) => a.id === id)?.name ?? id;

  const refreshAll = async () => {
    for (const a of agents ?? []) {
      await refresh.mutateAsync(a.id).catch(() => undefined);
    }
  };

  if (isLoading) {
    return <div className="p-6 text-sm text-gray-400">统计中…（首次需扫描各 agent 会话日志）</div>;
  }

  const list = usage ?? [];
  const supported = list.filter((u) => u.supported && u.total > 0);
  const grand = supported.reduce((sum, u) => sum + u.total, 0);

  // 总趋势：所有 agent 按天合并
  const merged = new Map<string, number>();
  for (const u of supported) {
    for (const p of u.trend ?? []) {
      merged.set(p.date, (merged.get(p.date) ?? 0) + p.count);
    }
  }
  const totalSeries = toSeries(
    Array.from(merged, ([date, count]) => ({ date, count })),
    30,
  );
  const last7 = sumSeries(totalSeries.slice(-7));
  const last30 = sumSeries(totalSeries);
  const activeDays = totalSeries.filter((p) => p.count > 0).length;

  return (
    <div className="mx-auto max-w-7xl p-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold">使用统计</h1>
          <div className="mt-0.5 text-[12px] text-gray-500">
            按 agent 分别统计：数据来自各自会话日志，互不相通
          </div>
        </div>
        <button
          className="inline-flex items-center gap-1.5 rounded-lg border border-black/10 px-2.5 py-1.5 text-[12px] text-gray-600 hover:bg-gray-50 disabled:opacity-50"
          disabled={refresh.isPending}
          onClick={refreshAll}
        >
          <RefreshCw size={13} className={refresh.isPending ? "animate-spin" : ""} />
          {refresh.isPending ? "重新统计中…" : "全部重新统计"}
        </button>
      </div>

      {grand > 0 && (
        <div className="mt-4 rounded-xl border border-black/8 bg-white p-4">
          <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-[13px] text-gray-600">
            <span>
              共记录{" "}
              <span className="font-medium text-gray-900 tabular-nums">{grand}</span> 次技能调用
            </span>
            <span className="text-gray-400">
              近 7 天 <span className="tabular-nums text-gray-700">{last7}</span> 次
            </span>
            <span className="text-gray-400">
              近 30 天 <span className="tabular-nums text-gray-700">{last30}</span> 次
            </span>
            <span className="text-gray-400">
              活跃 <span className="tabular-nums text-gray-700">{activeDays}</span> / 30 天
            </span>
          </div>
          <div className="mt-3">
            <TrendBars series={totalSeries} height={64} />
            <div className="mt-1 flex justify-between text-[10px] text-gray-400">
              <span>{totalSeries[0]?.date ?? ""}</span>
              <span>近 30 天使用趋势（全部 agent 合并）</span>
              <span>{totalSeries[totalSeries.length - 1]?.date ?? ""}</span>
            </div>
          </div>
        </div>
      )}

      <div className="mt-4 grid gap-3 md:grid-cols-2">
        {list.map((u) => (
          <AgentUsageCard
            key={u.agentId}
            usage={u}
            name={nameOf(u.agentId)}
            onRefresh={() => refresh.mutate(u.agentId)}
            pending={refresh.isPending}
          />
        ))}
      </div>
    </div>
  );
}

function AgentUsageCard({
  usage,
  name,
  onRefresh,
  pending,
}: {
  usage: AgentUsage;
  name: string;
  onRefresh: () => void;
  pending: boolean;
}) {
  const top = usage.stats.slice(0, 8);
  const max = top[0]?.count ?? 1;
  const series = toSeries(usage.trend ?? [], 30);
  const hasTrend = series.some((p) => p.count > 0);

  return (
    <div className="rounded-xl border border-black/8 bg-white p-4">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="truncate text-[13px] font-medium text-gray-900">{name}</div>
          <div className="mt-0.5 flex flex-wrap items-center gap-1.5 text-[11px] text-gray-500">
            {usage.supported ? (
              <>
                <span className="tabular-nums">
                  合计 {usage.total} 次 · {usage.stats.length} 个技能被使用
                </span>
                <span
                  className={
                    usage.precision === "exact"
                      ? "rounded bg-emerald-50 px-1.5 text-emerald-700"
                      : "rounded bg-amber-50 px-1.5 text-amber-700"
                  }
                >
                  {usage.precision === "exact" ? "精确" : "近似"}
                </span>
              </>
            ) : (
              <span className="text-gray-400">无可用数据源</span>
            )}
          </div>
        </div>
        <button
          className="shrink-0 rounded p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-600 disabled:opacity-50"
          disabled={pending}
          title="重新统计该 agent"
          onClick={onRefresh}
        >
          <RefreshCw size={12} className={pending ? "animate-spin" : ""} />
        </button>
      </div>

      {hasTrend && (
        <div className="mt-3">
          <TrendBars series={series} height={36} className="bg-indigo-400" />
          <div className="mt-1 flex justify-between text-[10px] text-gray-400">
            <span>{series[0]?.date ?? ""}</span>
            <span>近 30 天</span>
            <span>{series[series.length - 1]?.date ?? ""}</span>
          </div>
        </div>
      )}

      {top.length > 0 ? (
        <div className="mt-3 space-y-1.5">
          {top.map((s) => (
            <div key={s.folder} className="flex items-center gap-2">
              <div className="w-40 shrink-0 truncate text-[12px] text-gray-700" title={s.folder}>
                {s.folder}
              </div>
              <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-gray-100">
                <div
                  className="h-full rounded-full bg-blue-500"
                  style={{ width: `${Math.max(6, (s.count / max) * 100)}%` }}
                />
              </div>
              <div className="w-8 shrink-0 text-right text-[12px] tabular-nums text-gray-600">
                {s.count}
              </div>
              <div className="w-24 shrink-0 truncate text-right text-[10px] text-gray-400">
                {s.lastUsed ? formatTime(s.lastUsed).slice(0, 10) : ""}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="mt-3 text-[12px] text-gray-400">{usage.note}</div>
      )}

      <div className="mt-3 border-t border-black/5 pt-2 text-[10px] leading-relaxed text-gray-400">
        {usage.note}
        {usage.scannedFiles > 0 && ` · 扫描 ${usage.scannedFiles} 个文件`}
      </div>
    </div>
  );
}
