use std::path::PathBuf;

use crate::domain::{Direction, ListSkillsInput, StartOperationInput};
use crate::list::list_skills;
use crate::paths::resolve_roots;
use crate::skill_slug::is_safe_slug;

#[derive(Debug, Clone)]
pub struct MovePlanItem {
    pub slug: String,
    pub src: PathBuf,
    pub dst: PathBuf,
}

pub fn build_plan(input: &StartOperationInput) -> Vec<MovePlanItem> {
    let roots = resolve_roots();
    let enabled_root = PathBuf::from(roots.enabled_root);
    let disabled_root = PathBuf::from(roots.disabled_root);

    let source_slugs = match input.mode {
        crate::domain::OperationMode::Selected => input.slugs.clone().unwrap_or_default(),
        crate::domain::OperationMode::AllVisible => {
            let listed = list_skills(ListSkillsInput {
                include_hidden: input.include_hidden,
                filter: input.filter.clone(),
            });
            match input.direction {
                Direction::Disable => listed.enabled.into_iter().map(|entry| entry.slug).collect(),
                Direction::Enable => listed.disabled.into_iter().map(|entry| entry.slug).collect(),
            }
        }
    };

    source_slugs
        .into_iter()
        .filter(|slug| is_safe_slug(slug))
        .map(|slug| {
            let (src, dst) = match input.direction {
                Direction::Disable => (enabled_root.join(&slug), disabled_root.join(&slug)),
                Direction::Enable => (disabled_root.join(&slug), enabled_root.join(&slug)),
            };
            MovePlanItem { slug, src, dst }
        })
        .collect()
}
