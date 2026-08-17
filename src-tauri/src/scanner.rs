use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

use crate::adapters::{AdapterKind, AgentAdapter, central_dir};
use crate::lockfile;
use crate::model::{Agent, FileNode, LockEntry, MountInfo, MountType, Skill};

const MAX_DEPTH: usize = 6;
const MAX_ENTRIES: usize = 5000;

/// 扫描单个 agent，返回技能列表（按名称排序）。
/// 合并逻辑：
/// - global_skills_source（如 Codex/DSH → 中央仓库）：源技能标 is_global=true（紫色「全局」）
/// - compat_skills_source（如 OpenCode → ~/.claude/skills）：源技能标 is_compat=true（青色「兼容」）
pub fn scan_agent(adapter: &AgentAdapter) -> Vec<Skill> {
    let mut skills = scan_local(adapter);
    let mut seen: std::collections::HashSet<String> =
        skills.iter().map(|s| s.folder.clone()).collect();

    if let Some(src_id) = adapter.global_skills_source {
        if let Some(src) = crate::adapters::find(src_id) {
            for mut s in scan_local(src) {
                if s.is_system || seen.contains(&s.folder) {
                    continue; // 跳过源的内置项与已存在同名技能
                }
                s.agent_id = adapter.id.to_string();
                s.is_global = true;
                seen.insert(s.folder.clone());
                skills.push(s);
            }
        }
    }

    if let Some(src_id) = adapter.compat_skills_source {
        if let Some(src) = crate::adapters::find(src_id) {
            for mut s in scan_local(src) {
                if s.is_system || seen.contains(&s.folder) {
                    continue;
                }
                s.agent_id = adapter.id.to_string();
                s.is_compat = true;
                seen.insert(s.folder.clone());
                skills.push(s);
            }
        }
    }

    skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    skills
}

/// 仅扫描 agent 自身技能目录（不合并全局技能）
pub fn scan_local(adapter: &AgentAdapter) -> Vec<Skill> {
    let Some(dir) = &adapter.skills_path else {
        return Vec::new();
    };
    if !dir.is_dir() {
        return Vec::new();
    }
    let lock = lockfile::load_lock();
    let central = fs::canonicalize(central_dir()).ok();

    let mut skills: Vec<Skill> = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') && name != ".system" {
            continue; // 过滤 .DS_Store 等隐藏项，但保留 .system
        }
        let path = entry.path();
        let lmeta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        // 只处理目录（含指向目录的符号链接）；.system 本身是目录
        if !lmeta.is_dir() && !lmeta.file_type().is_symlink() {
            continue;
        }
        let is_system = name == ".system";
        let mount = mount_info(&path, central.as_deref());
        let (fm_name, fm_desc, skill_md_found, modified) = read_skill_meta(&path);
        let (dir_size, file_count) = dir_stats(&path);
        let source = lockfile::lookup(&lock, &name).cloned();
        skills.push(Skill {
            agent_id: adapter.id.to_string(),
            folder: name.clone(),
            path: path.to_string_lossy().to_string(),
            name: fm_name.unwrap_or_else(|| name.clone()),
            description: fm_desc.unwrap_or_default(),
            skill_md_found,
            mount,
            is_system,
            is_global: false,
            is_compat: false,
            source: source.filter(|s: &LockEntry| !s.source.is_empty()),
            dir_size,
            file_count,
            modified_at: modified,
        });
    }
    skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    skills
}

/// Agent 概览（含计数）
pub fn agent_overview(adapter: &AgentAdapter) -> Agent {
    let skills = scan_agent(adapter);
    let symlink_count = skills
        .iter()
        .filter(|s| matches!(s.mount.mount_type, MountType::Symlink | MountType::BrokenSymlink))
        .count() as u32;
    let real_count = (skills.len() as u32).saturating_sub(symlink_count);
    Agent {
        id: adapter.id.to_string(),
        name: adapter.name.to_string(),
        icon: adapter.icon.to_string(),
        skills_path: adapter
            .skills_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        exists: adapter
            .skills_path
            .as_ref()
            .map(|p| p.is_dir())
            .unwrap_or(false),
        read_only: adapter.read_only,
        kind: match adapter.kind {
            AdapterKind::Standard => "standard".into(),
            AdapterKind::Informational => "informational".into(),
        },
        note: adapter.note.to_string(),
        skill_count: skills.len() as u32,
        symlink_count,
        real_count,
    }
}

