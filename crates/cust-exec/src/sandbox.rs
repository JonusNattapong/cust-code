use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxProfile {
    Off,
    #[default]
    Workspace,
    ReadOnly,
    Strict,
}

impl std::fmt::Display for SandboxProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => write!(f, "off"),
            Self::Workspace => write!(f, "workspace"),
            Self::ReadOnly => write!(f, "read-only"),
            Self::Strict => write!(f, "strict"),
        }
    }
}

impl std::str::FromStr for SandboxProfile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "workspace" => Ok(Self::Workspace),
            "read-only" | "readonly" => Ok(Self::ReadOnly),
            "strict" => Ok(Self::Strict),
            _ => Err(anyhow::anyhow!(
                "Unknown sandbox profile: '{s}' (valid: off, workspace, read-only, strict)"
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Self-Protection: paths the agent must never be allowed to write to,
// regardless of sandbox profile. Inspired by grok-build and codex which use
// kernel-enforced profiles (Landlock / Seatbelt) to deny writes to hook
// directories, preventing a compromised agent from installing persistence.
// ---------------------------------------------------------------------------

/// Directories that are always protected from agent writes, even when the
/// sandbox profile is `Off`. Returns `true` when the target path falls inside
/// a protected directory.
pub fn is_self_protected(target: &Path) -> bool {
    // Canonicalize for reliable comparison (best-effort).
    let target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    let target_str = target.to_string_lossy();

    // Normalise separators on Windows for reliable substring matching.
    let normalised = target_str.replace('\\', "/").to_lowercase();

    PROTECTED_PATH_PATTERNS
        .iter()
        .any(|pattern| normalised.contains(pattern))
}

/// Lower-cased, forward-slash-normalised substrings that mark a path as
/// protected. Any path containing one of these is denied write access.
const PROTECTED_PATH_PATTERNS: &[&str] = &[
    // Git hooks — prevent installing persistent hooks
    "/.git/hooks/",
    "/.git/hooks",
    // Agent configuration directories (prevent config poisoning)
    "/.cust/config",
    "/.claude/config",
    "/.codex/config",
    // SSH keys
    "/.ssh/",
    // Shell startup files
    "/.bashrc",
    "/.zshrc",
    "/.profile",
    "/.bash_profile",
    // Windows equivalents
    "/appdata/roaming/microsoft/windows/start menu/programs/startup/",
];

impl SandboxProfile {
    pub fn check_permission(
        &self,
        cwd: &Path,
        is_write: bool,
        target_path: Option<&Path>,
    ) -> Result<(), anyhow::Error> {
        // Self-protection check: always enforced regardless of profile.
        if is_write {
            if let Some(target) = target_path {
                if is_self_protected(target) {
                    return Err(anyhow::anyhow!(
                        "Write access to '{}' blocked: path is in a self-protected directory \
                         (git hooks, SSH keys, agent config). This protection cannot be disabled.",
                        target.display()
                    ));
                }
            }
        }

        match self {
            Self::Off => Ok(()),
            Self::ReadOnly => {
                if is_write {
                    Err(anyhow::anyhow!(
                        "Write operation blocked under sandbox profile 'read-only' (on Windows boundary enforcement)"
                    ))
                } else {
                    Ok(())
                }
            }
            Self::Workspace => {
                if let Some(target) = target_path {
                    let canonical_cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
                    let canonical_target = target
                        .canonicalize()
                        .unwrap_or_else(|_| target.to_path_buf());
                    if is_write && !canonical_target.starts_with(&canonical_cwd) {
                        return Err(anyhow::anyhow!(
                            "Write access to '{}' blocked under 'workspace' sandbox profile (outside working dir '{}')",
                            target.display(),
                            cwd.display()
                        ));
                    }
                }
                Ok(())
            }
            Self::Strict => {
                if let Some(target) = target_path {
                    let canonical_cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
                    let canonical_target = target
                        .canonicalize()
                        .unwrap_or_else(|_| target.to_path_buf());
                    if !canonical_target.starts_with(&canonical_cwd) {
                        return Err(anyhow::anyhow!(
                            "Path access to '{}' blocked under 'strict' sandbox profile (outside working dir '{}')",
                            target.display(),
                            cwd.display()
                        ));
                    }
                }
                Ok(())
            }
        }
    }
}
