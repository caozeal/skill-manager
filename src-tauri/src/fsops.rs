use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::adapters;
use crate::error::{AppError, AppResult};
use crate::model::{FrontmatterUpdate, Skill};
use crate::scanner;

/// 校验 folder 合法且解析后位于 agent 技能目录内（防路径穿越）
fn resolve_in_agent_dir(agent: &adapters::AgentAdapter, folder: &str) -> AppResult<PathBuf> {
    let dir = agent
        .skills_path
        .as_ref()
        .filter(|_| agent.has_skills_dir)
        .ok_or(AppError::AgentDirMissing)?;
    let full = dir.join(folder);
    // 拒绝绝对路径与 .. 穿越
    let mut ok = folder != "..";
    for c in Path::new(folder).components() {
        if !matches!(c, Component::Normal(_)) && c != Component::CurDir {
            ok = false;
            break;
        }
    }
    if !ok {
        return Err(AppError::AgentNotFound(format!("非法路径: {folder}")));
    }
    let canon_dir = fs::canonicalize(dir).map_err(|_| AppError::AgentDirMissing)?;
    let meta = fs::symlink_metadata(&full).map_err(|_| AppError::AgentDirMissing)?;
    if meta.file_type().is_symlink() {
        // 符号链接：只校验链接本体位于 agent 目录下（不跟随目标，
        // 因为目标可能在中央仓库等 agent 目录之外）
        return Ok(full);
    }
    let canon_full = fs::canonicalize(&full).map_err(|_| AppError::AgentDirMissing)?;
    if !canon_full.starts_with(&canon_dir) {
        return Err(AppError::AgentNotFound(format!("非法路径: {folder}")));
    }
    Ok(canon_full)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 核心不变量：禁用目录必须在 skills 目录之外，否则 agent 仍可能扫描到
    #[test]
    fn disabled_dir_sits_outside_skills_dir() {
        let agent = adapters::find("claude").expect("claude adapter 应存在");
        let skills = agent.skills_path.as_ref().expect("claude 有 skills 目录");
        let ddir = adapters::disabled_dir(agent).expect("应有 disabled 目录");
        assert_eq!(ddir, skills.parent().unwrap().join("disabled-skills"));
        assert!(!ddir.starts_with(skills), "禁用目录不能位于 skills 之内");
    }
}

/// folder 必须是单层合法目录名（禁止绝对路径、.. 穿越）
fn validate_folder(folder: &str) -> AppResult<()> {
    if folder.is_empty() || folder == ".." {
        return Err(AppError::AgentNotFound(format!("非法路径: {folder}")));
    }
    for c in Path::new(folder).components() {
        if !matches!(c, Component::Normal(_)) {
            return Err(AppError::AgentNotFound(format!("非法路径: {folder}")));
        }
    }
    Ok(())
}

fn check_writable(agent: &adapters::AgentAdapter, folder: &str) -> AppResult<()> {
    if agent.kind == adapters::AdapterKind::Informational || agent.read_only {
        return Err(AppError::ReadOnlyAgent);
    }
    if folder == ".system" || folder.starts_with(".system/") {
        return Err(AppError::SystemDirReadOnly);
    }
    Ok(())
}

/// 计算相对符号链接目标（保持与既有 `../../.agents/skills/x` 风格一致）
fn relative_target(from_dir: &Path, to: &Path) -> PathBuf {
    let from: Vec<_> = from_dir.components().collect();
    let to: Vec<_> = to.components().collect();
    let mut i = 0;
    while i < from.len() && i < to.len() && from[i] == to[i] {
        i += 1;
    }
    let up = from.len() - i;
    let mut rel = PathBuf::new();
    for _ in 0..up {
        rel.push("..");
    }
    for c in &to[i..] {
        rel.push(c.as_os_str());
    }
    rel
}

