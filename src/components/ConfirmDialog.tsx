import { AlertTriangle, CheckCircle2, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { useConfirmStore, useToastStore } from "@/stores/ui";

/**
 * 全局反馈层：应用内确认弹窗 + toast。
 * 必须在 App 中渲染一次。之所以不用 window.confirm/alert：
 * Tauri(wry) 的 WKWebView 未实现对应 UI delegate，调用会静默返回 false/undefined。
 */
export function ConfirmDialog() {
  const current = useConfirmStore((s) => s.current);
  const answer = useConfirmStore((s) => s.answer);
  const toasts = useToastStore((s) => s.toasts);

  if (!current) {
    return toasts.length ? <Toasts /> : null;
  }

  return (
    <>
      <div className="fixed inset-0 z-40 bg-black/30" onClick={() => answer(false)} />
      <div className="fixed left-1/2 top-1/2 z-50 w-[400px] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-black/10 bg-white p-4 shadow-xl">
        <div className="flex items-start gap-2.5">
          {current.danger && (
            <AlertTriangle size={18} className="mt-0.5 shrink-0 text-red-500" />
          )}
          <div className="min-w-0 flex-1">
            <div className="text-[14px] font-semibold text-gray-900">{current.title}</div>
            <div className="mt-1.5 whitespace-pre-wrap text-[12.5px] leading-relaxed text-gray-600">
              {current.message}
            </div>
          </div>
        </div>
        <div className="mt-4 flex justify-end gap-2">
          <button
            className="rounded-lg border border-black/10 px-3 py-1.5 text-[12.5px] text-gray-600 hover:bg-gray-50"
            onClick={() => answer(false)}
          >
            取消
          </button>
          <button
            autoFocus
            className={cn(
              "rounded-lg px-3 py-1.5 text-[12.5px] font-medium text-white",
              current.danger ? "bg-red-500 hover:bg-red-600" : "bg-blue-600 hover:bg-blue-700",
            )}
            onClick={() => answer(true)}
          >
            {current.confirmText ?? "确定"}
          </button>
        </div>
      </div>
      <Toasts />
    </>
  );
}

function Toasts() {
  const toasts = useToastStore((s) => s.toasts);
  const dismiss = useToastStore((s) => s.dismiss);
  if (!toasts.length) return null;
  return (
    <div className="fixed bottom-4 left-1/2 z-[60] flex -translate-x-1/2 flex-col items-center gap-2">
      {toasts.map((t) => (
        <div
          key={t.id}
          className={cn(
            "flex items-center gap-1.5 rounded-lg px-3 py-2 text-[12.5px] shadow-lg",
            t.kind === "error" ? "bg-red-600 text-white" : "bg-gray-800 text-white",
          )}
        >
          {t.kind === "error" ? (
            <AlertTriangle size={13} className="shrink-0" />
          ) : (
            <CheckCircle2 size={13} className="shrink-0" />
          )}
          <span className="max-w-[420px] break-all">{t.text}</span>
          <button className="ml-1 opacity-70 hover:opacity-100" onClick={() => dismiss(t.id)}>
            <X size={12} />
          </button>
        </div>
      ))}
    </div>
  );
}
