//! Adding rules to a rules file on disk (ADR 004).

use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, Value, value};

use crate::error::RedactifyError;
use crate::rules::{USER_PATTERN_SIZE_LIMIT, compile_user_pattern, parse_rules};

/// Append a rule to the rules file at `path`, creating the file if it does
/// not exist.
///
/// The rule is validated exactly as a loaded rule is, and nothing is written
/// if it, or the file as it stands, is invalid. Comments and layout are kept,
/// the file is replaced atomically, and an existing file keeps its
/// permissions.
pub fn append_rule(path: &Path, id: &str, name: &str, pattern: &str) -> Result<(), RedactifyError> {
    let shown = path.display().to_string();
    let in_file = |cause: RedactifyError| RedactifyError::RulesFile {
        path: shown.clone(),
        cause: Box::new(cause),
    };

    let target = resolve_target(path).map_err(|source| RedactifyError::RulesIo {
        path: shown.clone(),
        source,
    })?;
    let existing = match fs::read_to_string(&target) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(RedactifyError::RulesIo {
                path: shown,
                source,
            });
        }
    };
    let text = existing.as_deref().unwrap_or("");

    let current = parse_rules(text, USER_PATTERN_SIZE_LIMIT).map_err(in_file)?;
    let ids: HashSet<&str> = current.iter().map(|r| r.id.as_str()).collect();
    if ids.contains(id) {
        return Err(in_file(RedactifyError::DuplicateRuleId {
            id: id.to_string(),
        }));
    }
    compile_user_pattern(id, pattern, USER_PATTERN_SIZE_LIMIT).map_err(in_file)?;

    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| in_file(RedactifyError::RulesEdit(e)))?;
    if !doc.contains_key("rules") {
        doc.insert("rules", Item::ArrayOfTables(ArrayOfTables::new()));
    }
    let item = &mut doc["rules"];
    if !item.is_array_of_tables() {
        let inline = std::mem::take(item);
        *item = Item::ArrayOfTables(
            inline
                .into_array_of_tables()
                .map_err(|_| in_file(RedactifyError::RulesLayout))?,
        );
    }
    let rules = item
        .as_array_of_tables_mut()
        .ok_or_else(|| in_file(RedactifyError::RulesLayout))?;
    let mut table = Table::new();
    table["id"] = value(id);
    table["name"] = value(name);
    table["pattern"] = pattern_item(pattern);
    rules.push(table);

    let updated = doc.to_string();
    parse_rules(&updated, USER_PATTERN_SIZE_LIMIT).map_err(in_file)?;

    write_atomic(&target, &updated, existing.is_some()).map_err(|source| {
        RedactifyError::RulesWrite {
            path: path.display().to_string(),
            source,
        }
    })
}

/// Write through a symlink to the file it points at; renaming over the link
/// itself would replace the link with a plain file.
fn resolve_target(path: &Path) -> io::Result<PathBuf> {
    match fs::canonicalize(path) {
        Ok(real) => Ok(real),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(e) => Err(e),
    }
}

/// A pattern as a TOML literal string ('...') where possible, matching how
/// the custom rules guide tells people to write them; a basic string
/// otherwise.
fn pattern_item(pattern: &str) -> Item {
    let literal_ok = !pattern.contains('\'') && !pattern.chars().any(char::is_control);
    if literal_ok && let Ok(v) = format!("'{pattern}'").parse::<Value>() {
        return Item::Value(v);
    }
    value(pattern)
}

fn write_atomic(path: &Path, contents: &str, replacing: bool) -> io::Result<()> {
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(contents.as_bytes())?;
    tmp.as_file().sync_all()?;

    if !replacing {
        tmp.persist(path).map_err(|e| e.error)?;
        return Ok(());
    }
    replace_existing(tmp, path)
}

#[cfg(not(windows))]
fn replace_existing(tmp: tempfile::NamedTempFile, path: &Path) -> io::Result<()> {
    fs::set_permissions(tmp.path(), fs::metadata(path)?.permissions())?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(windows)]
