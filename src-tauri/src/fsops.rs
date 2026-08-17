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
    let canon_full = fs::canonicalize(&full).map_err(|_| AppError::AgentDirMissing)?;
    if !canon_full.starts_with(&canon_dir) {
        return Err(AppError::AgentNotFound(format!("非法路径: {folder}")));
    }
    Ok(canon_full)
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

fn backup_tag() -> String {
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
