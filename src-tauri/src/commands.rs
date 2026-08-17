use crate::adapters;
use crate::error::{AppError, AppResult};
use crate::fsops;
use crate::model::{Agent, FrontmatterUpdate, SkillDetail, Skill};
use crate::scanner;

#[tauri::command]
pub async fn list_agents() -> AppResult<Vec<Agent>> {
    tauri::async_runtime::spawn_blocking(|| {
        Ok(adapters::all_adapters()
            .iter()
            .map(scanner::agent_overview)
            .collect())
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?
}

#[tauri::command]
pub async fn scan_skills(agent_id: String) -> AppResult<Vec<Skill>> {
    tauri::async_runtime::spawn_blocking(move || {
        let agent = adapters::find(&agent_id)
            .ok_or_else(|| AppError::AgentNotFound(agent_id.clone()))?;
        // scan_agent 内部处理：无本地目录的 agent（如 DSH）返回空本地列表，
        // 但仍会合并 global_skills_source 的全局技能
        Ok(scanner::scan_agent(agent))
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?
}

#[tauri::command]
pub async fn scan_all_skills() -> AppResult<Vec<Skill>> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut out = Vec::new();
        for agent in adapters::all_adapters() {
            out.extend(scanner::scan_agent(agent));
        }
        Ok(out)
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?
}

#[tauri::command]
pub async fn get_skill_detail(agent_id: String, folder: String) -> AppResult<SkillDetail> {
    tauri::async_runtime::spawn_blocking(move || {
        let agent = adapters::find(&agent_id)
            .ok_or_else(|| AppError::AgentNotFound(agent_id.clone()))?;
        let skill = scanner::scan_agent(agent)
            .into_iter()
            .find(|s| s.folder == folder)
            .ok_or_else(|| AppError::AgentNotFound(format!("技能不存在: {folder}")))?;
        let dir = std::path::Path::new(&skill.path);
        let (frontmatter_raw, body_md) = match std::fs::read_to_string(dir.join("SKILL.md")) {
            Ok(text) => {
                let (fm, body) = crate::scanner::split_frontmatter(&text);
                (fm.unwrap_or_default().to_string(), body.to_string())
            }
            Err(_) => (String::new(), String::new()),
        };
        let file_tree = crate::scanner::file_tree(dir);
        Ok(SkillDetail {
            skill,
            frontmatter_raw,
            body_md,
            file_tree,
        })
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?
}

#[tauri::command]
pub async fn search_skills(query: String) -> AppResult<Vec<Skill>> {
    tauri::async_runtime::spawn_blocking(move || {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for agent in adapters::all_adapters() {
            if !agent.has_skills_dir {
                continue;
            }
            for s in scanner::scan_agent(agent) {
                let hay = format!("{} {} {}", s.name, s.description, s.folder).to_lowercase();
                if hay.contains(&q) {
                    out.push(s);
                }
            }
        }
        Ok(out)
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?
}

#[tauri::command]
pub async fn mount_skill(agent_id: String, central_folder: String) -> AppResult<Skill> {
    tauri::async_runtime::spawn_blocking(move || fsops::mount_skill(&agent_id, &central_folder))
        .await
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?
}

#[tauri::command]
pub async fn unmount_skill(agent_id: String, folder: String) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || fsops::unmount_skill(&agent_id, &folder))
        .await
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?
}

#[tauri::command]
pub async fn update_frontmatter(
    agent_id: String,
    folder: String,
    update: FrontmatterUpdate,
) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        fsops::update_frontmatter(&agent_id, &folder, update)
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?
}

// reveal（在 Finder 中显示）由前端直接调用 tauri-plugin-opener 的
// revealItemInDir 实现，无需后端命令。

