//! Persistence integration tests: create → persist → reopen → verify.
//!
//! These run against a real SQLite file (not in-memory) to prove the M0
//! acceptance requirement: a task survives an application restart.

use ifuyo_tide_lib::core::ids;
use ifuyo_tide_lib::core::source::{Source, SourceType};
use ifuyo_tide_lib::core::task::{Task, TaskState};
use ifuyo_tide_lib::error::TideError;
use ifuyo_tide_lib::persistence::open;
use ifuyo_tide_lib::persistence::repositories::{
    SettingsRepository, SourceRepository, TaskRepository, KEY_DOWNLOAD_DIR,
};
use std::path::PathBuf;

fn temp_db_path(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tide-m0-test-{}-{}", label, ids::new_id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir.join("tide.db")
}

fn make_source_and_task() -> (Source, Task) {
    let source = Source::new(
        ids::new_id(),
        SourceType::Http,
        "Integration source".to_string(),
    )
    .expect("valid source");
    let task = Task::new(
        ids::new_id(),
        source.source_id.clone(),
        "Integration task".to_string(),
        TaskState::Created,
    )
    .expect("valid task");
    (source, task)
}

#[test]
fn create_persist_reload_delete_roundtrip() {
    let db_path = temp_db_path("roundtrip");
    let repo = TaskRepository;
    let source_repo = SourceRepository;

    // First "app run": create source + task.
    {
        let conn = open(&db_path).expect("open db");
        let (source, task) = make_source_and_task();
        source_repo.insert(&conn, &source).unwrap();
        repo.insert(&conn, &task).unwrap();
        let task_id = task.task_id.clone();
        drop(conn); // simulate app shutdown
        assert!(!task_id.is_empty());
    }

    // Second "app run": the task must still exist.
    {
        let conn = open(&db_path).expect("reopen db");
        let tasks = repo.list(&conn).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "Integration task");
        assert_eq!(tasks[0].state, TaskState::Created);
    }
    let _ = std::fs::remove_dir_all(db_path.parent().unwrap());
}

#[test]
fn restart_survives_and_state_update_persists() {
    let db_path = temp_db_path("state-update");
    let repo = TaskRepository;
    let source_repo = SourceRepository;

    let (source, mut task) = make_source_and_task();
    {
        let conn = open(&db_path).unwrap();
        source_repo.insert(&conn, &source).unwrap();
        repo.insert(&conn, &task).unwrap();
    }
    {
        let conn = open(&db_path).unwrap();
        task.transition_to(TaskState::Ready).unwrap();
        task.transition_to(TaskState::Running).unwrap();
        task.transition_to(TaskState::Paused).unwrap();
        repo.update(&conn, &task).unwrap();
    }
    {
        let conn = open(&db_path).unwrap();
        let reloaded = repo.get(&conn, &task.task_id).unwrap();
        assert_eq!(reloaded.state, TaskState::Paused);
    }
    let _ = std::fs::remove_dir_all(db_path.parent().unwrap());
}

#[test]
fn crash_like_drop_reopens_without_corruption() {
    let db_path = temp_db_path("crash-like");
    let repo = TaskRepository;
    let source_repo = SourceRepository;

    let (source, task) = make_source_and_task();
    {
        let conn = open(&db_path).unwrap();
        source_repo.insert(&conn, &source).unwrap();
        repo.insert(&conn, &task).unwrap();
        // Simulate a hard kill: drop the connection without any
        // checkpoint/close ceremony.
        drop(conn);
    }
    {
        // WAL must recover; the committed task must survive.
        let conn = open(&db_path).unwrap();
        let recovered = repo.get(&conn, &task.task_id);
        assert!(recovered.is_ok(), "task must survive a hard process drop");
    }
    let _ = std::fs::remove_dir_all(db_path.parent().unwrap());
}

#[test]
fn delete_removes_task_and_source_without_orphans() {
    let db_path = temp_db_path("delete");
    let repo = TaskRepository;
    let source_repo = SourceRepository;

    let (source, task) = make_source_and_task();
    let conn = open(&db_path).unwrap();
    source_repo.insert(&conn, &source).unwrap();
    repo.insert(&conn, &task).unwrap();

    repo.delete(&conn, &task.task_id).unwrap();
    source_repo.delete(&conn, &source.source_id).unwrap();

    assert!(matches!(
        repo.get(&conn, &task.task_id),
        Err(TideError::NotFound(_))
    ));
    assert!(matches!(
        source_repo.get(&conn, &source.source_id),
        Err(TideError::NotFound(_))
    ));
    assert_eq!(repo.list(&conn).unwrap().len(), 0);
    let _ = std::fs::remove_dir_all(db_path.parent().unwrap());
}

#[test]
fn foreign_keys_prevent_orphan_tasks() {
    let db_path = temp_db_path("fk");
    let repo = TaskRepository;
    let conn = open(&db_path).unwrap();

    let task = Task::new(
        ids::new_id(),
        ids::new_id(),
        "orphan".into(),
        TaskState::Created,
    )
    .unwrap();
    // source_id does not exist → the FK constraint must reject the insert.
    assert!(repo.insert(&conn, &task).is_err());
    let _ = std::fs::remove_dir_all(db_path.parent().unwrap());
}

#[test]
fn settings_persist_across_restarts() {
    let db_path = temp_db_path("settings");
    let settings = SettingsRepository;
    {
        let conn = open(&db_path).unwrap();
        settings
            .set(&conn, KEY_DOWNLOAD_DIR, "C:\\Downloads\\Tide")
            .unwrap();
    }
    {
        let conn = open(&db_path).unwrap();
        assert_eq!(
            settings.get(&conn, KEY_DOWNLOAD_DIR).unwrap(),
            Some("C:\\Downloads\\Tide".to_string())
        );
    }
    let _ = std::fs::remove_dir_all(db_path.parent().unwrap());
}