fn replace_existing(tmp: tempfile::NamedTempFile, path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    fn wide(p: &Path) -> Vec<u16> {
        p.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    // Closes the handle; the file is deleted on drop unless kept.
    let tmp_path = tmp.into_temp_path();
    let replaced = wide(path);
    let replacement = wide(&tmp_path);
    // SAFETY: both pointers are to NUL-terminated UTF-16 strings that live
    // until the call returns, and the optional arguments are null, which
    // ReplaceFileW accepts.
    let ok = unsafe {
        ReplaceFileW(
            replaced.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // The replacement file has become `path`; nothing is left to delete.
    let _ = tmp_path.keep();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_rules_file;

    const EXAMPLE: &str = "\
# Team rules. Ask security before changing.

[[rules]]
id = \"project_code\"   # internal tracker ids
name = \"Project Code\"
pattern = '(?i)\\bPRJ-\\d+\\b'
";

    fn write_example(dir: &Path) -> PathBuf {
        let path = dir.join("rules.toml");
        fs::write(&path, EXAMPLE).expect("write example");
        path
    }

    #[test]
    fn append_keeps_comments_and_existing_rules() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_example(dir.path());

        append_rule(&path, "badge", "Badge Number", r"\bBDG-\d{6}\b").expect("append");

        let text = fs::read_to_string(&path).expect("read");
        assert!(text.starts_with(EXAMPLE), "{text}");
        assert!(text.contains(r"pattern = '\bBDG-\d{6}\b'"), "{text}");
        let rules = load_rules_file(&path).expect("loads");
        let ids: Vec<&str> = rules.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["project_code", "badge"]);
    }

    #[test]
    fn append_creates_a_missing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("new.toml");

        append_rule(&path, "badge", "Badge Number", r"\bBDG-\d{6}\b").expect("append");

        let rules = load_rules_file(&path).expect("loads");
        assert_eq!(rules.len(), 1);
        assert!(rules[0].pattern.is_match("BDG-123456"));
    }

    #[test]
    fn duplicate_id_is_refused_and_file_unchanged() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_example(dir.path());

        match append_rule(&path, "project_code", "Again", "x") {
            Err(RedactifyError::RulesFile { cause, .. }) => {
                assert!(
                    matches!(*cause, RedactifyError::DuplicateRuleId { .. }),
                    "{cause}"
                )
            }
            other => panic!("expected a duplicate id error, got {other:?}"),
        }
        assert_eq!(fs::read_to_string(&path).expect("read"), EXAMPLE);
    }

    #[test]
    fn invalid_pattern_is_refused_and_file_unchanged() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_example(dir.path());

        match append_rule(&path, "broken", "Broken", "(unclosed") {
            Err(RedactifyError::RulesFile { cause, .. }) => {
                assert!(
                    matches!(*cause, RedactifyError::InvalidRule { .. }),
                    "{cause}"
                )
            }
            other => panic!("expected an invalid pattern error, got {other:?}"),
        }
        assert_eq!(fs::read_to_string(&path).expect("read"), EXAMPLE);
    }

    #[test]
    fn an_invalid_file_is_not_written_to() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("rules.toml");
        let broken = "[[rule]]\nid = \"x\"\nname = \"X\"\npattern = 'a'\n";
        fs::write(&path, broken).expect("write");

        assert!(append_rule(&path, "badge", "Badge", "b").is_err());
        assert_eq!(fs::read_to_string(&path).expect("read"), broken);
    }

    #[test]
    fn a_pattern_with_a_single_quote_is_still_saved_correctly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("rules.toml");

        append_rule(&path, "quoted", "Quoted", r"it's\s+\d+").expect("append");

        let rules = load_rules_file(&path).expect("loads");
        assert_eq!(rules[0].pattern.as_str(), r"it's\s+\d+");
    }

    #[cfg(unix)]
    #[test]
    fn existing_permissions_are_kept_and_new_files_are_private() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let shared = write_example(dir.path());
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o644)).expect("chmod");
        append_rule(&shared, "badge", "Badge", "b").expect("append");
        let mode = fs::metadata(&shared).expect("stat").permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);

        let fresh = dir.path().join("fresh.toml");
        append_rule(&fresh, "badge", "Badge", "b").expect("append");
        let mode = fs::metadata(&fresh).expect("stat").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_rules_file_is_written_through() {
        let dir = tempfile::tempdir().expect("tempdir");
        let real = write_example(dir.path());
        let link = dir.path().join("link.toml");
        std::os::unix::fs::symlink(&real, &link).expect("symlink");

        append_rule(&link, "badge", "Badge", "b").expect("append");

        assert!(
            fs::symlink_metadata(&link)
                .expect("lstat")
                .file_type()
                .is_symlink()
        );
        assert_eq!(load_rules_file(&real).expect("loads").len(), 2);
    }
}
