use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::adapters;
use crate::scanner;

/// 单个技能的使用统计
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UsageStat {
    /// 技能目录名，与 Skill.folder 对应
    pub folder: String,
    pub count: u32,
    /// 最近一次使用时间（RFC3339），近似统计时为 null
    pub last_used: Option<String>,
}

/// 趋势上的单个点（按天聚合）
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UsagePoint {
    /// YYYY-MM-DD
    pub date: String,
    pub count: u32,
}

/// 某个 agent 的使用统计总览
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AgentUsage {
    pub agent_id: String,
    /// 该 agent 是否有可用的会话日志数据源
    pub supported: bool,
    /// 统计精度：exact（工具调用记录）/ approx（文本提及近似）/ none
    pub precision: String,
    pub scanned_files: u32,
    pub total: u32,
    /// 本次统计时间（RFC3339）
    pub last_scan: Option<String>,
    pub stats: Vec<UsageStat>,
    /// 按天聚合的使用趋势（日期升序）
    #[serde(default)]
    pub trend: Vec<UsagePoint>,
    pub note: String,
}

impl AgentUsage {
    fn unsupported(agent_id: &str, note: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            supported: false,
            precision: "none".into(),
            scanned_files: 0,
            total: 0,
            last_scan: Some(now_iso()),
            stats: Vec::new(),
            trend: Vec::new(),
            note: note.to_string(),
        }
    }
}

/// 扫描过程中的累加器：技能计数 + 按天趋势
#[derive(Default, Clone)]
struct Acc {
    counts: HashMap<String, (u32, Option<String>)>,
    days: HashMap<String, u32>,
}

impl Acc {
    fn bump(&mut self, folder: String, ts: Option<String>) {
        if let Some(d) = ts.as_deref().and_then(date_of) {
            *self.days.entry(d).or_insert(0) += 1;
        }
        let entry = self.counts.entry(folder).or_insert((0, None));
        entry.0 += 1;
        if let Some(t) = ts {
            if entry.1.as_deref().map_or(true, |cur| t.as_str() > cur) {
                entry.1 = Some(t);
            }
        }
    }

    fn bump_day(&mut self, date: String) {
        *self.days.entry(date).or_insert(0) += 1;
    }

    fn total(&self) -> u32 {
        self.counts.values().map(|v| v.0).sum()
    }
}

/// 从时间戳字符串取日期：支持 ISO（2026-09-08T…）与 epoch（秒或毫秒）
fn date_of(ts: &str) -> Option<String> {
    let t = ts.trim();
    if let Some(pos) = t.find('T') {
        if pos >= 10 {
            return Some(t[..10].to_string());
        }
    }
    if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
        let n: i64 = t.parse().ok()?;
        // 12 位以上视为毫秒
        let secs = if t.len() >= 12 { n / 1000 } else { n };
        return Some(epoch_to_date(secs));
    }
    None
}

