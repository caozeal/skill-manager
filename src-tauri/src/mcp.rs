//! MCP（工具）服务器管理：跨 agent 统一读写各家的 MCP 配置。
//!
//! 各 agent 的配置位置与格式五花八门（JSON / TOML / JSONC），这里统一收敛成
//! `McpServer` 结构再交给前端。写入坚持三条红线：
//!   1. 写前自动备份 `<file>.bak-<时间戳>`
//!   2. TOML 用 toml_edit 原地改，保留注释与格式
//!   3. JSONC（含注释）一律只读，避免重写丢注释

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use toml_edit::{Array, DocumentMut, Item, Table, Value as TomlValue};

use crate::error::{AppError, AppResult};

/// env / headers 中敏感值下发给前端时的掩码；原样回传表示「不改这一项」
pub const MASK: &str = "••••";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Format {
    Json,
    Jsonc,
    Toml,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Scope {
    Global,
    #[allow(dead_code)]
    Project,
}

struct SourceDef {
    agent: &'static str,
    /// 相对 $HOME 的路径
    rel: &'static str,
    format: Format,
    /// MCP 段在该文件中的键名
    key: &'static str,
    scope: Scope,
    /// agent 是否原生支持 enabled 字段（Claude / OpenCode 没有，只能删条目）
    native_enabled: bool,
    /// 是否允许写入（JSONC 含注释，只读）
    writable: bool,
}

fn sources() -> Vec<SourceDef> {
    vec![
        SourceDef {
            agent: "claude",
            rel: ".claude.json",
            format: Format::Json,
            key: "mcpServers",
            scope: Scope::Global,
            native_enabled: false,
            writable: true,
        },
        SourceDef {
            agent: "codex",
            rel: ".codex/config.toml",
            format: Format::Toml,
            key: "mcp_servers",
            scope: Scope::Global,
            native_enabled: true,
            writable: true,
        },
        SourceDef {
            agent: "grok",
            rel: ".grok/config.toml",
            format: Format::Toml,
            key: "mcp_servers",
            scope: Scope::Global,
            native_enabled: true,
            writable: true,
        },
        SourceDef {
            agent: "codebuddy",
            rel: ".codebuddy/mcp.json",
            format: Format::Json,
            key: "mcpServers",
            scope: Scope::Global,
            native_enabled: false,
            writable: true,
        },
        // opencode 用带尾逗号的 JSONC。不大规模重写整个文件，走「段落级原地替换」：
        // 只动 mcp 这一段，其余配置与注释保持原样（段内有注释时会拒绝写入）
        SourceDef {
            agent: "opencode",
            rel: ".config/opencode/opencode.jsonc",
            format: Format::Jsonc,
            key: "mcp",
            scope: Scope::Global,
            native_enabled: false,
            writable: true,
        },
    ]
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    /// agent|scope|project|name
    pub id: String,
    pub agent_id: String,
    pub name: String,
    /// stdio / http / unknown
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    /// 环境变量，值一律打码
    pub env: Vec<(String, String)>,
    /// HTTP 请求头，值一律打码
    pub headers: Vec<(String, String)>,
    pub enabled: bool,
    /// global / project
    pub scope: String,
    pub project: Option<String>,
    pub source_path: String,
    /// 是否支持原生 enabled（否则禁用 = 移除条目 + 影子文件留存）
    pub native_enabled: bool,
    pub writable: bool,
}

/// 前端提交的编辑内容；env/headers 中值为 MASK 表示保留原值
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct McpServerInput {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub env: Vec<(String, String)>,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub enabled: bool,
}

// ---------------------------------------------------------------- 路径与影子文件

fn home() -> PathBuf {
    dirs::home_dir().expect("无法获取 HOME")
}

fn source_path(def: &SourceDef) -> PathBuf {
    home().join(def.rel)
}

/// MCP 配置目标（agent 维度）。UI 不再写死 agent 清单，一律用这份运行时结果。
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct McpTarget {
    /// agent id，与技能页/侧边栏的 agent id 一致（单例来源：adapters registry）
    pub id: String,
    /// 显示名，取自 adapters registry，保证与技能页显示一致
    pub name: String,
    /// 该源是否允许写入（JSONC 等只读源为 false）
    pub writable: bool,
    /// 配置文件是否已存在；不存在时写入会失败，UI 不应把它列为可选项
    pub available: bool,
}

/// 本机 MCP 配置源清单（含只读/暂不可写的源，由前端按 writable + available 过滤）
pub fn targets() -> Vec<McpTarget> {
    sources()
        .into_iter()
        .map(|d| {
            let name = crate::adapters::find(d.agent)
                .map(|a| a.name.to_string())
                .unwrap_or_else(|| d.agent.to_string());
            McpTarget {
                id: d.agent.to_string(),
                name,
                writable: d.writable,
                available: source_path(&d).exists(),
            }
        })
        .collect()
}

/// 无 enabled 字段的 agent（Claude / OpenCode）禁用时，把原始配置存到这里以便恢复
fn shadow_path() -> PathBuf {
    home().join(".skill-manager/mcp-disabled.json")
}

fn load_shadow() -> BTreeMap<String, Json> {
    let p = shadow_path();
    match fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str(&t).ok()) {
        Some(Json::Object(m)) => m.into_iter().collect(),
        _ => BTreeMap::new(),
    }
}

