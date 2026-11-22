use reciprocal::workspace::WorkspaceState;
use std::{fs, path::PathBuf};

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("reciprocal-{label}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn workspace_reads_writes_lists_and_searches_files() {
    let root = temp_dir("workspace");
    fs::write(root.join("start.txt"), "first line\nneedle here\n").unwrap();
    let workspace = WorkspaceState::default();
    workspace.set_root(root.to_str().unwrap()).unwrap();

    assert!(workspace.read("start.txt").unwrap().contains("first line"));
    workspace.write("start.txt", "updated needle\n").unwrap();
    assert_eq!(workspace.read("start.txt").unwrap(), "updated needle\n");
    assert!(workspace.list(None).unwrap().iter().any(|entry| entry.name == "start.txt"));
    let matches = workspace.search("needle").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line, 1);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn workspace_rejects_parent_traversal_and_absolute_paths() {
    let root = temp_dir("boundary");
    let outside = root.parent().unwrap().join(format!("outside-{}.txt", uuid::Uuid::new_v4()));
    fs::write(&outside, "outside").unwrap();
    let workspace = WorkspaceState::default();
    workspace.set_root(root.to_str().unwrap()).unwrap();

    assert!(workspace.read("../outside.txt").is_err());
    assert!(workspace.read(outside.to_str().unwrap()).is_err());
    assert!(workspace.write("../outside.txt", "changed").is_err());
    assert_eq!(fs::read_to_string(&outside).unwrap(), "outside");

    fs::remove_file(outside).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn workspace_rejects_symlinks_that_escape_root() {
    use std::os::unix::fs::symlink;
    let root = temp_dir("symlink-root");
    let outside = temp_dir("symlink-outside");
    fs::write(outside.join("secret.txt"), "secret").unwrap();
    symlink(&outside, root.join("escape")).unwrap();
    let workspace = WorkspaceState::default();
    workspace.set_root(root.to_str().unwrap()).unwrap();

    assert!(workspace.read("escape/secret.txt").is_err());
    assert!(workspace.write("escape/new.txt", "nope").is_err());

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}
