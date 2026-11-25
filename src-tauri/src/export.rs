use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tauri::{AppHandle, Manager};

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn target(dir: &Path, name: &str, kind: &str) -> Result<PathBuf, String> {
    if !safe_name(name) {
        return Err("The name may only use lowercase letters, digits and hyphens.".into());
    }
    match kind {
        "skill" => Ok(dir.join(name).join("SKILL.md")),
        "prompt" => Ok(dir.join(format!("{name}.md"))),
        _ => Err("unknown kind".into()),
    }
}

#[tauri::command(async)]
pub fn export(app: AppHandle, name: String, kind: String, content: String, dir: Option<String>, overwrite: bool) -> Result<String, String> {
    let dir = match dir {
        Some(d) => PathBuf::from(d),
        None => app.path().home_dir().map_err(|e| e.to_string())?.join(".claude/skills"),
    };
    let path = target(&dir, &name, &kind)?;
    if path.exists() && !overwrite {
        return Err("EXISTS".into());
    }
    std::fs::create_dir_all(path.parent().unwrap())
        .and_then(|_| std::fs::write(&path, content))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command(async)]
pub fn copy_text(text: String) -> Result<(), String> {
    let mut child = Command::new("/usr/bin/pbcopy")
        .env("LC_CTYPE", "UTF-8")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child.stdin.take().unwrap().write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    child.wait().map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn reveal(path: String) -> Result<(), String> {
    Command::new("/usr/bin/open").arg("-R").arg("--").arg(path).status().map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::target;
    use std::path::Path;

    #[test]
    fn targets() {
        let d = Path::new("/x");
        assert_eq!(target(d, "a-b", "skill").unwrap(), Path::new("/x/a-b/SKILL.md"));
        assert_eq!(target(d, "a", "prompt").unwrap(), Path::new("/x/a.md"));
        for bad in ["", "..", "a/b", "A", "a b"] {
            assert!(target(d, bad, "skill").is_err(), "{bad}");
        }
    }
}
