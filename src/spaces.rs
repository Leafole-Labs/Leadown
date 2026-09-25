//! Spaces: named note folders, like Obsidian vaults. The list lives in
//! `$XDG_CONFIG_HOME/abstract/spaces` as one absolute path per line; the line
//! prefixed with `* ` is the active space.

use std::path::{Path, PathBuf};

use crate::store::{write_atomic, xdg};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spaces {
    pub paths: Vec<PathBuf>,
    pub active: usize,
}

impl Spaces {
    pub fn parse(src: &str) -> Self {
        let mut paths: Vec<PathBuf> = Vec::new();
        let mut active = 0;
        for line in src.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let (is_active, path) = match line.strip_prefix("* ") {
                Some(p) => (true, p.trim()),
                None => (false, line),
            };
            let path = PathBuf::from(path);
            if paths.contains(&path) {
                continue;
            }
            if is_active {
                active = paths.len();
            }
            paths.push(path);
        }
        Self { paths, active }
    }

    pub fn serialize(&self) -> String {
        self.paths
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "{}{}\n",
                    if i == self.active { "* " } else { "" },
                    p.display()
                )
            })
            .collect()
    }

    pub fn current(&self) -> &Path {
        &self.paths[self.active]
    }

    /// Adds `path` if new and makes it active.
    pub fn activate_path(&mut self, path: PathBuf) {
        self.active = match self.paths.iter().position(|p| *p == path) {
            Some(i) => i,
            None => {
                self.paths.push(path);
                self.paths.len() - 1
            }
        };
    }

    /// Forgets a space (files are untouched). The last space cannot be removed.
    pub fn remove(&mut self, ix: usize) -> bool {
        if self.paths.len() <= 1 || ix >= self.paths.len() {
            return false;
        }
        self.paths.remove(ix);
        if self.active > ix || self.active >= self.paths.len() {
            self.active = self.active.saturating_sub(1);
        }
        true
    }
}

pub fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn config_file() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config")
        .join("abstract")
        .join("spaces")
}

/// Blocking. Loads the list, seeding `~/.local/share/abstract/Pessoal` on first
/// run. A CLI path argument is added and activated.
pub fn load() -> Spaces {
    let mut spaces = std::fs::read_to_string(config_file())
        .map(|s| Spaces::parse(&s))
        .unwrap_or(Spaces {
            paths: Vec::new(),
            active: 0,
        });
    if spaces.paths.is_empty() {
        spaces.paths.push(
            xdg("XDG_DATA_HOME", ".local/share")
                .join("abstract")
                .join("Pessoal"),
        );
    }
    if let Some(arg) = std::env::args_os().nth(1) {
        let p = PathBuf::from(arg);
        spaces.activate_path(std::fs::canonicalize(&p).unwrap_or(p));
    }
    spaces
}

/// Blocking.
pub fn save(spaces: &Spaces) {
    let file = config_file();
    let result = write_atomic(&file, spaces.serialize().as_bytes());
    if let Err(err) = result {
        eprintln!(
            "abstract: failed to save spaces to {}: {err}",
            file.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_keeps_active_and_dedups() {
        let s = Spaces::parse("/a\n* /b\n/a\n\n/c\n");
        assert_eq!(
            s.paths,
            vec![
                PathBuf::from("/a"),
                PathBuf::from("/b"),
                PathBuf::from("/c")
            ]
        );
        assert_eq!(s.current(), Path::new("/b"));
        assert_eq!(Spaces::parse(&s.serialize()), s);
    }

    #[test]
    fn remove_keeps_active_space_pointed_correctly() {
        let mut s = Spaces::parse("/a\n/b\n* /c\n");
        assert!(s.remove(0));
        assert_eq!(s.current(), Path::new("/c"));
        assert!(s.remove(1));
        assert_eq!(s.current(), Path::new("/b"));
        assert!(!s.remove(0), "last space must stay");
    }

    #[test]
    fn activate_existing_or_new() {
        let mut s = Spaces::parse("* /a\n/b\n");
        s.activate_path("/b".into());
        assert_eq!(s.active, 1);
        s.activate_path("/z".into());
        assert_eq!((s.active, s.paths.len()), (2, 3));
    }
}
