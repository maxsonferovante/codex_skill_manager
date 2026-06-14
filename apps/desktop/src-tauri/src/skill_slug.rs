use std::path::{Path, PathBuf};

pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;

    for ch in input.chars() {
        let mapped = match ch {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'a',
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
            _ => '-',
        };

        if mapped == '-' {
            if !prev_dash && !out.is_empty() {
                out.push('-');
                prev_dash = true;
            }
            continue;
        }
        out.push(mapped);
        prev_dash = false;
    }

    let slug = out.trim_matches('-').to_string();
    if slug.is_empty() {
        "skill".to_string()
    } else {
        slug
    }
}

pub fn resolve_unique_skill_dir(skills_root: &Path, slug: &str, overwrite: bool) -> PathBuf {
    let target = skills_root.join(slug);
    if overwrite || !target.exists() {
        return target;
    }

    let mut idx = 2usize;
    loop {
        let candidate = skills_root.join(format!("{slug}-{idx}"));
        if !candidate.exists() {
            return candidate;
        }
        idx += 1;
    }
}

pub fn is_safe_slug(slug: &str) -> bool {
    if slug.is_empty() || slug == "." || slug == ".." {
        return false;
    }
    !(slug.contains('/') || slug.contains('\\'))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{is_safe_slug, resolve_unique_skill_dir, slugify};

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

    #[test]
    fn slug_safety_rejects_path_like_values() {
        assert!(is_safe_slug("alpha-skill"));
        assert!(!is_safe_slug("../alpha"));
        assert!(!is_safe_slug("alpha/beta"));
        assert!(!is_safe_slug("alpha\\beta"));
    }
}
