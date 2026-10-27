#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // helpers outside #[test] fns
#[tokio::test]
async fn system_diag_reports_os_session_display_and_both_input_tools() {
    let items = clipsync_core::diag::system().await;
    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
    for want in [
        "sistema operacional",
        "sessão gráfica",
        "DISPLAY / XWayland",
        "Input Leap",
        "Barrier",
    ] {
        assert!(names.contains(&want), "{want} missing in {names:?}");
    }
}
