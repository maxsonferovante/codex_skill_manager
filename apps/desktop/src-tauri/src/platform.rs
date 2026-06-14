use std::path::Path;
use std::process::Command;

use crate::domain::OpenSkillsFolderOutput;
use crate::paths::resolve_roots;

pub fn open_skills_folder() -> OpenSkillsFolderOutput {
    let roots = resolve_roots();
    let path = roots.enabled_root;
    let path_ref = Path::new(&path);

    #[cfg(target_os = "macos")]
    let cmd = ("open", vec![path_ref.as_os_str()]);
    #[cfg(target_os = "linux")]
    let cmd = ("xdg-open", vec![path_ref.as_os_str()]);
    #[cfg(target_os = "windows")]
    let cmd = ("explorer", vec![path_ref.as_os_str()]);

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        return OpenSkillsFolderOutput {
            opened: false,
            path,
            message: Some("unsupported platform".to_string()),
        };
    }

    let result = Command::new(cmd.0).args(cmd.1).status();
    match result {
        Ok(status) if status.success() => OpenSkillsFolderOutput {
            opened: true,
            path,
            message: None,
        },
        Ok(status) => OpenSkillsFolderOutput {
            opened: false,
            path,
            message: Some(format!("command failed with status: {status}")),
        },
        Err(exc) => OpenSkillsFolderOutput {
            opened: false,
            path,
            message: Some(exc.to_string()),
        },
    }
}

