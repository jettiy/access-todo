//! Round-trip test for the local-file cache fallback.
//!
//! Verifies that when no GITHUB_TOKEN is configured, mutations are
//! persisted to a temp cache file and reloaded on the next startup —
//! closing the "in-memory only, data lost on restart" gap.

use std::io::Write;

use api_server::AppState;
use todo_core::store::TodoInput;

#[tokio::test(flavor = "current_thread")]
async fn local_cache_survives_restart() {
    // 임시 캐시 파일. env_var 로 주입.
    let tmp = tempfile::NamedTempFile::new().unwrap();
    // 파일이 존재하지 않는 상태에서 시작하기 위해 비움
    tmp.as_file().set_len(0).ok();
    let path = tmp.path().to_path_buf();
    drop(tmp); // close handle so the AppState can write to it

    std::env::set_var("ACCESS_LOCAL_CACHE", &path);
    std::env::remove_var("GITHUB_TOKEN");
    std::env::remove_var("TODO_GIST_ID");

    // 첫 세션: 항목 추가 → flusher 가 파일에 기록할 시간 대기
    let s1 = AppState::with_gist_from_env();
    assert!(s1.sync.is_none(), "no GITHUB_TOKEN → no Gist engine");
    s1.clone().spawn_flusher();
    {
        let mut st = s1.store.write().await;
        st.add(
            TodoInput {
                title: "persisted".into(),
                ..Default::default()
            },
            "tester",
        );
    }
    assert!(s1.mark_push_pending());

    // debounce window + margin
    tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
    drop(s1);

    // 파일에 JSON 이 쓰였는지 확인
    let bytes = std::fs::read(&path).unwrap();
    let s = String::from_utf8(bytes).unwrap();
    assert!(s.contains("persisted"), "cache file should contain the added todo: {s}");

    // 두 번째 세션: 캐시에서 복원
    let s2 = AppState::with_gist_from_env();
    s2.load_local_cache().await.unwrap();
    let st = s2.store.read().await;
    let titles: Vec<_> = st.list().iter().map(|t| t.title.clone()).collect();
    assert!(
        titles.iter().any(|t| t == "persisted"),
        "after restart, todo must be restored from cache, got {titles:?}"
    );

    std::env::remove_var("ACCESS_LOCAL_CACHE");

    // touch tempfile module so the next build links it even if we extend
    let _ = std::io::sink().write(b"");
}