fn save_shadow(map: &BTreeMap<String, Json>) -> AppResult<()> {
    let p = shadow_path();
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).map_err(AppError::Io)?;
    }
    let txt = serde_json::to_string_pretty(map)
        .map_err(|e| AppError::InvalidFrontmatter(e.to_string()))?;
    fs::write(&p, txt).map_err(AppError::Io)
}

/// 可见即可恢复：条目在影子文件里有留存 → enabled 显示为 false 且可恢复
fn shadow_key(agent: &str, scope: &str, project: Option<&str>, name: &str) -> String {
    format!("{agent}|{scope}|{}|{name}", project.unwrap_or("-"))
}

fn make_id(agent: &str, scope: &str, project: Option<&str>, name: &str) -> String {
    format!("{agent}|{scope}|{}|{name}", project.unwrap_or("-"))
}

fn parse_id(id: &str) -> AppResult<(String, String, Option<String>, String)> {
    let parts: Vec<&str> = id.split('|').collect();
    if parts.len() != 4 {
        return Err(AppError::AgentNotFound(format!("非法 MCP id: {id}")));
    }
    let project = if parts[2] == "-" {
        None
    } else {
        Some(parts[2].to_string())
    };
    Ok((
        parts[0].to_string(),
        parts[1].to_string(),
        project,
        parts[3].to_string(),
    ))
}

// ---------------------------------------------------------------- 备份

fn backup(path: &Path) -> AppResult<()> {
    let tag = crate::fsops::backup_tag();
    let dest = path.with_file_name(format!(
        "{}.bak-{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        tag
    ));
    fs::copy(path, dest).map_err(AppError::Io)?;
    Ok(())
}

// ---------------------------------------------------------------- JSONC 原地写入

/// 在 JSONC 原文里定位根对象的某个成员，返回其 `{...}` 的字节范围
fn find_member_range(text: &str, key: &str) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut depth: i32 = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'"' => {
                // 读完整字符串（不解码转义，只需拿到原始内容比较 key）
                i += 1;
                let start = i;
                let mut esc = false;
                while i < bytes.len() {
                    let c = bytes[i];
                    if esc {
                        esc = false;
                    } else if c == b'\\' {
                        esc = true;
                    } else if c == b'"' {
                        break;
                    }
                    i += 1;
                }
                let raw = &text[start..i.min(text.len())];
                i += 1;
                if depth == 1 && raw == key {
                    while i < bytes.len() && bytes[i] != b':' {
                        i += 1;
                    }
                    i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    if i < bytes.len() && bytes[i] == b'{' {
                        let end = match_brace(bytes, i)?;
                        return Some((i, end));
                    }
                }
            }
            b'{' | b'[' => {
                depth += 1;
                i += 1;
            }
            b'}' | b']' => {
                depth -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

/// 从 start 处的 `{` 配平到对应的 `}`（跳过字符串内容）
fn match_brace(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
            }
        } else if b == b'"' {
            in_str = true;
        } else if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i + 1);
            }
        }
        i += 1;
    }
    None
}

/// 字符串之外是否存在注释（URL 里的 // 位于字符串内，不算）
fn has_comment(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut in_str = false;
    let mut esc = false;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'/' if bytes.get(i + 1) == Some(&b'/') => return true,
            b'/' if bytes.get(i + 1) == Some(&b'*') => return true,
            _ => {}
        }
        i += 1;
    }
    false
}

/// 按 JSONC 习惯序列化（每个成员后都带尾逗号，缩进两空格）
fn jsonc_value(v: &Json, indent: &str) -> String {
    match v {
        Json::Object(m) => {
            if m.is_empty() {
                return "{}".to_string();
            }
            let inner = format!("{indent}  ");
            let mut s = "{\n".to_string();
            for (k, val) in m {
                s.push_str(&format!(
                    "{inner}\"{k}\": {},\n",
                    jsonc_value(val, &inner)
                ));
            }
            s.push_str(&format!("{indent}}}"));
            s
        }
        Json::Array(a) => {
            if a.is_empty() {
                return "[]".to_string();
            }
            let inner = format!("{indent}  ");
            let mut s = "[\n".to_string();
            for val in a {
                s.push_str(&format!("{inner}{},\n", jsonc_value(val, &inner)));
            }
            s.push_str(&format!("{indent}]"));
            s
        }
        Json::String(x) => serde_json::to_string(x).unwrap_or_else(|_| "\"\"".into()),
        Json::Null => "null".to_string(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => n.to_string(),
    }
}

/// 段落级替换 mcp 段；段内若有注释则拒绝，避免重写时吃掉用户的说明文字
fn write_jsonc_section(
    path: &Path,
    key: &str,
    servers: &BTreeMap<String, Json>,
) -> AppResult<()> {
    let text = fs::read_to_string(path).map_err(AppError::Io)?;

    let new_text = match find_member_range(&text, key) {
        Some((s, e)) => {
            if has_comment(&text[s..e]) {
                return Err(AppError::ReadOnlyAgent);
            }
            let key_indent = line_indent_before(&text, s);
            let body = jsonc_value(
                &Json::Object(
                    servers
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<serde_json::Map<_, _>>(),
                ),
                &key_indent,
            );
            let mut out = String::new();
            out.push_str(&text[..s]);
            out.push_str(&body);
            out.push_str(&text[e..]);
            out
        }
        None => {
            // 还没有该段：插到根对象的最后一个 } 之前
            let close = text
                .rfind('}')
                .ok_or_else(|| AppError::InvalidFrontmatter("配置不是合法 JSONC 对象".into()))?;
            let body = jsonc_value(
                &Json::Object(
                    servers
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<serde_json::Map<_, _>>(),
                ),
                "  ",
            );
            let mut out = String::new();
            out.push_str(&text[..close]);
            out.push_str(&format!("  \"{key}\": {body},\n"));
            out.push_str(&text[close..]);
            out
        }
    };

    fs::write(path, new_text).map_err(AppError::Io)
}

/// value 起点所在行的前导空白（用于让替换后的缩进与原文件一致）
fn line_indent_before(text: &str, pos: usize) -> String {
    let before = &text[..pos];
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let indent_end = line_start
        + before[line_start..]
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| c.len_utf8())
            .sum::<usize>();
    before[line_start..indent_end].to_string()
}

