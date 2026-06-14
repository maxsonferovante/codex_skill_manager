use std::path::{Path, PathBuf};

pub fn slugify(input: &str) -> String {
    crate::skill_slug::slugify(input)
}

pub fn resolve_unique_skill_dir(skills_root: &Path, slug: &str, overwrite: bool) -> PathBuf {
    crate::skill_slug::resolve_unique_skill_dir(skills_root, slug, overwrite)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn slugify_normalizes_and_falls_back() {
        assert_eq!(slugify("Padrões de Projetos"), "padroes-de-projetos");
        assert_eq!(slugify("  ---  "), "skill");
    }

    #[test]
    fn unique_dir_resolves_suffix_when_collision() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        std::fs::create_dir(root.join("book")).expect("mkdir");
        std::fs::create_dir(root.join("book-2")).expect("mkdir");

        let dir = resolve_unique_skill_dir(root, "book", false);
        assert!(dir.ends_with("book-3"));
    }
}