/// epoch 秒 -> YYYY-MM-DD（北京时间 UTC+8）
fn epoch_to_date(secs: i64) -> String {
    let shifted = secs + 8 * 3600;
    let days = shifted.div_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn now_iso() -> String {
    // 不引入 chrono，直接用系统时间的秒级 RFC3339 近似
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs_to_iso(secs)
}

fn secs_to_iso(secs: u64) -> String {
    // 天/时分秒换算（UTC）
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant 的 days_from_civil 逆运算
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as i64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

fn cache_path() -> PathBuf {
    home().join(".skill-manager/usage.json")
}

fn load_cache() -> HashMap<String, AgentUsage> {
    let Ok(text) = std::fs::read_to_string(cache_path()) else {
        return HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_cache(cache: &HashMap<String, AgentUsage>) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(path, json);
    }
}

/// 读取缓存；无缓存时现算一次
pub fn get(agent_id: &str) -> AgentUsage {
    let cache = load_cache();
    if let Some(hit) = cache.get(agent_id) {
        // 有效缓存：有统计结果且有趋势数据；否则（旧格式 / 曾错误算出 0）重算
        if hit.supported && hit.total > 0 && !hit.trend.is_empty() {
            return hit.clone();
        }
        if !hit.supported {
            return hit.clone();
        }
    }
    let computed = compute(agent_id);
    let mut cache = cache;
    cache.insert(agent_id.to_string(), computed.clone());
    save_cache(&cache);
    computed
}

/// 强制重新扫描
pub fn refresh(agent_id: &str) -> AgentUsage {
    let computed = compute(agent_id);
    let mut cache = load_cache();
    cache.insert(agent_id.to_string(), computed.clone());
    save_cache(&cache);
    computed
}

/// 全部 agent（用于统计总览页）——只统计本机实际存在的 agent
pub fn all() -> Vec<AgentUsage> {
    adapters::installed_adapters()
        .into_iter()
        .map(|a| get(&a.id))
        .collect()
}

fn compute(agent_id: &str) -> AgentUsage {
    let Some(agent) = adapters::find(agent_id) else {
        return AgentUsage::unsupported(agent_id, "未知 agent");
    };
    // 候选项：目录名与 frontmatter name 都能命中
    let mut token_map: HashMap<String, String> = HashMap::new();
    for s in scanner::scan_agent(agent) {
        for t in [s.folder.clone(), s.name.clone()] {
            let t = t.trim().to_lowercase();
            if t.len() >= 2 {
                token_map.insert(t, s.folder.clone());
            }
        }
    }
    // 中央仓库技能也要能被识别（各 agent 常通过符号链接引用）
    for s in scanner::scan_agent(
        adapters::find("agents-central").unwrap_or(agent),
    ) {
        for t in [s.folder.clone(), s.name.clone()] {
            let t = t.trim().to_lowercase();
            if t.len() >= 2 {
                token_map.entry(t).or_insert_with(|| s.folder.clone());
            }
        }
    }

    match agent_id {
        "claude" => {
            let (acc, files) = scan_skill_tool(&home().join(".claude/projects"), &token_map);
            finish(agent_id, acc, files, "exact", "按 Claude Code 会话中的 Skill 工具调用记录精确统计")
        }
        "codebuddy" => {
            let (acc, files) = scan_skill_tool(&home().join(".codebuddy/projects"), &token_map);
            if acc.total() > 0 {
                finish(agent_id, acc, files, "exact", "按 CodeBuddy 会话中的 Skill 工具调用记录精确统计")
            } else {
                let (c2, f2) = scan_mentions(&collect_files(&home().join(".codebuddy/projects"), "jsonl"), &token_map, Some(b'$'));
                finish(agent_id, c2, f2, "approx", "未找到结构化调用记录，按会话文本中技能名提及次数近似统计")
            }
        }
        "kimi-code" => {
            let (acc, files) = scan_kimi(&token_map);
            if acc.total() > 0 {
                finish(agent_id, acc, files, "exact", "按 Kimi Code 会话 wire 记录中的 Skill 工具调用精确统计")
            } else {
                let (c2, f2) = scan_mentions(&collect_files(&home().join(".kimi-code/sessions"), "jsonl"), &token_map, Some(b'$'));
                finish(agent_id, c2, f2, "approx", "未找到结构化调用记录，按会话文本中 $技能名 提及次数近似统计")
            }
        }
        "opencode" => match scan_opencode(&token_map) {
            Some((acc, files)) if acc.total() > 0 => finish(
                agent_id,
                acc,
                files,
                "exact",
                "按 OpenCode 数据库 part 表中的 skill 工具调用精确统计（依赖 sqlite3 命令）",
            ),
            Some((acc, files)) => finish(
                agent_id,
                acc,
                files,
                "approx",
                "已读取 OpenCode 数据库，但未发现 skill 工具调用记录",
            ),
            None => AgentUsage::unsupported(
                agent_id,
                "未找到 OpenCode 数据库或系统缺少 sqlite3 命令，无法统计",
            ),
        },
        "codex" => {
            let mut files = collect_files(&home().join(".codex/sessions"), "json");
            files.extend(collect_files(&home().join(".codex/archived_sessions"), "json"));
            for f in ["state_5.sqlite", "thread_history_1.sqlite"] {
                let p = home().join(".codex").join(f);
                if p.is_file() {
                    files.push(p);
                }
            }
            let (acc, n) = scan_mentions(&files, &token_map, Some(b'$'));
            finish(agent_id, acc, n, "approx", "Codex 无结构化技能调用日志，按会话存储中 $技能名 出现次数近似统计；趋势按会话文件修改日期归集（数字可能高于实际调用数）")
        }
        "grok" => {
            let (acc, files) = scan_grok(&token_map);
            finish(agent_id, acc, files, "exact", "按 Grok 会话（chat_history.jsonl）中 assistant 的 tool_calls 读取技能 SKILL.md 记录精确统计；趋势按会话文件修改日期归集")
        }
        "omp" => {
            let (acc, files) = scan_omp_skill_reads(&token_map);
            finish(agent_id, acc, files, "exact", "OMP 无技能调用工具，技能以读取 SKILL.md 方式加载；按会话中读取技能文件精确统计（技能名提及噪声过大，不做近似口径）")
        }
        "openclaw" => AgentUsage::unsupported(
            agent_id,
            "OpenClaw 未找到可解析的会话记录（仅有配置审计与 memory 库），无法统计",
        ),
        other => AgentUsage::unsupported(
            other,
            "该 agent 暂未提供可解析的会话日志，无法统计使用频次",
        ),
    }
}

fn finish(agent_id: &str, acc: Acc, files: u32, precision: &str, note: &str) -> AgentUsage {
    let mut stats: Vec<UsageStat> = acc
        .counts
        .into_iter()
        .map(|(folder, (count, last_used))| UsageStat {
            folder,
            count,
            last_used,
        })
        .collect();
    stats.sort_by(|a, b| b.count.cmp(&a.count).then(a.folder.cmp(&b.folder)));
    let total = stats.iter().map(|s| s.count).sum();
    let mut trend: Vec<UsagePoint> = acc
        .days
        .into_iter()
        .map(|(date, count)| UsagePoint { date, count })
        .collect();
    trend.sort_by(|a, b| a.date.cmp(&b.date));
    AgentUsage {
        agent_id: agent_id.to_string(),
        supported: true,
        precision: precision.to_string(),
        scanned_files: files,
        total,
        last_scan: Some(now_iso()),
        stats,
        trend,
        note: note.to_string(),
    }
}

/// 递归收集指定后缀的文件（目录不存在时返回空）
fn collect_files(root: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return out;
    }
    walk(root, ext, &mut out, 0);
    out
}

fn walk(dir: &Path, ext: &str, out: &mut Vec<PathBuf>, depth: u32) {
    if depth > 6 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, ext, out, depth + 1);
        } else if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            out.push(path);
        }
    }
}

