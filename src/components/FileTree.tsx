import { useState } from "react";
import { ChevronRight, File, FolderOpen } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { FileNode } from "@/types";
import { formatSize } from "@/lib/utils";

export function FileTree({ node, depth = 0 }: { node: FileNode; depth?: number }) {
  const [open, setOpen] = useState(depth < 2);

  if (node.isDir) {
    return (
      <div>
        <button
          className="flex w-full items-center gap-1 rounded px-1 py-0.5 text-left text-[12px] text-gray-700 hover:bg-gray-100"
          style={{ paddingLeft: depth * 14 + 4 }}
          onClick={() => setOpen(!open)}
        >
          <ChevronRight
            size={12}
            className={`shrink-0 transition-transform ${open ? "rotate-90" : ""}`}
          />
          <span className="truncate font-medium">{node.name}</span>
        </button>
        {open &&
          node.children.map((c) => <FileTree key={c.path} node={c} depth={depth + 1} />)}
      </div>
    );
  }
  return (
    <button
      className="flex w-full items-center gap-1.5 rounded px-1 py-0.5 text-left text-[12px] text-gray-500 hover:bg-gray-100 hover:text-gray-700"
      style={{ paddingLeft: depth * 14 + 20 }}
      title={`${node.path} · ${formatSize(node.size)}`}
      onClick={() => revealItemInDir(node.path)}
    >
      <File size={12} className="shrink-0 opacity-60" />
      <span className="truncate">{node.name}</span>
      <span className="ml-auto shrink-0 text-[10px] text-gray-400">{formatSize(node.size)}</span>
    </button>
  );
}

export function FileTreeRoot({ node }: { node: FileNode }) {
  return (
    <div className="rounded-lg border border-black/8 p-2">
      <div className="mb-1 flex items-center gap-1.5 px-1 text-[12px] font-medium text-gray-700">
        <FolderOpen size={13} className="opacity-60" />
        {node.name}
      </div>
      {node.children.map((c) => (
        <FileTree key={c.path} node={c} depth={1} />
      ))}
    </div>
  );
}
