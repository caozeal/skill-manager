use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub skills_path: Option<String>,
    pub exists: bool,
    pub read_only: bool,
    pub kind: String, // "standard" | "informational"
    pub note: String,
    pub skill_count: u32,
    pub symlink_count: u32,
    pub real_count: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MountType {
    RealDir,
    Symlink,
    BrokenSymlink,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MountInfo {
    pub mount_type: MountType,
    pub raw_target: Option<String>,
    pub resolved_target: Option<String>,
    pub points_to_central: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LockEntry {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub source_type: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub skill_path: String,
    #[serde(default)]
    pub installed_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub agent_id: String,
    pub folder: String,
    pub path: String,
    pub name: String,
    pub description: String,
    pub skill_md_found: bool,
    pub mount: MountInfo,
    pub is_system: bool,
    pub is_global: bool,
    pub is_compat: bool,
    pub source: Option<LockEntry>,
    pub dir_size: u64,
    pub file_count: u32,
    pub modified_at: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub children: Vec<FileNode>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    pub skill: Skill,
    pub frontmatter_raw: String,
    pub body_md: String,
    pub file_tree: FileNode,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontmatterUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
}
