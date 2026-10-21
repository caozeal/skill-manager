export type MountType = "real_dir" | "symlink" | "broken_symlink";

export interface MountInfo {
  mountType: MountType;
  rawTarget: string | null;
  resolvedTarget: string | null;
  pointsToCentral: boolean;
}

export interface LockEntry {
  source: string;
  sourceType: string;
  sourceUrl: string;
  skillPath: string;
  installedAt: string | null;
  updatedAt: string | null;
}

export interface Agent {
  id: string;
  name: string;
  icon: string;
  skillsPath: string | null;
  exists: boolean;
  readOnly: boolean;
  kind: "standard" | "informational";
  note: string;
  skillCount: number;
  symlinkCount: number;
  realCount: number;
}

export interface Skill {
  agentId: string;
  folder: string;
  path: string;
  name: string;
  description: string;
  skillMdFound: boolean;
  mount: MountInfo;
  isSystem: boolean;
  isGlobal: boolean;
  isCompat: boolean;
  /** 系统 tier 标签：内置 / 精选 / 实验（Codex 的 .system 等目录） */
  systemTier?: string | null;
  disabled: boolean;
  source: LockEntry | null;
  dirSize: number;
  fileCount: number;
  modifiedAt: string | null;
}

export interface FileNode {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  children: FileNode[];
}

export interface UsageStat {
  folder: string;
  count: number;
  lastUsed: string | null;
}

/** 趋势上的单个点（按天聚合） */
export interface UsagePoint {
  /** YYYY-MM-DD */
  date: string;
  count: number;
}

export interface AgentUsage {
  agentId: string;
  supported: boolean;
  /** exact（工具调用记录） / approx（文本提及近似） / none */
  precision: string;
  scannedFiles: number;
  total: number;
  lastScan: string | null;
  stats: UsageStat[];
  /** 按天聚合的使用趋势（日期升序） */
  trend: UsagePoint[];
  note: string;
}

export interface SkillDetail {
  skill: Skill;
  frontmatterRaw: string;
  bodyMd: string;
  fileTree: FileNode;
}

export interface FrontmatterUpdate {
  name?: string;
  description?: string;
}

/** env / headers 里敏感值的掩码；原样回传表示「不改这一项」 */
export const MCP_MASK = "\u2022\u2022\u2022\u2022";

export interface McpServer {
  /** agent|scope|project|name */
  id: string;
  agentId: string;
  name: string;
  /** stdio / http / unknown */
  transport: string;
  command: string | null;
  args: string[];
  url: string | null;
  /** 键值对，值已打码 */
  env: [string, string][];
  headers: [string, string][];
  enabled: boolean;
  /** global / project */
  scope: string;
  project: string | null;
  sourcePath: string;
  /** 是否支持原生 enabled（否则禁用 = 移除条目并留存到影子文件） */
  nativeEnabled: boolean;
  writable: boolean;
}

export interface McpServerInput {
  name: string;
  transport: string;
  command: string | null;
  args: string[];
  url: string | null;
  env: [string, string][];
  headers: [string, string][];
  enabled: boolean;
}

/** MCP 配置目标（哪些 agent 有 MCP 配置源、能否写入）——后端运行时给出，前端不写死 */
export interface McpTarget {
  id: string;
  name: string;
  /** 该源是否允许写入 */
  writable: boolean;
  /** 配置文件是否已存在；不存在时写入会失败 */
  available: boolean;
}
