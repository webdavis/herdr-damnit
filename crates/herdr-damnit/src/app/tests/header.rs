use super::*;

#[test]
fn the_header_names_the_exclusive_job_its_spinner_and_its_elapsed_time() {
    let mut harness = harness();
    let started = harness.started();
    harness.app.submit(
        JobKind::Exclusive(SyncKind::Push),
        herdr_damnit_application::argv::push(),
    );
    harness.app.tick(started);

    let header = harness.app.header(started + Duration::from_millis(3200));
    assert!(header.contains("push"), "{header}");
    assert!(header.contains("3.2s"), "{header}");
}

#[test]
fn elapsed_is_tenths_under_ten_seconds_and_whole_seconds_after_it() {
    let mut harness = harness();
    let started = harness.started();
    harness.app.submit(
        JobKind::Exclusive(SyncKind::Pull),
        herdr_damnit_application::argv::pull(),
    );

    assert!(
        harness
            .app
            .header(started + Duration::from_millis(1500))
            .contains("1.5s")
    );
    assert!(
        harness
            .app
            .header(started + Duration::from_secs(42))
            .contains("42s")
    );
}

#[test]
fn two_reads_in_flight_are_counted_rather_than_named() {
    let mut harness = harness();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness
        .app
        .submit(JobKind::ReadLog, herdr_damnit_application::argv::log());

    let header = harness.app.header(Instant::now());
    assert!(header.contains("2 reads"), "{header}");
}

#[test]
fn the_spinner_steps_one_frame_per_tick() {
    let mut harness = harness();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );

    let first = harness.app.spinner;
    harness.app.tick(Instant::now());
    assert_ne!(harness.app.spinner, first);
}
