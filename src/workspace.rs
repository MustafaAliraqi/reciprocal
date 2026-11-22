use axum::{extract::Query, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    collections::HashSet,
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
};

const MAX_READ_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SEARCH_FILES: usize = 10_000;
const MAX_SEARCH_RESULTS: usize = 100;

#[derive(Clone, Default)]
pub struct WorkspaceState {
    root: Arc<RwLock<Option<PathBuf>>>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceView {
    pub configured: bool,
    pub root: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetWorkspaceRequest {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct PathQuery {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub kind: &'static str,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SearchMatch {
    pub path: String,
    pub line: usize,
    pub text: String,
}

impl WorkspaceState {
    fn root(&self) -> Result<PathBuf, String> {
        self.root
            .read()
            .map_err(|_| "workspace state is unavailable".to_string())?
            .clone()
            .ok_or_else(|| "اختَر مجلد Workspace أولاً".to_string())
    }

    pub fn view(&self) -> WorkspaceView {
        let root = self.root.read().ok().and_then(|value| value.clone());
        WorkspaceView {
            configured: root.is_some(),
            root: root.map(|path| path.to_string_lossy().into_owned()),
        }
    }

    pub fn set_root(&self, requested: &str) -> Result<WorkspaceView, String> {
        let path = PathBuf::from(requested.trim());
        let canonical = fs::canonicalize(&path)
            .map_err(|error| format!("تعذر فتح المجلد: {error}"))?;
        if !canonical.is_dir() {
            return Err("المسار المحدد ليس مجلداً".into());
        }
        *self.root.write().map_err(|_| "workspace state is unavailable".to_string())? =
            Some(canonical);
        Ok(self.view())
    }

    fn relative_path(value: &str) -> Result<PathBuf, String> {
        let path = Path::new(value);
        if path.as_os_str().is_empty() {
            return Ok(PathBuf::new());
        }
        if path.is_absolute()
            || path.components().any(|part| {
                matches!(part, Component::ParentDir | Component::RootDir | Component::Prefix(_))
            })
        {
            return Err("المسار يجب أن يكون نسبياً وداخل Workspace".into());
        }
        Ok(path.to_path_buf())
    }

    fn existing_path(&self, value: &str) -> Result<(PathBuf, PathBuf), String> {
        let root = self.root()?;
        let relative = Self::relative_path(value)?;
        let canonical = fs::canonicalize(root.join(&relative))
            .map_err(|error| format!("تعذر الوصول إلى المسار: {error}"))?;
        if !canonical.starts_with(&root) {
            return Err("تم رفض المسار لأنه يخرج من Workspace".into());
        }
        Ok((root, canonical))
    }

    pub fn list(&self, value: Option<&str>) -> Result<Vec<FileEntry>, String> {
        let (root, directory) = self.existing_path(value.unwrap_or(""))?;
        if !directory.is_dir() {
            return Err("المسار ليس مجلداً".into());
        }
        let mut entries = Vec::new();
        for item in fs::read_dir(&directory).map_err(|error| error.to_string())? {
            let item = item.map_err(|error| error.to_string())?;
            let path = item.path();
            let canonical = match fs::canonicalize(&path) {
                Ok(path) if path.starts_with(&root) => path,
                _ => continue,
            };
            let metadata = fs::metadata(&canonical).map_err(|error| error.to_string())?;
            let name = item.file_name().to_string_lossy().into_owned();
            let relative = canonical.strip_prefix(&root).map_err(|error| error.to_string())?;
            entries.push(FileEntry {
                name,
                path: relative.to_string_lossy().replace('\\', "/"),
                kind: if metadata.is_dir() { "directory" } else { "file" },
                size: if metadata.is_file() { Some(metadata.len()) } else { None },
            });
        }
        entries.sort_by(|a, b| (a.kind != "directory", &a.name).cmp(&(b.kind != "directory", &b.name)));
        Ok(entries)
    }

    pub fn read(&self, value: &str) -> Result<String, String> {
        let (_, path) = self.existing_path(value)?;
        let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
        if !metadata.is_file() {
            return Err("المسار ليس ملفاً".into());
        }
        if metadata.len() > MAX_READ_BYTES {
            return Err("حجم الملف يتجاوز حد القراءة (2 MiB)".into());
        }
        fs::read_to_string(path).map_err(|error| format!("الملف ليس نصياً أو تعذرت قراءته: {error}"))
    }

    pub fn write(&self, value: &str, content: &str) -> Result<(), String> {
        let root = self.root()?;
        let relative = Self::relative_path(value)?;
        if relative.as_os_str().is_empty() || relative.file_name().is_none() {
            return Err("حدد اسم ملف صالحاً".into());
        }
        let target = root.join(&relative);
        let parent = target.parent().ok_or_else(|| "مسار الملف غير صالح".to_string())?;
        let canonical_parent = fs::canonicalize(parent)
            .map_err(|error| format!("مجلد الملف غير موجود: {error}"))?;
        if !canonical_parent.starts_with(&root) {
            return Err("تم رفض الكتابة خارج Workspace".into());
        }
        match fs::symlink_metadata(&target) {
            Ok(_) => {
                let canonical_target = fs::canonicalize(&target)
                    .map_err(|_| "تم رفض رابط رمزي غير صالح".to_string())?;
                if !canonical_target.starts_with(&root) || !canonical_target.is_file() {
                    return Err("تم رفض الكتابة إلى مسار غير آمن".into());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        fs::write(target, content).map_err(|error| format!("فشلت كتابة الملف: {error}"))
    }

    pub fn search(&self, needle: &str) -> Result<Vec<SearchMatch>, String> {
        let root = self.root()?;
        let needle = needle.trim().to_lowercase();
        if needle.is_empty() {
            return Err("اكتب نصاً للبحث عنه".into());
        }
        let mut pending = vec![root.clone()];
        let mut visited_dirs = HashSet::new();
        let mut visited = 0usize;
        let mut matches = Vec::new();
        while let Some(directory) = pending.pop() {
            if !visited_dirs.insert(directory.clone()) {
                continue;
            }
            let items = fs::read_dir(directory).map_err(|error| error.to_string())?;
            for item in items {
                let item = item.map_err(|error| error.to_string())?;
                let path = item.path();
                let name = item.file_name().to_string_lossy().into_owned();
                if name == ".git" || name == "target" || name == "node_modules" {
                    continue;
                }
                let canonical = match fs::canonicalize(&path) {
                    Ok(path) if path.starts_with(&root) => path,
                    _ => continue,
                };
                let metadata = fs::metadata(&canonical).map_err(|error| error.to_string())?;
                if metadata.is_dir() {
                    pending.push(canonical);
                    continue;
                }
                if !metadata.is_file() || metadata.len() > MAX_READ_BYTES {
                    continue;
                }
                visited += 1;
                if visited > MAX_SEARCH_FILES {
                    return Ok(matches);
                }
                let Ok(contents) = fs::read_to_string(&canonical) else {
                    continue;
                };
                let relative = canonical.strip_prefix(&root).map_err(|error| error.to_string())?
                    .to_string_lossy().replace('\\', "/");
                for (index, line) in contents.lines().enumerate() {
                    if line.to_lowercase().contains(&needle) {
                        matches.push(SearchMatch {
                            path: relative.clone(),
                            line: index + 1,
                            text: line.chars().take(500).collect(),
                        });
                        if matches.len() >= MAX_SEARCH_RESULTS {
                            return Ok(matches);
                        }
                    }
                }
            }
        }
        Ok(matches)
    }
}

pub async fn get_workspace(
    axum::extract::State(state): axum::extract::State<Arc<crate::providers::AppState>>,
) -> Json<WorkspaceView> {
    Json(state.workspace.view())
}

pub async fn set_workspace(
    axum::extract::State(state): axum::extract::State<Arc<crate::providers::AppState>>,
    Json(input): Json<SetWorkspaceRequest>,
) -> Result<Json<WorkspaceView>, (StatusCode, String)> {
    state.workspace.set_root(&input.path).map(Json).map_err(|error| (StatusCode::BAD_REQUEST, error))
}

pub async fn list_files(
    axum::extract::State(state): axum::extract::State<Arc<crate::providers::AppState>>,
    Query(query): Query<PathQuery>,
) -> Result<Json<Vec<FileEntry>>, (StatusCode, String)> {
    state.workspace.list(query.path.as_deref()).map(Json).map_err(|error| (StatusCode::BAD_REQUEST, error))
}

pub async fn read_file(
    axum::extract::State(state): axum::extract::State<Arc<crate::providers::AppState>>,
    Query(query): Query<PathQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let path = query.path.ok_or((StatusCode::BAD_REQUEST, "حدد مسار الملف".into()))?;
    state.workspace.read(&path).map(|content| Json(serde_json::json!({"path":path,"content":content})))
        .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

pub async fn write_file(
    axum::extract::State(state): axum::extract::State<Arc<crate::providers::AppState>>,
    Json(input): Json<WriteFileRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    state.workspace.write(&input.path, &input.content)
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

pub async fn search_files(
    axum::extract::State(state): axum::extract::State<Arc<crate::providers::AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<SearchMatch>>, (StatusCode, String)> {
    state.workspace.search(&query.query).map(Json).map_err(|error| (StatusCode::BAD_REQUEST, error))
}
