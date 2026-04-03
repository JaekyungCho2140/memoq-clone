use crate::models::{Project, ProjectFile, ProjectStats, SegmentStatus};
use tauri::command;
use uuid::Uuid;

const RECENT_PROJECTS_FILE: &str = "memoq-clone-recent.json";
const MAX_RECENT: usize = 5;

fn recent_projects_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join(RECENT_PROJECTS_FILE))
}

fn read_recent_projects() -> Vec<String> {
    let Some(path) = recent_projects_path() else {
        return Vec::new();
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
}

fn write_recent_projects(projects: &[String]) {
    let Some(path) = recent_projects_path() else {
        return;
    };
    if let Ok(json) = serde_json::to_string_pretty(projects) {
        let _ = std::fs::write(path, json);
    }
}

fn push_recent(mqclone_path: &str) {
    let mut recent = read_recent_projects();
    recent.retain(|p| p != mqclone_path);
    recent.insert(0, mqclone_path.to_string());
    recent.truncate(MAX_RECENT);
    write_recent_projects(&recent);
}

// ── Tauri Commands ──────────────────────────────────────────────────────────

#[command]
pub async fn add_file_to_project(
    mut project: Project,
    file_path: String,
) -> Result<Project, String> {
    let pf = ProjectFile {
        id: Uuid::new_v4().to_string(),
        path: file_path,
        segments: Vec::new(),
    };
    project.files.push(pf);
    Ok(project)
}

#[command]
pub async fn remove_file_from_project(
    mut project: Project,
    file_id: String,
) -> Result<Project, String> {
    project.files.retain(|f| f.id != file_id);
    Ok(project)
}

#[command]
pub async fn save_project(project: Project, save_path: String) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(&project).map_err(|e| format!("Serialization error: {e}"))?;
    std::fs::write(&save_path, json).map_err(|e| format!("Write error: {e}"))?;
    push_recent(&save_path);
    Ok(())
}

#[command]
pub async fn load_project(load_path: String) -> Result<Project, String> {
    let json = std::fs::read_to_string(&load_path).map_err(|e| format!("Read error: {e}"))?;
    let project: Project = serde_json::from_str(&json).map_err(|e| format!("Parse error: {e}"))?;
    push_recent(&load_path);
    Ok(project)
}

#[command]
pub async fn get_project_stats(project: Project) -> Result<ProjectStats, String> {
    let mut total = project.segments.len();
    let mut translated = project
        .segments
        .iter()
        .filter(|s| {
            matches!(
                s.status,
                SegmentStatus::Translated | SegmentStatus::Confirmed
            )
        })
        .count();
    let mut confirmed = project
        .segments
        .iter()
        .filter(|s| s.status == SegmentStatus::Confirmed)
        .count();

    for file in &project.files {
        let (ft, ftr, fc) = file.completion_stats();
        total += ft;
        translated += ftr;
        confirmed += fc;
    }

    let completion_pct = if total == 0 {
        0.0
    } else {
        (translated as f32 / total as f32) * 100.0
    };

    Ok(ProjectStats {
        total_segments: total,
        translated,
        confirmed,
        completion_pct,
    })
}

#[command]
pub async fn get_recent_projects() -> Result<Vec<String>, String> {
    Ok(read_recent_projects())
}

// ── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Segment, SegmentStatus};
    use chrono::Utc;

    fn seg(status: SegmentStatus) -> Segment {
        Segment {
            id: Uuid::new_v4().to_string(),
            source: "src".to_string(),
            target: "tgt".to_string(),
            status,
            order: 0,
        }
    }

    fn empty_project() -> Project {
        Project {
            id: Uuid::new_v4().to_string(),
            name: "Test".to_string(),
            source_lang: "en".to_string(),
            target_lang: "ko".to_string(),
            created_at: Utc::now(),
            files: Vec::new(),
            source_path: String::new(),
            segments: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_add_file_to_project() {
        let project = empty_project();
        let result = add_file_to_project(project, "/path/to/file.xliff".to_string())
            .await
            .unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].path, "/path/to/file.xliff");
    }

    #[tokio::test]
    async fn test_remove_file_from_project() {
        let mut project = empty_project();
        let file_id = Uuid::new_v4().to_string();
        project.files.push(ProjectFile {
            id: file_id.clone(),
            path: "/path/to/file.xliff".to_string(),
            segments: Vec::new(),
        });
        let result = remove_file_from_project(project, file_id).await.unwrap();
        assert!(result.files.is_empty());
    }

    #[tokio::test]
    async fn test_save_and_load_project() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.mqclone");
        let path_str = path.to_str().unwrap().to_string();

        let mut project = empty_project();
        project.name = "Test Project".to_string();
        project.files.push(ProjectFile {
            id: "f1".to_string(),
            path: "/some/file.xliff".to_string(),
            segments: Vec::new(),
        });

        save_project(project.clone(), path_str.clone())
            .await
            .unwrap();
        assert!(path.exists());

        let loaded = load_project(path_str).await.unwrap();
        assert_eq!(loaded.name, "Test Project");
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].path, "/some/file.xliff");
    }

    #[tokio::test]
    async fn test_get_project_stats_empty() {
        let project = empty_project();
        let stats = get_project_stats(project).await.unwrap();
        assert_eq!(stats.total_segments, 0);
        assert_eq!(stats.completion_pct, 0.0);
    }

    #[tokio::test]
    async fn test_get_project_stats_with_files() {
        let mut project = empty_project();
        project.files.push(ProjectFile {
            id: "f1".to_string(),
            path: "/file1.xliff".to_string(),
            segments: vec![
                seg(SegmentStatus::Confirmed),
                seg(SegmentStatus::Translated),
                seg(SegmentStatus::Untranslated),
            ],
        });
        project.files.push(ProjectFile {
            id: "f2".to_string(),
            path: "/file2.xliff".to_string(),
            segments: vec![seg(SegmentStatus::Confirmed), seg(SegmentStatus::Draft)],
        });

        let stats = get_project_stats(project).await.unwrap();
        assert_eq!(stats.total_segments, 5);
        assert_eq!(stats.translated, 3); // 2 confirmed + 1 translated
        assert_eq!(stats.confirmed, 2);
        assert!((stats.completion_pct - 60.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_get_project_stats_legacy_segments() {
        let mut project = empty_project();
        // Legacy: segments directly on project (no files)
        project.segments = vec![
            seg(SegmentStatus::Translated),
            seg(SegmentStatus::Confirmed),
        ];

        let stats = get_project_stats(project).await.unwrap();
        assert_eq!(stats.total_segments, 2);
        assert_eq!(stats.translated, 2);
        assert_eq!(stats.confirmed, 1);
        assert!((stats.completion_pct - 100.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_mqclone_file_extension_convention() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("myproject.mqclone");
        let path_str = path.to_str().unwrap().to_string();

        let project = empty_project();
        save_project(project, path_str.clone()).await.unwrap();

        let loaded = load_project(path_str).await.unwrap();
        assert_eq!(loaded.source_lang, "en");
        assert_eq!(loaded.target_lang, "ko");
    }
}