/// 挂载：在 agent 技能目录下创建指向中央仓库技能的符号链接
pub fn mount_skill(agent_id: &str, central_folder: &str) -> AppResult<Skill> {
    let agent = adapters::find(agent_id).ok_or_else(|| AppError::AgentNotFound(agent_id.into()))?;
    if !agent.supports_symlink_mount {
        return Err(AppError::MountNotSupported);
    }
    let central_skill = adapters::central_dir().join(central_folder);
    let central_skill = fs::canonicalize(&central_skill)
        .map_err(|_| AppError::CentralSkillMissing(central_folder.into()))?;
    let central_root = fs::canonicalize(adapters::central_dir())
        .map_err(|_| AppError::CentralSkillMissing("中央仓库不可用".into()))?;
    if !central_skill.starts_with(&central_root) || !central_skill.is_dir() {
        return Err(AppError::CentralSkillMissing(central_folder.into()));
    }
    let agent_dir = agent
        .skills_path
        .as_ref()
        .ok_or(AppError::MountNotSupported)?;
    fs::create_dir_all(agent_dir)?;
    let link_path = agent_dir.join(central_folder);
    if fs::symlink_metadata(&link_path).is_ok() {
        return Err(AppError::TargetExists);
    }
    let rel = relative_target(agent_dir, &central_skill);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&rel, &link_path)?;
    #[cfg(not(unix))]
    std::os::windows::fs::symlink_dir(&rel, &link_path)?;
    // 返回新挂载的技能信息
    let skills = scanner::scan_agent(agent);
    skills
        .into_iter()
        .find(|s| s.folder == central_folder)
        .ok_or_else(|| AppError::Io(std::io::Error::other("挂载后未找到技能")))
}

/// 卸载：仅允许删除符号链接，真实目录一律拒绝
pub fn unmount_skill(agent_id: &str, folder: &str) -> AppResult<()> {
    let agent = adapters::find(agent_id).ok_or_else(|| AppError::AgentNotFound(agent_id.into()))?;
    check_writable(agent, folder)?;
    let full = resolve_in_agent_dir(agent, folder)?;
    let meta = fs::symlink_metadata(&full).map_err(AppError::Io)?;
    if !meta.file_type().is_symlink() {
        return Err(AppError::RealDirNoDelete);
    }
    fs::remove_file(&full).map_err(AppError::Io)
}

/// 禁用：将技能目录重命名为 `.name`（隐藏目录），agent 扫描会跳过 → 技能失效。
/// 可逆：enable_skill 重命名回来。
pub fn disable_skill(agent_id: &str, folder: &str) -> AppResult<()> {
    let agent = adapters::find(agent_id).ok_or_else(|| AppError::AgentNotFound(agent_id.into()))?;
    check_writable(agent, folder)?;
    validate_folder(folder)?;
    let dir = agent
        .skills_path
        .as_ref()
        .filter(|_| agent.has_skills_dir)
        .ok_or(AppError::AgentDirMissing)?;
    let full = resolve_in_agent_dir(agent, folder)?;
    // 目标必须位于 agent 技能目录根下（禁用只支持一层技能目录）
    let canon_dir = fs::canonicalize(dir).map_err(|_| AppError::AgentDirMissing)?;
    let parent = full.parent().unwrap_or(Path::new("/"));
    let parent_canon = fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
    if parent_canon != canon_dir {
        return Err(AppError::AgentNotFound(format!("不支持嵌套技能禁用: {folder}")));
    }
    // 移到 skills 同级目录，彻底脱离 agent 的扫描范围
    let droot = adapters::disabled_dir(agent).ok_or(AppError::AgentDirMissing)?;
    fs::create_dir_all(&droot).map_err(AppError::Io)?;
    let dest = droot.join(folder);
    if fs::symlink_metadata(&dest).is_ok() {
        return Err(AppError::TargetExists);
    }
    fs::rename(&full, &dest).map_err(AppError::Io)
}

/// 启用：将 `.name` 重命名回 `name`
pub fn enable_skill(agent_id: &str, folder: &str) -> AppResult<()> {
    let agent = adapters::find(agent_id).ok_or_else(|| AppError::AgentNotFound(agent_id.into()))?;
    check_writable(agent, folder)?;
    validate_folder(folder)?;
    let dir = agent
        .skills_path
        .as_ref()
        .filter(|_| agent.has_skills_dir)
        .ok_or(AppError::AgentDirMissing)?;
    let enabled_path = dir.join(folder);
    if fs::symlink_metadata(&enabled_path).is_ok() {
        return Err(AppError::TargetExists);
    }
    // 新版位置：skills 同级的 disabled-skills/<folder>
    let new_path = adapters::disabled_dir(agent)
        .ok_or(AppError::AgentDirMissing)?
        .join(folder);
    // 旧版兼容：skills 下的 `.` 前缀目录
    let legacy_path = dir.join(format!(".{folder}"));
    let src = if fs::symlink_metadata(&new_path).is_ok() {
        new_path
    } else if fs::symlink_metadata(&legacy_path).is_ok() {
        legacy_path
    } else {
        return Err(AppError::AgentNotFound(format!("未找到已禁用的技能: {folder}")));
    };
    let meta = fs::symlink_metadata(&src).map_err(AppError::Io)?;
    if !meta.is_dir() && !meta.file_type().is_symlink() {
        return Err(AppError::AgentNotFound(format!("未找到已禁用的技能: {folder}")));
    }
    fs::create_dir_all(dir).map_err(AppError::Io)?;
    fs::rename(&src, &enabled_path).map_err(AppError::Io)
}

