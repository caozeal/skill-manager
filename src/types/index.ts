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
