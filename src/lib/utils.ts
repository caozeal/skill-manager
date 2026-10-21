import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useToastStore } from "@/stores/ui";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

function parentDir(path: string): string | null {
  const i = path.replace(/\/+$/, "").lastIndexOf("/");
  return i > 0 ? path.slice(0, i) : null;
}

/**
 * 在 Finder 中显示技能。断链符号链接无法被 canonicalize（revealItemInDir 内部
 * 会先 std::fs::canonicalize，断链时抛 ENOENT），因此断链时直接打开父目录；
 * 其他情况若 reveal 失败也兜底打开父目录。
 */
export async function revealSkillInFinder(
  path: string,
  mountType?: string,
): Promise<void> {
  try {
    if (mountType === "broken_symlink") {
      const parent = parentDir(path);
      if (parent) {
        await revealItemInDir(parent);
        return;
      }
    }
    await revealItemInDir(path);
  } catch (e) {
    const parent = parentDir(path);
    if (parent && parent !== path) {
      try {
        await revealItemInDir(parent);
        return;
      } catch {
        // 兜底也失败，给出可见反馈而不是静默
      }
    }
    useToastStore
      .getState()
      .push(`在 Finder 中显示失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

export function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function formatTime(iso: string | null): string {
  if (!iso) return "—";
  return iso.replace("T", " ").replace(/\.\d+Z$/, "").replace(/Z$/, "");
}