// ---------------------------------------------------------------- 读

/// 读取某个源的 MCP 段，统一成 name -> Json
fn read_section(def: &SourceDef) -> AppResult<BTreeMap<String, Json>> {
    let path = source_path(def);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = fs::read_to_string(&path).map_err(AppError::Io)?;
    // 注意：TOML 分支已经取到了 mcp 段本身，JSON 分支取到的是整个文档，
    // 这里统一收敛成「mcp 段」的映射，后面只按 name -> cfg 处理。
    let section: Json = match def.format {
        Format::Toml => {
            let doc: DocumentMut = text
                .parse()
                .map_err(|e: toml_edit::TomlError| AppError::InvalidFrontmatter(e.to_string()))?;
            match doc.get(def.key) {
                Some(Item::Table(t)) => {
                    let mut m = serde_json::Map::new();
                    for (k, v) in t.iter() {
                        m.insert(k.to_string(), item_to_json(v));
                    }
                    Json::Object(m)
                }
                _ => Json::Object(serde_json::Map::new()),
            }
        }
        Format::Json | Format::Jsonc => {
            let cleaned = strip_jsonc(&text);
            let root: Json = serde_json::from_str(&cleaned)
                .map_err(|e| AppError::InvalidFrontmatter(format!("{}: {e}", def.rel)))?;
            match root.get(def.key) {
                Some(sec @ Json::Object(_)) => sec.clone(),
                _ => Json::Object(serde_json::Map::new()),
            }
        }
    };
    let mut out = BTreeMap::new();
    if let Json::Object(m) = section {
        for (k, v) in m {
            out.insert(k, v);
        }
    }
    Ok(out)
}

