use std::collections::HashMap;
use std::path::Path;

use crate::model::LockEntry;

/// 只读解析 ~/.agents/.skill-lock.json，返回 folder -> LockEntry 映射。
/// 解析失败时返回空 map（降级为“来源未知”），不阻塞扫描。
pub fn load_lock() -> HashMap<String, LockEntry> {
    let path = dirs::home_dir()
        .map(|h| h.join(".agents/.skill-lock.json"))
        .unwrap_or_default();
    if !path.is_file() {
        return HashMap::new();
    }
    let Ok(text) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct LockFile {
        #[serde(default)]
        skills: HashMap<String, LockEntry>,
    }
    match serde_json::from_str::<LockFile>(&text) {
        Ok(lock) => lock.skills,
        Err(_) => HashMap::new(),
    }
}

/// 查找某个技能目录在 lock 中的记录：优先精确匹配 folder 名；
/// 若技能路径含子路径（如 skills/tavily/crawl），也尝试匹配末段。
pub fn lookup<'a>(lock: &'a HashMap<String, LockEntry>, folder: &str) -> Option<&'a LockEntry> {
    if let Some(e) = lock.get(folder) {
        return Some(e);
    }
    // 中央仓内的目录可能是符号链接到别处（如项目目录），用 canonical 名再试一次
    let base = Path::new(folder)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(folder);
    lock.get(base)
}