/// 扫描 jsonl 中的 Skill 工具调用：`"name":"Skill","input":{...,"skill":"X"}`
fn scan_skill_tool(root: &Path, token_map: &HashMap<String, String>) -> (Acc, u32) {
    let files = collect_files(root, "jsonl");
    let mut acc = Acc::default();
    for f in &files {
        let Ok(file) = File::open(f) else { continue };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if !line.contains("\"Skill\"") || !line.contains("\"skill\":\"") {
                continue;
            }
            let Some(token) = extract_value(&line, "\"skill\":") else {
                continue;
            };
            // 插件技能形如 plugin:skill，取末段；未安装的技能也计入（以原名记录）
            let token = token.rsplit(':').next().unwrap_or(token).to_lowercase();
            let folder = token_map
                .get(&token)
                .cloned()
                .unwrap_or_else(|| token.clone());
            acc.bump(folder, extract_timestamp(&line).map(|s| s.to_string()));
        }
    }
    (acc, files.len() as u32)
}

/// 扫描 Kimi Code 会话 wire 记录：`"name":"Skill","args":{"skill":"X",...},"time":<ms>`
fn scan_kimi(token_map: &HashMap<String, String>) -> (Acc, u32) {
    let files = collect_files(&home().join(".kimi-code/sessions"), "jsonl");
    let mut acc = Acc::default();
    for f in &files {
        let Ok(file) = File::open(f) else { continue };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if !line.contains("\"name\":\"Skill\"") || !line.contains("\"skill\":\"") {
                continue;
            }
            let Some(token) = extract_value(&line, "\"skill\":") else {
                continue;
            };
            let token = token.rsplit(':').next().unwrap_or(token).to_lowercase();
            let folder = token_map
                .get(&token)
                .cloned()
                .unwrap_or_else(|| token.clone());
            let ts = extract_value(&line, "\"time\":").map(|s| s.to_string());
            acc.bump(folder, ts);
        }
    }
    (acc, files.len() as u32)
}

