use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    AgentNotFound(String),
    #[error("该 agent 的技能目录不存在")]
    AgentDirMissing,
    #[error("该 agent 为只读展示，不支持此操作")]
    ReadOnlyAgent,
    #[error("该 agent 不支持符号链接挂载")]
    MountNotSupported,
    #[error("真实目录不允许卸载（仅符号链接可卸载）")]
    RealDirNoDelete,
    #[error("系统内置技能（.system）为只读")]
    SystemDirReadOnly,
    #[error("目标已存在同名条目")]
    TargetExists,
    #[error("中央仓库中不存在该技能: {0}")]
    CentralSkillMissing(String),
    #[error("未找到 SKILL.md")]
    SkillMdMissing,
    #[error("frontmatter 解析失败: {0}")]
    InvalidFrontmatter(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let code = match self {
            AppError::AgentNotFound(_) => "AgentNotFound",
            AppError::AgentDirMissing => "AgentDirMissing",
            AppError::ReadOnlyAgent => "ReadOnlyAgent",
            AppError::MountNotSupported => "MountNotSupported",
            AppError::RealDirNoDelete => "RealDirNoDelete",
            AppError::SystemDirReadOnly => "SystemDirReadOnly",
            AppError::TargetExists => "TargetExists",
            AppError::CentralSkillMissing(_) => "CentralSkillMissing",
            AppError::SkillMdMissing => "SkillMdMissing",
            AppError::InvalidFrontmatter(_) => "InvalidFrontmatter",
            AppError::Io(_) => "IoError",
        };
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", code)?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
