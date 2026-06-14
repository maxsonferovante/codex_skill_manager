use std::path::PathBuf;

use crate::domain::RootsInfo;

pub fn resolve_base_dir() -> PathBuf {
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        return PathBuf::from(codex_home);
    }

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".codex")
}

pub fn resolve_roots() -> RootsInfo {
    let base = resolve_base_dir();
    let enabled_root = base.join("skills");
    let disabled_root = base.join("skills_disabled");

    RootsInfo {
        base: base.display().to_string(),
        enabled_root: enabled_root.display().to_string(),
        disabled_root: disabled_root.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_codex_home_when_defined() {
        std::env::set_var("CODEX_HOME", "/tmp/codex-test-home");
        let roots = resolve_roots();
        assert_eq!(roots.base, "/tmp/codex-test-home");
        assert!(roots.enabled_root.ends_with("/tmp/codex-test-home/skills"));
        assert!(roots.disabled_root.ends_with("/tmp/codex-test-home/skills_disabled"));
    }
}
