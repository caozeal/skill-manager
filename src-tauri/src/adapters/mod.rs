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
    /// 安装探测路径（相对 $HOME）。任一存在 ⇒ 认为本机装了/用过该 agent，UI 才展示。
    /// 这样「界面里有哪些 agent」由本机实际情况决定，不写死；空数组表示无法探测，始终展示。
    pub probe_paths: &'static [&'static str],
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
        compat = $compat:expr,
        probe = $probe:expr
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
            probe_paths: $probe,
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
                compat = None,
                probe = &[".claude", ".claude.json"]
            ),
            adapter!(
                id = "codex",
                name = "Codex",
                icon = "code",
                path = ".codex/skills",
                kind = AdapterKind::Standard,
                mount = true,
                read_only = false,
                note = "内置 tier（.system/.curated/.experimental，已展开为独立技能）+ 本地安装技能；全局技能官方位置为 ~/.agents/skills（中央仓库，直接生效）",
                global = Some("agents-central"),
                compat = None,
                probe = &[".codex"]
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
                compat = None,
                probe = &[".omp"]
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
                compat = None,
                probe = &[".kimi-code"]
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
                compat = None,
                probe = &[".grok"]
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
                compat = None,
                probe = &[".codebuddy"]
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
                compat = None,
                probe = &[".openclaw"]
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
                probe_paths: &[".dsh", ".deepseek"],
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
                probe_paths: &[".config/opencode"],
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
                compat = None,
                probe = &[".agents/skills"]
            ),
        ]
    })
}

/// 本机「实际存在」的 agent：probe 路径任一命中即算，未命中则不进 UI。
/// 判定实时进行（不缓存），因此装/卸 agent 后重新拉一次列表即可反映。
pub fn is_installed(adapter: &AgentAdapter) -> bool {
    if adapter.probe_paths.is_empty() {
        return true;
    }
    let h = home();
    adapter
        .probe_paths
        .iter()
        .any(|rel| std::fs::symlink_metadata(h.join(rel)).is_ok())
}

/// 只包含本机实际存在的 adapter（UI 列表 / 全量扫描的统一入口）
pub fn installed_adapters() -> Vec<&'static AgentAdapter> {
    registry().iter().filter(|a| is_installed(a)).collect()
}

pub fn find(id: &str) -> Option<&'static AgentAdapter> {
    registry().iter().find(|a| a.id == id)
}

/// 中央仓库技能目录的 canonical 路径（用于判断符号链接是否指向中央仓）
pub fn central_dir() -> PathBuf {
    home().join(".agents/skills")
}

/// 已禁用技能的存放目录：与 skills 目录同级（~/.claude/skills → ~/.claude/disabled-skills）。
///
/// 之所以不沿用「skills 目录下加 `.` 前缀」的旧做法：隐藏目录是否被加载完全取决于
/// 各 agent 的实现细节（Codex 明确跳过 `.` 开头目录，OpenCode 用 glob 依赖库的 dot 选项，
/// Claude Code 未公开约定），且社区脚本常用 `find ~/.xx/skills -name SKILL.md`，
/// 这类写法不排除隐藏目录。放到 skills **目录之外**才是确定性的隔离。
pub fn disabled_dir(adapter: &AgentAdapter) -> Option<PathBuf> {
    adapter
        .skills_path
        .as_ref()
        .and_then(|p| p.parent())
        .map(|p| p.join("disabled-skills"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 注册表里的每个 adapter 都必须声明探测路径，否则「动态展示」会退化成永久隐藏
    #[test]
    fn every_adapter_declares_probe_paths() {
        for a in registry() {
            assert!(!a.probe_paths.is_empty(), "adapter {} 缺少 probe_paths", a.id);
        }
    }

    /// 探测未命中 ⇒ 不进 UI 列表（openclaw 卸载后就靠这条自动消失）
    #[test]
    fn nonexistent_probe_means_not_installed() {
        let mut fake = registry()
            .iter()
            .find(|a| a.id == "openclaw")
            .expect("openclaw 定义应保留在注册表中（由探测决定是否展示）")
            .clone();
        fake.probe_paths = &["__skill_manager_definitely_missing__"];
        assert!(!is_installed(&fake));
    }

    /// 已安装列表是注册表的子集，且不会凭空多出条目
    #[test]
    fn installed_adapters_is_subset_of_registry() {
        let inst = installed_adapters();
        assert!(inst.len() <= registry().len());
        for a in &inst {
            assert!(registry().iter().any(|r| r.id == a.id));
        }
    }

    /// 真实 HOME 下探测命中 ⇒ 一定出现在已安装列表里（用当前环境最可能存在的中央仓库验证）
    #[test]
    fn probe_hit_appears_in_installed_list() {
        let central = find("agents-central").expect("中央仓库定义应存在");
        if is_installed(central) {
            assert!(installed_adapters().iter().any(|a| a.id == "agents-central"));
        }
    }
}