/// 判断挂载类型：真实目录 / 符号链接 / 断链
pub fn mount_info(path: &Path, central: Option<&Path>) -> MountInfo {
    let lmeta = fs::symlink_metadata(path);
    let is_symlink = lmeta
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    if !is_symlink {
        return MountInfo {
            mount_type: MountType::RealDir,
            raw_target: None,
            resolved_target: None,
            points_to_central: false,
        };
    }
    let raw = fs::read_link(path).ok();
    // 相对目标需相对链接所在目录解析
    let resolved: Option<PathBuf> = raw.as_ref().and_then(|t| {
        let abs = if t.is_absolute() {
            t.clone()
        } else {
            path.parent().unwrap_or(Path::new("/")).join(t)
        };
        fs::canonicalize(&abs).ok()
    });
    let points_to_central = match (&resolved, central) {
        (Some(r), Some(c)) => r == c || r.starts_with(c),
        _ => false,
    };
    MountInfo {
        mount_type: if resolved.is_some() {
            MountType::Symlink
        } else {
            MountType::BrokenSymlink
        },
        raw_target: raw.map(|p| p.to_string_lossy().to_string()),
        resolved_target: resolved.map(|p| p.to_string_lossy().to_string()),
        points_to_central,
    }
}

/// 读取 SKILL.md：返回 (name, description, found, mtime)
fn read_skill_meta(dir: &Path) -> (Option<String>, Option<String>, bool, Option<String>) {
    let md = dir.join("SKILL.md");
    if !md.is_file() {
        return (None, None, false, None);
    }
    let modified = fs::metadata(&md)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| {
            chrono_like_time(d.as_secs())
        });
    let Ok(text) = fs::read_to_string(&md) else {
        return (None, None, true, modified);
    };
    let (fm, _) = split_frontmatter(&text);
    match fm {
        Some(raw) => {
            #[derive(serde::Deserialize, Default)]
            struct Fm {
                #[serde(default)]
                name: Option<String>,
                #[serde(default)]
                description: Option<String>,
            }
            match serde_yaml::from_str::<Fm>(&raw) {
                Ok(f) => (f.name, f.description, true, modified),
                Err(_) => (None, None, true, modified),
            }
        }
        None => (None, None, true, modified),
    }
}

/// 切分 frontmatter：返回 (frontmatter原文, 正文)
pub fn split_frontmatter(text: &str) -> (Option<String>, &str) {
    if let Some(rest) = text.strip_prefix("---") {
        // 跳过 --- 后的首个换行
        let rest = rest.trim_start_matches('\r').strip_prefix('\n').unwrap_or(rest);
        if let Some(pos) = rest.find("\n---") {
            let fm = rest[..pos].to_string();
            let body = rest[pos + 4..].trim_start_matches(['\r', '\n']);
            return (Some(fm), body);
        }
    }
    (None, text)
}

/// 简易时间格式化（避免引入 chrono；仅用于展示）
fn chrono_like_time(secs: u64) -> String {
    // 转换为 YYYY-MM-DD HH:MM (UTC+8 近似)：用 libc 无依赖方案太啰嗦，直接算
    let days = secs / 86400;
    let rem = secs % 86400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // 自 1970-01-01 起的天数 -> 年月日（民用算法）
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

fn civil_from_days(z: i64) -> (i64, u64, u64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 统计目录大小与文件数（跟随符号链接一次；深度/条目防御）
pub fn dir_stats(path: &Path) -> (u64, u32) {
    let mut size = 0u64;
    let mut count = 0u32;
    for entry in WalkDir::new(path)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .take(MAX_ENTRIES)
        .flatten()
    {
        if entry.file_type().is_file() {
            count += 1;
            if let Ok(md) = entry.metadata() {
                size += md.len();
            }
        } else if entry.file_type().is_symlink() {
            count += 1;
        }
    }
    (size, count)
}

/// 构建文件树（不跟随符号链接；深度/条目防御）
pub fn file_tree(path: &Path) -> FileNode {
    fn build(p: &Path, depth: usize, counter: &mut usize) -> FileNode {
        *counter += 1;
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| p.to_string_lossy().to_string());
        let meta = fs::symlink_metadata(p);
        let is_symlink = meta.as_ref().map(|m| m.file_type().is_symlink()).unwrap_or(false);
        if is_symlink {
            let target = fs::read_link(p)
                .map(|t| t.to_string_lossy().to_string())
                .unwrap_or_default();
            return FileNode {
                name: format!("{name} → {target}"),
                path: p.to_string_lossy().to_string(),
                is_dir: false,
                size: 0,
                children: Vec::new(),
            };
        }
        let is_dir = p.is_dir();
        let size = meta.map(|m| m.len()).unwrap_or(0);
        let mut children = Vec::new();
        if is_dir && depth < MAX_DEPTH && *counter < MAX_ENTRIES {
            if let Ok(rd) = fs::read_dir(p) {
                let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                dirs.sort_by_key(|a| {
                    let is_dir = a.is_dir();
                    let name = a
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    (!is_dir, name.to_lowercase())
                });
                for child in dirs {
                    if *counter >= MAX_ENTRIES {
                        break;
                    }
                    children.push(build(&child, depth + 1, counter));
                }
            }
        }
        FileNode {
            name,
            path: p.to_string_lossy().to_string(),
            is_dir,
            size,
            children,
        }
    }
    let mut counter = 0usize;
    build(path, 0, &mut counter)
}
