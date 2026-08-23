use std::{
    env,
    path::{Path, PathBuf},
};

const CANDIDATES: &[&str] = &["zsh", "bash", "fish", "dash", "sh"];
const SEARCH_DIRS: &[&str] = &["/usr/bin", "/usr/local/bin", "/bin"];

fn is_executable_file(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.is_file()
            && std::fs::metadata(path)
                .map(|meta| meta.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// Parsea el campo shell (índice 6) de una línea de passwd.
pub fn shell_from_passwd_line(line: &str) -> Option<String> {
    let shell = line.split(':').nth(6)?;
    if shell.is_empty() {
        None
    } else {
        Some(shell.to_string())
    }
}

pub fn passwd_shell_for_uid(uid: u32) -> Option<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    for line in passwd.lines() {
        let mut fields = line.split(':');
        let parsed_uid = fields.nth(2)?.parse::<u32>().ok()?;
        if parsed_uid == uid {
            return shell_from_passwd_line(line);
        }
    }
    None
}

fn find_in_search_dirs(name: &str) -> Option<PathBuf> {
    SEARCH_DIRS
        .iter()
        .map(|dir| Path::new(dir).join(name))
        .find(|path| is_executable_file(path))
}

/// Detecta el intérprete local: $SHELL → passwd del uid actual → candidatos conocidos.
pub fn detect_shell() -> Option<PathBuf> {
    if let Ok(shell) = env::var("SHELL") {
        let path = PathBuf::from(&shell);
        if is_executable_file(&path) {
            return Some(path);
        }
    }
    let uid = current_uid();
    if let Some(shell) = passwd_shell_for_uid(uid) {
        let path = PathBuf::from(&shell);
        if is_executable_file(&path) {
            return Some(path);
        }
        if let Some(found) = find_in_search_dirs(
            &Path::new(&shell)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
        ) {
            return Some(found);
        }
    }
    CANDIDATES.iter().find_map(|name| find_in_search_dirs(name))
}

#[cfg(target_os = "linux")]
fn current_uid() -> u32 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self")
        .map(|meta| meta.uid())
        .unwrap_or(u32::MAX)
}

#[cfg(not(target_os = "linux"))]
fn current_uid() -> u32 {
    u32::MAX
}

/// Intérpretes disponibles para el selector, con el detectado primero.
pub fn list_shells(detected: &Path) -> Vec<String> {
    let mut shells = vec![detected.display().to_string()];
    for name in CANDIDATES {
        if let Some(path) = find_in_search_dirs(name) {
            let candidate = path.display().to_string();
            if !shells.contains(&candidate) {
                shells.push(candidate);
            }
        }
    }
    shells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_passwd_line() {
        assert_eq!(
            shell_from_passwd_line("pedro:x:1000:1000::/home/pedro:/usr/bin/zsh"),
            Some("/usr/bin/zsh".into())
        );
        assert_eq!(
            shell_from_passwd_line("root:x:0:0::/root:"),
            None,
            "campo shell vacío"
        );
        assert_eq!(shell_from_passwd_line(""), None);
    }

    #[test]
    fn detects_some_shell_on_this_host() {
        let detected = detect_shell().expect("host de test debe tener un shell");
        assert!(detected.is_absolute(), "ruta absoluta: {detected:?}");
        assert!(is_executable_file(&detected));
        let list = list_shells(&detected);
        assert!(!list.is_empty());
        assert_eq!(&list[0], &detected.display().to_string());
        assert!(list.windows(2).all(|pair| pair[0] != pair[1]));
    }
}
