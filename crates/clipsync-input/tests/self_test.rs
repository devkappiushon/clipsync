//! `self_test()` (§12.6, L7) against the real system: it must answer by itself, quickly and with
//! a detail, whatever the machine has. A dialog would block it until the timeout below.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_input::{SelfTestOutcome, self_test};
use std::time::{Duration, Instant};

#[tokio::test]
async fn answers_without_a_dialog_and_with_a_detail_in_portuguese() {
    let started = Instant::now();
    let outcome = tokio::time::timeout(Duration::from_secs(8), self_test())
        .await
        .expect("self_test waited for something: a dialog or a hung portal");
    let (kind, detail) = match &outcome {
        SelfTestOutcome::Validated(d) => ("validated", d),
        SelfTestOutcome::Experimental(d) => ("experimental", d),
        SelfTestOutcome::Unavailable(d) => ("unavailable", d),
    };
    println!("self_test: {kind} em {:?}: {detail}", started.elapsed());
    assert!(!detail.trim().is_empty());
    assert!(
        !detail.contains('\n') && !detail.contains('\u{2014}'),
        "{detail}"
    );
    if cfg!(not(target_os = "linux")) {
        assert!(matches!(outcome, SelfTestOutcome::Unavailable(_)));
    }
}