/// OMP：会话里没有 Skill 工具，技能加载有两种真实信号：
/// 1. custom 事件 `toolName:"read"` 读取 `*/skills/<name>/SKILL.md`
/// 2. custom_message `customType:"skill-prompt"`（技能内容注入，代表一次显式调用）
fn scan_omp_skill_reads(token_map: &HashMap<String, String>) -> (Acc, u32) {
    let files = collect_files(&home().join(".omp/agent/sessions"), "jsonl");
    let mut acc = Acc::default();
    for f in &files {
        let Ok(file) = File::open(f) else { continue };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if !line.contains("SKILL.md") && !line.contains("skill-prompt") {
                continue;
            }
            let ts = extract_timestamp(&line).map(|s| s.to_string());
            if line.contains("\"customType\":\"skill-prompt\"") {
                // `…invoked the "<name>" skill`（行内引号是 JSON 转义的 \"，两种措辞取公共后缀）
                let Some(i) = line.find("invoked the \\\"") else { continue };
                let rest = &line[i + "invoked the \\\"".len()..];
                let Some(end) = rest.find('\\').or_else(|| rest.find('"')).filter(|&e| e > 0)
                else {
                    continue;
                };
                let token = rest[..end].to_lowercase();
                let folder = token_map.get(&token).cloned().unwrap_or(token);
                acc.bump(folder, ts.clone());
                continue;
            }
            if !line.contains("\"toolName\":\"read\"") {
                continue;
            }
            let Some(path) = extract_value(&line, "\"path\":") else {
                continue;
            };
            // 从路径中取技能目录名：…/skills/<name>/SKILL.md
            for name in extract_skill_names(path) {
                let folder = token_map
                    .get(&name.to_lowercase())
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                acc.bump(folder, ts.clone());
            }
        }
    }
    (acc, files.len() as u32)
}

/// Grok：会话 chat_history.jsonl 中 assistant 消息的 tool_calls.arguments
/// 含读取 `…/skills/<name>/SKILL.md` 的记录，即技能加载事件。
fn scan_grok(token_map: &HashMap<String, String>) -> (Acc, u32) {
    let files: Vec<PathBuf> = collect_files(&home().join(".grok/sessions"), "jsonl")
        .into_iter()
        .filter(|f| {
            f.file_name()
                .map(|n| n == std::ffi::OsStr::new("chat_history.jsonl"))
                .unwrap_or(false)
        })
        .collect();
    let mut acc = Acc::default();
    for f in &files {
        let Ok(file) = File::open(f) else { continue };
        // 会话行内无独立时间戳，趋势按文件修改日期归集
        let file_day = std::fs::metadata(f)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| epoch_to_date(d.as_secs() as i64));
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if !line.contains("SKILL.md") {
                continue;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
                continue;
            }
            let Some(tcs) = v.get("tool_calls").and_then(|t| t.as_array()) else {
                continue;
            };
            for tc in tcs {
                let args = tc
                    .get("arguments")
                    .and_then(|a| a.as_str())
                    .or_else(|| {
                        tc.pointer("/function/arguments").and_then(|a| a.as_str())
                    });
                let Some(args) = args else { continue };
                for name in extract_skill_names(args) {
                    let folder = token_map
                        .get(&name.to_lowercase())
                        .cloned()
                        .unwrap_or_else(|| name.clone());
                    acc.bump(folder, None);
                    if let Some(d) = &file_day {
                        acc.bump_day(d.clone());
                    }
                }
            }
        }
    }
    (acc, files.len() as u32)
}

/// 从工具调用参数文本中提取 `…/skills/<name>/SKILL.md` 的技能目录名
fn extract_skill_names(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = args;
    while let Some(pos) = rest.find("SKILL.md") {
        // 路径形如 …/skills/<name>/SKILL.md：技能名在最后两个斜杠之间
        let before = rest[..pos].trim_end_matches('/');
        let seg_start = before.rfind('/').map(|i| i + 1).unwrap_or(0);
        let name = &before[seg_start..];
        let ok = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
        if ok {
            out.push(name.to_string());
        }
        rest = &rest[pos + "SKILL.md".len()..];
    }
    out
}

/// 通过 sqlite3 命令读取 OpenCode 数据库中的 skill 工具调用
/// `part.data` 形如 `{"type":"tool","tool":"skill","state":{"input":{"name":"X"}}}`
fn scan_opencode(token_map: &HashMap<String, String>) -> Option<(Acc, u32)> {
    let db = home().join(".local/share/opencode/opencode.db");
    if !db.is_file() {
        return None;
    }
    // instr 定位 input.name 片段（data 里 output 等大字段可能把它挤到 600 字节之外）
    let sql = "select time_created, substr(data, instr(data, '\"input\":{\"name\":\"'), 200) as d from part where data like '%\"tool\":\"skill\"%'";
    let out = std::process::Command::new("sqlite3")
        .arg("-json")
        .arg(&db)
        .arg(sql)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    #[derive(serde::Deserialize)]
    struct Row {
        time_created: Option<i64>,
        d: Option<String>,
    }
    let rows: Vec<Row> = serde_json::from_slice(&out.stdout).ok()?;
    let mut acc = Acc::default();
    for r in rows {
        let Some(data) = r.d else { continue };
        let Some(name) = extract_value(&data, "\"input\":{\"name\":") else {
            continue;
        };
        // 未安装的技能也计入（以原名记录）
        let token = name.rsplit(':').next().unwrap_or(name).to_lowercase();
        let folder = token_map
            .get(&token)
            .cloned()
            .unwrap_or_else(|| token.clone());
        acc.bump(folder, r.time_created.map(|t| t.to_string()));
    }
    Some((acc, 1))
}

