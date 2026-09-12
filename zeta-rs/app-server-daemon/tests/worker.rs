#[path = "../../arg0/tests/support/worker.rs"]
mod support;

#[test]
fn executable_runs_the_shared_worker_and_reopens_its_index() {
    support::assert_worker(std::path::Path::new(env!(
        "CARGO_BIN_EXE_zeta-app-server-daemon"
    )));
}
