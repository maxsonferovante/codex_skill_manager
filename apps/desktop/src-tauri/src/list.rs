use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{ListSkillsInput, ListSkillsOutput, SkillEntry};
use crate::paths::resolve_roots;

fn slug_is_hidden(slug: &str) -> bool {
    slug.starts_with('.')
}

fn list_one_root(root: &Path, include_hidden: bool, filter: Option<&str>) -> Vec<SkillEntry> {
    let mut items: Vec<SkillEntry> = Vec::new();
    let filter_lc = filter.map(str::to_lowercase);

    let read_dir = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return items,
    };

    for entry in read_dir.flatten() {
        let path: PathBuf = entry.path();
        if !path.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        if !include_hidden && slug_is_hidden(&slug) {
            continue;
        }
        if let Some(needle) = &filter_lc {
            if !slug.to_lowercase().contains(needle) {
                continue;
            }
        }
        items.push(SkillEntry {
            slug,
            path: path.display().to_string(),
        });
    }

    items.sort_by(|a, b| a.slug.cmp(&b.slug));
    items
}

pub fn list_skills(input: ListSkillsInput) -> ListSkillsOutput {
    let roots = resolve_roots();
    let enabled_root = PathBuf::from(roots.enabled_root);
    let disabled_root = PathBuf::from(roots.disabled_root);

    ListSkillsOutput {
        enabled: list_one_root(&enabled_root, input.include_hidden, input.filter.as_deref()),
        disabled: list_one_root(&disabled_root, input.include_hidden, input.filter.as_deref()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn lists_directories_only_and_filters_hidden() {
        let tmp = TempDir::new().expect("tempdir");
        let base = tmp.path().join(".codex");
        let enabled = base.join("skills");
        let disabled = base.join("skills_disabled");
        std::fs::create_dir_all(&enabled).expect("create enabled");
        std::fs::create_dir_all(&disabled).expect("create disabled");

        std::fs::create_dir_all(enabled.join("alpha")).expect("dir alpha");
        std::fs::create_dir_all(enabled.join(".system")).expect("dir hidden");
        std::fs::write(enabled.join("readme.txt"), "x").expect("file");

        std::env::set_var("CODEX_HOME", base.display().to_string());
        let output = list_skills(ListSkillsInput {
            include_hidden: false,
            filter: None,
        });
        assert_eq!(output.enabled.len(), 1);
        assert_eq!(output.enabled[0].slug, "alpha");
        assert_eq!(output.disabled.len(), 0);
    }

    #[test]
    fn applies_case_insensitive_filter() {
        let tmp = TempDir::new().expect("tempdir");
        let base = tmp.path().join(".codex");
        let enabled = base.join("skills");
        std::fs::create_dir_all(&enabled).expect("create enabled");
        std::fs::create_dir_all(enabled.join("AbcSkill")).expect("create skill");
        std::env::set_var("CODEX_HOME", base.display().to_string());

        let output = list_skills(ListSkillsInput {
            include_hidden: true,
            filter: Some("abc".to_string()),
        });
        assert_eq!(output.enabled.len(), 1);
        assert_eq!(output.enabled[0].slug, "AbcSkill");
    }
}