/// 扫描文件中技能名的提及（近似统计；趋势按文件修改日期归集）。
/// prefix = Some(b'$')：只匹配 `$技能名`（Codex 风格显式触发）；
/// prefix = None：匹配任意位置出现的技能名（词边界），噪声更大。
fn scan_mentions(
    files: &[PathBuf],
    token_map: &HashMap<String, String>,
    prefix: Option<u8>,
) -> (Acc, u32) {
    let mut acc = Acc::default();
    let mut scanned = 0u32;
    for f in files {
        let Ok(mut file) = File::open(f) else { continue };
        scanned += 1;
        // 无行内时间戳，用文件修改时间作为该文件的归集日期
        let file_day = std::fs::metadata(f)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| epoch_to_date(d.as_secs() as i64));
        let mut hits: Vec<String> = Vec::new();
        let mut buf = vec![0u8; 1024 * 1024];
        let mut tail = String::new();
        loop {
            let Ok(n) = file.read(&mut buf) else { break };
            if n == 0 {
                break;
            }
            let chunk = String::from_utf8_lossy(&buf[..n]);
            let hay = format!("{tail}{chunk}");
            count_tokens(&hay, token_map, &mut hits, prefix);
            // 保留尾部 64 字符，避免跨块截断
            tail = hay
                .chars()
                .rev()
                .take(64)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
        }
        for folder in hits.drain(..) {
            acc.bump(folder, None);
            if let Some(d) = &file_day {
                acc.bump_day(d.clone());
            }
        }
    }
    (acc, scanned)
}

fn count_tokens(
    text: &str,
    token_map: &HashMap<String, String>,
    hits: &mut Vec<String>,
    prefix: Option<u8>,
) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(p) = prefix {
            if bytes[i] != p {
                i += 1;
                continue;
            }
        } else if !bytes[i].is_ascii_alphanumeric() {
            // 无前缀模式：要求技能名前一个字符是词边界
            if i > 0
                && (bytes[i - 1].is_ascii_alphanumeric()
                    || bytes[i - 1] == b'-'
                    || bytes[i - 1] == b'_'
                    || bytes[i - 1] == b'/')
            {
                i += 1;
                continue;
            }
        }
        let start = i + 1;
        let mut end = start;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-' || bytes[end] == b'_')
        {
            end += 1;
        }
        if end > start {
            let token = text[start..end].to_lowercase();
            if let Some(folder) = token_map.get(&token) {
                hits.push(folder.clone());
            }
        }
        i = end.max(i + 1);
    }
}

/// 提取 `"key": "value"` / `"key":"value"` / `"key":123` 形式的值，
/// 返回字符串切片（数字也按原样返回，由调用方解析）
fn extract_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let idx = line.find(key)? + key.len();
    let rest = line[idx..].trim_start();
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        return Some(&stripped[..end]);
    }
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    Some(&rest[..end])
}

/// 提取行内时间戳并统一为 RFC3339（支持 ISO 字符串与毫秒数字）
fn extract_timestamp(line: &str) -> Option<String> {
    let raw = extract_value(line, "\"timestamp\":")?;
    if raw.chars().all(|c| c.is_ascii_digit()) {
        let ms: u64 = raw.parse().ok()?;
        return Some(secs_to_iso(ms / 1000));
    }
    if raw.len() >= 19 {
        return Some(raw.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_skill_names_regression() {
        // 回归：技能名在倒数第二个斜杠与 SKILL.md 之间（曾错误取到空串）
        let args = r#"{"target_file":"/Users/x/.grok/skills/orchestration/SKILL.md"}"#;
        assert_eq!(extract_skill_names(args), vec!["orchestration".to_string()]);
        // 带行号后缀的路径
        let p2 = "/Users/x/.agents/skills/mysql-query/SKILL.md:17-50";
        assert_eq!(extract_skill_names(p2), vec!["mysql-query".to_string()]);
        // extract_value：key 以引号结尾时 value 不以引号开头（曾误走数字分支返回 None）
        assert_eq!(
            extract_value(r#""args":{"path":"/a/b/SKILL.md"}"#, "\"path\":"),
            Some("/a/b/SKILL.md")
        );
    }

}