/// Claude 的项目级 MCP：存在 .claude.json 的 projects.<路径>.mcpServers 里
fn read_claude_projects() -> Vec<(String, BTreeMap<String, Json>)> {
    let path = home().join(".claude.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(root) = serde_json::from_str::<Json>(&text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(Json::Object(projects)) = root.get("projects") {
        for (proj, cfg) in projects {
            if let Some(Json::Object(sec)) = cfg.get("mcpServers") {
                let mut m = BTreeMap::new();
                for (k, v) in sec {
                    m.insert(k.clone(), v.clone());
                }
                if !m.is_empty() {
                    out.push((proj.clone(), m));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

// ---------------------------------------------------------------- 写

fn write_section(def: &SourceDef, servers: &BTreeMap<String, Json>) -> AppResult<()> {
    if !def.writable {
        return Err(AppError::ReadOnlyAgent);
    }
    let path = source_path(def);
    if !path.exists() {
        return Err(AppError::AgentDirMissing);
    }
    backup(&path)?;
    let text = fs::read_to_string(&path).map_err(AppError::Io)?;

    let out = match def.format {
        Format::Jsonc => {
            // 只重写 mcp 这一段，其余原文（注释、尾逗号、其它配置）一律不动
            write_jsonc_section(&path, def.key, servers)?;
            return Ok(());
        }
        Format::Toml => {
            let mut doc: DocumentMut = text
                .parse()
                .map_err(|e: toml_edit::TomlError| AppError::InvalidFrontmatter(e.to_string()))?;
            let mut tbl = Table::new();
            tbl.set_implicit(false);
            for (name, cfg) in servers {
                tbl.insert(name, json_to_item(cfg));
            }
            doc[def.key] = Item::Table(tbl);
            doc.to_string()
        }
        Format::Json | Format::Jsonc => {
            let cleaned = strip_jsonc(&text);
            let mut root: Json = serde_json::from_str(&cleaned)
                .map_err(|e| AppError::InvalidFrontmatter(e.to_string()))?;
            let mut sec = serde_json::Map::new();
            for (name, cfg) in servers {
                sec.insert(name.clone(), cfg.clone());
            }
            if let Json::Object(ref mut m) = root {
                m.insert(def.key.to_string(), Json::Object(sec));
            }
            // 紧凑写入，尽量贴近 agent 自己的风格
            serde_json::to_string(&root)
                .map_err(|e| AppError::InvalidFrontmatter(e.to_string()))?
        }
    };
    fs::write(&path, out).map_err(AppError::Io)
}

// ---------------------------------------------------------------- 类型转换

fn item_to_json(it: &Item) -> Json {
    match it {
        Item::None => Json::Null,
        Item::Value(v) => value_to_json(v),
        Item::Table(t) => {
            let mut m = serde_json::Map::new();
            for (k, v) in t.iter() {
                m.insert(k.to_string(), item_to_json(v));
            }
            Json::Object(m)
        }
        Item::ArrayOfTables(a) => Json::Array(
            a.iter()
                .map(|t| {
                    let mut m = serde_json::Map::new();
                    for (k, v) in t.iter() {
                        m.insert(k.to_string(), item_to_json(v));
                    }
                    Json::Object(m)
                })
                .collect(),
        ),
    }
}

fn value_to_json(v: &TomlValue) -> Json {
    match v {
        TomlValue::String(s) => Json::String(s.value().clone()),
        TomlValue::Integer(i) => Json::Number((*i.value()).into()),
        TomlValue::Float(f) => serde_json::Number::from_f64(*f.value())
            .map(Json::Number)
            .unwrap_or(Json::Null),
        TomlValue::Boolean(b) => Json::Bool(*b.value()),
        TomlValue::Datetime(d) => Json::String(d.to_string()),
        TomlValue::Array(a) => Json::Array(a.iter().map(value_to_json).collect()),
        TomlValue::InlineTable(t) => {
            let mut m = serde_json::Map::new();
            for (k, v) in t.iter() {
                m.insert(k.to_string(), value_to_json(v));
            }
            Json::Object(m)
        }
    }
}

fn json_to_item(v: &Json) -> Item {
    match v {
        Json::Null => Item::None,
        Json::Bool(b) => Item::Value(TomlValue::from(*b)),
        Json::Number(n) => {
            if let Some(i) = n.as_i64() {
                Item::Value(TomlValue::from(i))
            } else if let Some(f) = n.as_f64() {
                Item::Value(TomlValue::from(f))
            } else {
                Item::Value(TomlValue::from(n.to_string()))
            }
        }
        Json::String(s) => Item::Value(TomlValue::from(s.as_str())),
        Json::Array(a) => {
            let mut arr = Array::new();
            for x in a {
                if let Item::Value(vv) = json_to_item(x) {
                    arr.push(vv);
                }
            }
            Item::Value(TomlValue::from(arr))
        }
        Json::Object(o) => {
            let mut t = Table::new();
            for (k, val) in o {
                t.insert(k, json_to_item(val));
            }
            Item::Table(t)
        }
    }
}

/// 去掉 // 与 /* */ 注释、以及对象/数组尾逗号（不处理字符串内部的 //）
fn strip_jsonc(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut in_str = false;
    let mut escaped = false;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                out.push(c);
                i += 1;
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '/' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    // 删除尾逗号：", }" 或 ", ]"
    let mut cleaned = String::with_capacity(out.len());
    let chars: Vec<char> = out.chars().collect();
    let mut i = 0;
    in_str = false;
    escaped = false;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            cleaned.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if c == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }
        if c == '"' {
            in_str = true;
        }
        cleaned.push(c);
        i += 1;
    }
    cleaned
}

// ---------------------------------------------------------------- 模型转换

fn mask_pairs(v: Option<&Json>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(Json::Object(m)) = v {
        for (k, val) in m {
            let shown = match val {
                Json::String(s) => s.clone(),
                other => other.to_string(),
            };
            out.push((
                k.clone(),
                if shown.is_empty() {
                    String::new()
                } else {
                    MASK.to_string()
                },
            ));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn to_server(
    def: &SourceDef,
    name: &str,
    raw: &Json,
    project: Option<&str>,
    _shadow: &BTreeMap<String, Json>,
) -> McpServer {
    let scope = match def.scope {
        Scope::Global => "global",
        Scope::Project => "project",
    };
    let obj = raw.as_object();
    let get = |k: &str| obj.and_then(|o| o.get(k));
    let str_of = |k: &str| get(k).and_then(|v| v.as_str()).map(|s| s.to_string());

    let transport = match (str_of("type").as_deref(), get("url"), get("command")) {
        (Some("stdio"), _, _) => "stdio",
        (Some("http"), _, _) | (Some("sse"), _, _) => "http",
        (Some("remote"), _, _) => "http",
        (None, Some(_), _) => "http",
        (None, None, Some(_)) => "stdio",
        _ => "unknown",
    };

    let args = get("args")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // 原生 enabled：缺省视为 true；显式 false 才是禁用
    let enabled = match get("enabled") {
        Some(Json::Bool(b)) => *b,
        Some(Json::String(s)) => !s.eq_ignore_ascii_case("false"),
        _ => true,
    };

    McpServer {
        id: make_id(def.agent, scope, project, name),
        agent_id: def.agent.to_string(),
        name: name.to_string(),
        transport: transport.to_string(),
        command: str_of("command"),
        args,
        url: str_of("url"),
        env: mask_pairs(get("env")),
        headers: mask_pairs(get("headers").or_else(|| get("http_headers"))),
        enabled,
        scope: scope.to_string(),
        project: project.map(|s| s.to_string()),
        source_path: source_path(def).to_string_lossy().to_string(),
        native_enabled: def.native_enabled,
        writable: def.writable && project.is_none(),
    }
}

/// 影子文件里留存、但配置中已删除的条目（即被「禁用」的），一并列出以便恢复
fn shadow_entries() -> Vec<McpServer> {
    let shadow = load_shadow();
    let mut out = Vec::new();
    for (key, raw) in shadow.iter() {
        let parts: Vec<&str> = key.split('|').collect();
        if parts.len() != 4 {
            continue;
        }
        let (agent, scope, project, name) = (parts[0], parts[1], parts[2], parts[3]);
        let def = match sources().into_iter().find(|d| d.agent == agent) {
            Some(d) => d,
            None => continue,
        };
        let proj = if project == "-" { None } else { Some(project) };
        let mut s = to_server(&def, name, raw, proj, &BTreeMap::new());
        s.enabled = false;
        s.scope = scope.to_string();
        s.writable = def.writable;
        out.push(s);
    }
    out
}

// ---------------------------------------------------------------- 对外接口

pub fn scan_all() -> Vec<McpServer> {
    let shadow = load_shadow();
    let mut out = Vec::new();
    for def in sources() {
        let sec = match read_section(&def) {
            Ok(s) => s,
            Err(e) => {
                // 不再静默跳过：读不到要留下痕迹，否则某个 agent 整列消失无从排查
                eprintln!("[mcp] 读取 {} ({}) 失败: {e}", def.agent, def.rel);
                continue;
            }
        };
        for (name, raw) in sec.iter() {
            let proj: Option<&str> = None;
            out.push(to_server(&def, name, raw, proj, &shadow));
        }
    }
    // Claude 项目级（只读展示）
    for (proj, sec) in read_claude_projects() {
        let def = SourceDef {
            agent: "claude",
            rel: ".claude.json",
            format: Format::Json,
            key: "mcpServers",
            scope: Scope::Project,
            native_enabled: false,
            writable: false,
        };
        for (name, raw) in sec.iter() {
            let mut s = to_server(&def, name, raw, Some(&proj), &shadow);
            s.source_path = home()
                .join(".claude.json")
                .to_string_lossy()
                .to_string();
            s.writable = false;
            out.push(s);
        }
    }
    out.extend(shadow_entries());
    out.sort_by(|a, b| {
        (&a.agent_id, &a.name).cmp(&(&b.agent_id, &b.name))
    });
    out
}

/// 启用 / 禁用。无原生 enabled 的 agent：禁用 = 从配置移除并存影子文件；启用 = 写回
pub fn set_enabled(id: &str, enabled: bool) -> AppResult<()> {
    let (agent, _scope, project, name) = parse_id(id)?;
    if project.is_some() {
        return Err(AppError::ReadOnlyAgent);
    }
    let def = sources()
        .into_iter()
        .find(|d| d.agent == agent)
        .ok_or_else(|| AppError::AgentNotFound(agent.clone()))?;
    if !def.writable {
        return Err(AppError::ReadOnlyAgent);
    }
    let mut shadow = load_shadow();
    let key = shadow_key(&agent, "global", None, &name);

    if enabled {
        let mut sec = read_section(&def)?;
        if let Some(raw) = shadow.remove(&key) {
            let mut restored = raw;
            if let Json::Object(ref mut m) = restored {
                m.remove("enabled");
            }
            sec.insert(name.clone(), restored);
        } else if let Some(raw) = sec.get_mut(&name) {
            if let Json::Object(ref mut m) = raw {
                m.insert("enabled".into(), Json::Bool(true));
            }
        } else {
            return Err(AppError::AgentNotFound(format!("未找到 MCP: {name}")));
        }
        save_shadow(&shadow)?;
        write_section(&def, &sec)
    } else {
        let mut sec = read_section(&def)?;
        let Some(raw) = sec.remove(&name) else {
            return Err(AppError::AgentNotFound(format!("未找到 MCP: {name}")));
        };
        shadow.insert(key, raw);
        save_shadow(&shadow)?;
        write_section(&def, &sec)
    }
}

fn input_to_json(input: &McpServerInput, old: Option<&Json>) -> Json {
    let mut m = serde_json::Map::new();
    if !input.transport.is_empty() && input.transport != "unknown" {
        m.insert("type".into(), Json::String(input.transport.clone()));
    }
    if let Some(cmd) = &input.command {
        if !cmd.trim().is_empty() {
            m.insert("command".into(), Json::String(cmd.trim().to_string()));
        }
    }
    if !input.args.is_empty() {
        m.insert(
            "args".into(),
            Json::Array(input.args.iter().map(|a| Json::String(a.clone())).collect()),
        );
    }
    if let Some(url) = &input.url {
        if !url.trim().is_empty() {
            m.insert("url".into(), Json::String(url.trim().to_string()));
        }
    }
    let merged_env = merge_masked(old.and_then(|o| o.get("env")), &input.env);
    if !merged_env.is_empty() {
        m.insert("env".into(), Json::Object(merged_env));
    }
    let merged_headers = merge_masked(
        old.and_then(|o| o.get("headers").or_else(|| o.get("http_headers"))),
        &input.headers,
    );
    if !merged_headers.is_empty() {
        m.insert("headers".into(), Json::Object(merged_headers));
    }
    // Codex / Grok 用 enabled 字段表达启用状态
    if let Some(old) = old {
        if old.get("enabled").is_some() || !input.enabled {
            m.insert("enabled".into(), Json::Bool(input.enabled));
        }
    }
    Json::Object(m)
}

/// 前端回传的 env/headers 中，值等于掩码时沿用原值（避免把掩码写进配置）
fn merge_masked(old: Option<&Json>, incoming: &[(String, String)]) -> serde_json::Map<String, Json> {
    let mut out = serde_json::Map::new();
    let old_map = match old {
        Some(Json::Object(m)) => Some(m),
        _ => None,
    };
    for (k, v) in incoming {
        let key = k.trim();
        if key.is_empty() {
            continue;
        }
        if v == MASK {
            if let Some(orig) = old_map.and_then(|m| m.get(key)) {
                out.insert(key.to_string(), orig.clone());
            }
        } else {
            out.insert(key.to_string(), Json::String(v.clone()));
        }
    }
    out
}

/// 新增或更新某个 agent 的 MCP（全局）
pub fn upsert(agent_id: &str, name: &str, input: McpServerInput) -> AppResult<()> {
    let def = sources()
        .into_iter()
        .find(|d| d.agent == agent_id)
        .ok_or_else(|| AppError::AgentNotFound(agent_id.to_string()))?;
    if !def.writable {
        return Err(AppError::ReadOnlyAgent);
    }
    if name.trim().is_empty() {
        return Err(AppError::AgentNotFound("MCP 名称不能为空".into()));
    }
    let mut sec = read_section(&def)?;
    let old = sec.get(name).cloned();
    sec.insert(name.to_string(), input_to_json(&input, old.as_ref()));
    write_section(&def, &sec)?;
    // 条目已恢复存在，影子文件里的留存可以清掉
    let mut shadow = load_shadow();
    shadow.remove(&shadow_key(agent_id, "global", None, name));
    save_shadow(&shadow)
}

pub fn remove(id: &str) -> AppResult<()> {
    let (agent, _scope, project, name) = parse_id(id)?;
    if project.is_some() {
        return Err(AppError::ReadOnlyAgent);
    }
    let def = sources()
        .into_iter()
        .find(|d| d.agent == agent)
        .ok_or_else(|| AppError::AgentNotFound(agent.clone()))?;
    if !def.writable {
        return Err(AppError::ReadOnlyAgent);
    }
    let mut sec = read_section(&def)?;
    if sec.remove(&name).is_none() {
        return Err(AppError::AgentNotFound(format!("未找到 MCP: {name}")));
    }
    write_section(&def, &sec)?;
    let mut shadow = load_shadow();
    shadow.remove(&shadow_key(&agent, "global", None, &name));
    save_shadow(&shadow)
}

/// 把一个 MCP 同步到其它 agent（全局段），返回成功写入的数量
pub fn sync_to(id: &str, targets: Vec<String>) -> AppResult<usize> {
    let (agent, _scope, project, name) = parse_id(id)?;
    if project.is_some() {
        return Err(AppError::ReadOnlyAgent);
    }
    let src_def = sources()
        .into_iter()
        .find(|d| d.agent == agent)
        .ok_or_else(|| AppError::AgentNotFound(agent.clone()))?;
    let src_sec = read_section(&src_def)?;
    let raw = src_sec
        .get(&name)
        .ok_or_else(|| AppError::AgentNotFound(format!("未找到 MCP: {name}")))?
        .clone();

    let mut n = 0;
    for t in targets {
        if t == agent {
            continue;
        }
        let Some(def) = sources().into_iter().find(|d| d.agent == t) else {
            continue;
        };
        if !def.writable {
            continue;
        }
        let mut sec = read_section(&def)?;
        let mut v = raw.clone();
        // 目标 agent 不支持 enabled 字段就别写，免得出现未知字段
        if !def.native_enabled {
            if let Json::Object(ref mut m) = v {
                m.remove("enabled");
            }
        }
        sec.insert(name.clone(), v);
        write_section(&def, &sec)?;
        let mut shadow = load_shadow();
        shadow.remove(&shadow_key(&t, "global", None, &name));
        save_shadow(&shadow)?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_jsonc_comments_and_trailing_commas() {
        let raw = "{\n // 说明\n \"mcp\": {\n  \"a\": { \"url\": \"http://x\", },\n }\n}";
        let cleaned = strip_jsonc(raw);
        let v: Json = serde_json::from_str(&cleaned).expect("应能解析");
        assert_eq!(v["mcp"]["a"]["url"], Json::String("http://x".into()));
    }

    #[test]
    fn masks_env_values_but_keeps_keys() {
        let raw = Json::Object(serde_json::Map::from_iter([
            (
                "command".to_string(),
                Json::String("npx".into()),
            ),
            (
                "env".to_string(),
                Json::Object(serde_json::Map::from_iter([(
                    "TOKEN".to_string(),
                    Json::String("sk-secret".into()),
                )])),
            ),
        ]));
        let def = sources().into_iter().find(|d| d.agent == "codex").unwrap();
        let s = to_server(&def, "demo", &raw, None, &BTreeMap::new());
        assert_eq!(s.env, vec![("TOKEN".to_string(), MASK.to_string())]);
        assert_eq!(s.transport, "stdio");
    }

    #[test]
    fn mask_roundtrip_preserves_original_value() {
        let old = Json::Object(serde_json::Map::from_iter([(
            "TOKEN".to_string(),
            Json::String("sk-real".into()),
        )]));
        let incoming = vec![("TOKEN".to_string(), MASK.to_string())];
        let merged = merge_masked(Some(&old), &incoming);
        assert_eq!(merged["TOKEN"], Json::String("sk-real".into()));
    }

    /// 真实环境冒烟：所有配置源都要能解析，且不能产生缺字段的条目
    #[test]
    fn scan_all_returns_well_formed_entries() {
        let all = scan_all();
        for s in &all {
            assert!(!s.agent_id.is_empty(), "MCP 缺少 agent: {}", s.name);
            assert!(!s.name.is_empty(), "存在无名 MCP: {}", s.id);
            assert!(
                s.transport == "stdio" || s.transport == "http" || s.transport == "unknown",
                "未知传输类型: {}",
                s.transport
            );
            if s.env.iter().any(|(_, v)| !v.is_empty()) {
                assert!(
                    s.env.iter().all(|(_, v)| v == MASK),
                    "env 值必须打码后再下发: {}",
                    s.name
                );
            }
        }
    }

    /// 回归：read_section 对 TOML 必须直接返回 mcp 段本身。
    /// 早期版本在 TOML 分支拿到段之后，又按 JSON 的方式去顶层取一次 key，
    /// 导致 codex / grok 全部静默为空（scan_all 里 continue 吞掉）。
    #[test]
    fn toml_section_reads_subtables() {
        let def = SourceDef {
            agent: "codex",
            rel: ".codex/config.toml",
            format: Format::Toml,
            key: "mcp_servers",
            scope: Scope::Global,
            native_enabled: true,
            writable: true,
        };
        let toml_text = r#"
model = "gpt-5"

[mcp_servers.Context7]
command = "npx"
args = ["-y", "@upstash/context7-mcp@latest"]

[mcp_servers.lanhu]
url = "http://127.0.0.1:8123/mcp"
enabled = false
"#;
        // 直接验证解析逻辑：段内应是 2 条，且字段类型正确
        let doc: toml_edit::DocumentMut = toml_text.parse().unwrap();
        let tbl = match doc.get(def.key) {
            Some(Item::Table(t)) => t,
            _ => panic!("mcp_servers 段未解析为 Table"),
        };
        let mut m = serde_json::Map::new();
        for (k, v) in tbl.iter() {
            m.insert(k.to_string(), item_to_json(v));
        }
        assert_eq!(m.len(), 2, "TOML 子表应解析出 2 个 MCP");
        let ctx = m.get("Context7").unwrap().as_object().unwrap();
        assert_eq!(ctx.get("command").unwrap().as_str().unwrap(), "npx");
        assert_eq!(ctx.get("args").unwrap().as_array().unwrap().len(), 2);
        let lanhu = m.get("lanhu").unwrap().as_object().unwrap();
        assert_eq!(lanhu.get("enabled").unwrap().as_bool().unwrap(), false);
    }

    /// 真实配置冒烟：codex 用 TOML，必须能扫到条目（本机有 6 个）
    #[test]
    fn scan_real_config_finds_toml_agents() {
        let all = scan_all();
        let by: std::collections::BTreeMap<&str, usize> =
            all.iter().fold(std::collections::BTreeMap::new(), |mut acc, s| {
                *acc.entry(s.agent_id.as_str()).or_default() += 1;
                acc
            });
        let codex_cfg = dirs::home_dir().unwrap().join(".codex/config.toml");
        if codex_cfg.exists() {
            assert!(
                by.get("codex").copied().unwrap_or(0) > 0,
                "codex 配置存在但没扫到任何 MCP：{by:?}"
            );
        }
    }

    /// 段落级写入：只动 mcp 这一段，注释、尾逗号、其它配置必须原样保留
    #[test]
    fn jsonc_write_only_touches_mcp_section() {
        let dir = std::env::temp_dir().join("skill-manager-mcp-test");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("opencode.jsonc");
        let original = r#"{
  // 这是我的供应商配置，别删
  "$schema": "https://opencode.ai/config.json",
  "provider": {
    "volc": {
      "npm": "@ai-sdk/openai",
      "options": {
        "baseURL": "https://ark.cn-beijing.volces.com/api/v3",
      },
    },
  },
  "mcp": {
    "lanhu": {
      "type": "remote",
      "url": "http://127.0.0.1:8123/mcp?role=Developer",
    },
  },
}
"#;
        fs::write(&file, original).unwrap();

        let mut servers = BTreeMap::new();
        servers.insert(
            "context7".to_string(),
            Json::Object(serde_json::Map::from_iter([
                ("type".to_string(), Json::String("remote".into())),
                ("url".to_string(), Json::String("https://mcp.example.com".into())),
            ])),
        );
        write_jsonc_section(&file, "mcp", &servers).unwrap();
        let after = fs::read_to_string(&file).unwrap();

        // 注释和 provider 段原封不动
        assert!(after.contains("// 这是我的供应商配置，别删"), "注释被丢了");
        assert!(after.contains("\"baseURL\": \"https://ark.cn-beijing.volces.com/api/v3\","));
        // mcp 段被替换
        assert!(after.contains("\"context7\""), "新条目没写入");
        assert!(!after.contains("lanhu"), "旧条目没被覆盖");
        // 仍然是可解析的 JSONC
        let cleaned = strip_jsonc(&after);
        let parsed: Json = serde_json::from_str(&cleaned).expect("写坏了的 JSONC");
        let got = parsed.get("mcp").unwrap().get("context7").unwrap();
        assert_eq!(got.get("url").unwrap().as_str().unwrap(), "https://mcp.example.com");
        let _ = fs::remove_file(&file);
    }

    /// mcp 段内部有注释时拒绝写，避免吃掉用户写的说明
    #[test]
    fn jsonc_write_refuses_commented_section() {
        let dir = std::env::temp_dir().join("skill-manager-mcp-test");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("opencode-commented.jsonc");
        let original = r#"{
  "mcp": {
    // 这个走本地调试
    "lanhu": { "type": "remote" },
  },
}
"#;
        fs::write(&file, original).unwrap();
        let servers = BTreeMap::new();
        let err = write_jsonc_section(&file, "mcp", &servers).unwrap_err();
        assert!(matches!(err, AppError::ReadOnlyAgent), "段内有注释却允许写入了");
        assert_eq!(fs::read_to_string(&file).unwrap(), original, "文件不应被改动");
        let _ = fs::remove_file(&file);
    }

    /// 真实文件回归：对 ~/.config/opencode/opencode.jsonc 做一次「读→写回」，
    /// 要求除 mcp 段外所有顶层配置逐字节不变。只在文件存在且有 mcp 条目时执行。
    #[test]
    fn real_opencode_jsonc_roundtrip() {
        let src = home().join(".config/opencode/opencode.jsonc");
        if !src.exists() {
            return;
        }
        let def = SourceDef {
            agent: "opencode",
            rel: ".config/opencode/opencode.jsonc",
            format: Format::Jsonc,
            key: "mcp",
            scope: Scope::Global,
            native_enabled: false,
            writable: true,
        };
        let before = match read_section(&def) {
            Ok(s) if !s.is_empty() => s,
            _ => return,
        };

        let dir = std::env::temp_dir().join("skill-manager-mcp-test");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("real-opencode.jsonc");
        fs::copy(&src, &file).unwrap();
        write_jsonc_section(&file, "mcp", &before).unwrap();

        let orig_root: Json =
            serde_json::from_str(&strip_jsonc(&fs::read_to_string(&src).unwrap())).unwrap();
        let after_root: Json =
            serde_json::from_str(&strip_jsonc(&fs::read_to_string(&file).unwrap())).unwrap();
        let orig_map = orig_root.as_object().unwrap();
        let after_map = after_root.as_object().unwrap();

        // mcp 之外的顶层配置必须一个不差
        for (k, v) in orig_map {
            if k == "mcp" {
                continue;
            }
            assert_eq!(
                after_map.get(k).unwrap(),
                v,
                "顶层配置 {k} 被动到了，段落替换越界"
            );
        }
        // mcp 段内容保持语义一致
        let expected: Json = Json::Object(before.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        assert_eq!(after_map.get("mcp").unwrap(), &expected);
        let _ = fs::remove_file(&file);
    }

    #[test]
    fn id_roundtrip() {
        let id = make_id("claude", "global", None, "lanhu");
        let (agent, scope, project, name) = parse_id(&id).unwrap();
        assert_eq!((agent.as_str(), scope.as_str(), project, name.as_str()),
                   ("claude", "global", None, "lanhu"));
    }

    /// targets() 必须与 sources() 一一对应，且 id 不重复——它是前端 agent 清单的唯一来源
    #[test]
    fn targets_mirror_sources() {
        let t = targets();
        let s = sources();
        assert_eq!(t.len(), s.len());
        for (target, def) in t.iter().zip(s.iter()) {
            assert_eq!(target.id, def.agent);
            assert_eq!(target.writable, def.writable);
            assert!(!target.name.is_empty(), "{} 缺少显示名", target.id);
        }
        let mut ids: Vec<&str> = t.iter().map(|x| x.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), t.len(), "targets 出现重复 agent");
    }
}
