use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AdapterKind {
    Standard,
    Informational,
}

#[derive(Clone, Debug)]
pub struct AgentAdapter {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    /// None 表示无本地技能目录（informational）
    pub skills_path: Option<PathBuf>,
    pub kind: AdapterKind,
    pub has_skills_dir: bool,
    pub supports_symlink_mount: bool,
    pub read_only: bool,
    pub note: &'static str,
    /// 若指定，该 agent 会将此源（如中央仓库）的技能合并展示为「全局技能」
    pub global_skills_source: Option<&'static str>,
    /// 若指定，该 agent 会将此源的技能合并展示为「兼容技能」
    /// （如 OpenCode 兼容读取 ~/.claude/skills）
    pub compat_skills_source: Option<&'static str>,
}

fn home() -> PathBuf {
    dirs::home_dir().expect("无法获取 HOME 目录")
}

macro_rules! adapter {
    (
        id = $id:literal,
        name = $name:literal,
        icon = $icon:literal,
        path = $path:expr,
        kind = $kind:expr,
        mount = $mount:expr,
        read_only = $read_only:expr,
        note = $note:literal,
        global = $global:expr,
        compat = $compat:expr
    ) => {
        AgentAdapter {
            id: $id,
            name: $name,
            icon: $icon,
            skills_path: Some(home().join($path)),
            kind: $kind,
            has_skills_dir: true,
            supports_symlink_mount: $mount,
            read_only: $read_only,
            note: $note,
            global_skills_source: $global,
            compat_skills_source: $compat,
        }
    };
}

fn registry() -> &'static Vec<AgentAdapter> {
    static REG: OnceLock<Vec<AgentAdapter>> = OnceLock::new();
    REG.get_or_init(|| {
        vec![
            adapter!(
                id = "claude",
                name = "Claude Code",
                icon = "terminal",
                path = ".claude/skills",
                kind = AdapterKind::Standard,
                mount = true,
                read_only = false,
                note = "官方/社区技能目录：本地安装 + 符号链接挂载中央仓库（~/.agents/skills）",
                global = None,
                compat = None
            ),
            adapter!(
                id = "codex",
                name = "Codex",
                icon = "code",
                path = ".codex/skills",
                kind = AdapterKind::Standard,
                mount = true,
                read_only = false,
                note = "内置(.system) + 本地安装技能；全局技能官方位置为 ~/.agents/skills（中央仓库，直接生效）",
                global = Some("agents-central"),
                compat = None
            ),
            adapter!(
                id = "omp",
                name = "OMP",
                icon = "bot",
                path = ".omp/agent/managed-skills",
                kind = AdapterKind::Standard,
                mount = true,
                read_only = false,
                note = "本地托管技能目录，不加载中央仓库",
                global = None,
                compat = None
            ),
            adapter!(
                id = "kimi-code",
                name = "Kimi Code",
                icon = "moon",
                path = ".kimi-code/skills",
                kind = AdapterKind::Standard,
                mount = true,
                read_only = false,
                note = "本地安装 + 符号链接挂载中央仓库；全局技能官方位置为 ~/.agents/skills（中央仓库，直接生效）",
                global = Some("agents-central"),
                compat = None
            ),
            adapter!(
                id = "grok",
                name = "Grok",
                icon = "zap",
                path = ".grok/skills",
                kind = AdapterKind::Standard,
                mount = true,
                read_only = false,
                note = "技能全部为符号链接，挂载自中央仓库 ~/.agents/skills",
                global = None,
                compat = None
            ),
            adapter!(
                id = "codebuddy",
                name = "CodeBuddy",
                icon = "bird",
                path = ".codebuddy/skills",
                kind = AdapterKind::Standard,
                mount = true,
                read_only = false,
                note = "技能全部为符号链接，挂载自中央仓库 ~/.agents/skills",
                global = None,
                compat = None
            ),
            adapter!(
                id = "openclaw",
                name = "OpenClaw",
                icon = "paw-print",
                path = ".openclaw/skills",
                kind = AdapterKind::Standard,
                mount = true,
                read_only = false,
                note = "技能目录：本地 + 符号链接挂载中央仓库（另有 plugin-skills / skill-workshop）",
                global = None,
                compat = None
            ),
            AgentAdapter {
                id: "dsh",
                name: "DeepSeek Harness",
                icon: "waves",
                skills_path: None,
                kind: AdapterKind::Standard,
                has_skills_dir: false,
                supports_symlink_mount: false,
                read_only: true,
                note: "无本地 skills 目录；全局技能官方位置为 ~/.agents/skills（中央仓库，直接生效）",
                global_skills_source: Some("agents-central"),
                compat_skills_source: None,
            },
            AgentAdapter {
                id: "opencode",
                name: "OpenCode",
                icon: "blocks",
                skills_path: Some(home().join(".config/opencode/skills")),
                kind: AdapterKind::Standard,
                has_skills_dir: true,
                supports_symlink_mount: false,
                read_only: true,
                note: "技能发现：~/.config/opencode/skills（本地）+ ~/.agents/skills（全局）+ ~/.claude/skills（Claude 兼容）",
                global_skills_source: Some("agents-central"),
                compat_skills_source: Some("claude"),
            },
            adapter!(
                id = "agents-central",
                name = "中央仓库",
                icon = "database",
                path = ".agents/skills",
                kind = AdapterKind::Standard,
                mount = false,
                read_only = false,
                note = "跨 agent 共享技能仓：claude/kimi/grok/codebuddy/openclaw 符号链接挂载；codex/dsh/kimi 全局机制直接读取",
                global = None,
                compat = None
            ),
        ]
    })
}

pub fn all_adapters() -> &'static Vec<AgentAdapter> {
    registry()
}

pub fn find(id: &str) -> Option<&'static AgentAdapter> {
    registry().iter().find(|a| a.id == id)
}

/// 中央仓库技能目录的 canonical 路径（用于判断符号链接是否指向中央仓）
pub fn central_dir() -> PathBuf {
    home().join(".agents/skills")
}