/// 删除：将技能目录移动到回收站 `~/.skill-manager-trash/<ts>-<folder>`（可手动恢复）。
/// 仅支持真实目录；符号链接请用 unmount。
pub fn delete_skill(agent_id: &str, folder: &str) -> AppResult<()> {
    let agent = adapters::find(agent_id).ok_or_else(|| AppError::AgentNotFound(agent_id.into()))?;
    check_writable(agent, folder)?;
    let full = resolve_in_agent_dir(agent, folder)?;
    let meta = fs::symlink_metadata(&full).map_err(AppError::Io)?;
    if meta.file_type().is_symlink() {
        return Err(AppError::MountNotSupported); // 链接请用卸载
    }
    if !meta.is_dir() {
        return Err(AppError::AgentNotFound(format!("不是目录: {folder}")));
    }
    let trash = dirs::home_dir()
        .expect("HOME 缺失")
        .join(".skill-manager-trash");
    fs::create_dir_all(&trash).map_err(AppError::Io)?;
    let dest = trash.join(format!("{}-{}", backup_tag(), folder));
    fs::rename(&full, &dest).map_err(AppError::Io)
}

pub fn backup_tag() -> String {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    let days = secs / 86400;
    let rem = secs % 86400;
    let (y, m, dd) = {
        let z = days as i64 + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = (z - era * 146097) as u64;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let mon = if mp < 10 { mp + 3 } else { mp - 9 };
        (if mon <= 2 { y + 1 } else { y }, mon, day)
    };
    format!("{y:04}{m:02}{dd:02}{:02}{:02}{:02}", rem / 3600, (rem % 3600) / 60, rem % 60)
}

/// 编辑 frontmatter（name/description），写前生成 .bak 备份
pub fn update_frontmatter(
    agent_id: &str,
    folder: &str,
    update: FrontmatterUpdate,
) -> AppResult<()> {
    let agent = adapters::find(agent_id).ok_or_else(|| AppError::AgentNotFound(agent_id.into()))?;
    check_writable(agent, folder)?;
    let full = resolve_in_agent_dir(agent, folder)?;
    let md = full.join("SKILL.md");
    if !md.is_file() {
        return Err(AppError::SkillMdMissing);
    }
    let text = fs::read_to_string(&md).map_err(AppError::Io)?;
    let (fm_raw, body) = scanner::split_frontmatter(&text);
    let fm_raw = fm_raw.ok_or_else(|| AppError::InvalidFrontmatter("缺少 frontmatter".into()))?;

    let mut fm: serde_yaml::Value =
        serde_yaml::from_str(&fm_raw).map_err(|e| AppError::InvalidFrontmatter(e.to_string()))?;
    if let Some(name) = update.name {
        if !name.trim().is_empty() {
            fm["name"] = serde_yaml::Value::String(name.trim().to_string());
        }
    }
    if let Some(desc) = update.description {
        fm["description"] = serde_yaml::Value::String(desc.trim().to_string());
    }
    let new_fm = serde_yaml::to_string(&fm).map_err(|e| AppError::InvalidFrontmatter(e.to_string()))?;
    let new_text = format!("---\n{}---\n\n{}", new_fm, body);

    // 备份
    let backup = md.with_file_name(format!("SKILL.md.bak-{}", backup_tag()));
    fs::copy(&md, &backup).map_err(AppError::Io)?;
    fs::write(&md, new_text).map_err(AppError::Io)
}

/// 删除指定技能的备份文件列表（M4 可选，暂不暴露）
#[allow(dead_code)]
pub fn list_backups(agent_id: &str, folder: &str) -> AppResult<Vec<String>> {
    let agent = adapters::find(agent_id).ok_or_else(|| AppError::AgentNotFound(agent_id.into()))?;
    let full = resolve_in_agent_dir(agent, folder)?;
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(&full) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("SKILL.md.bak-") {
                out.push(n);
            }
        }
    }
    Ok(out)
}
