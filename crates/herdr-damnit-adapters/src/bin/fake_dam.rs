//! A stand-in for `dam`, driven entirely by its environment. Every test that needs a `dam` points
//! the pane's configured `dam` argv at this binary, so no test mangles `PATH` and no test needs a
//! real store.

use std::io::Write;
use std::path::PathBuf;

fn main() -> std::process::ExitCode {
    install_interrupt_handler();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    log(&argv);

    if let Ok(millis) = std::env::var("FAKE_DAM_SLEEP_MS") {
        let millis = millis.parse().unwrap_or(0);
        std::thread::sleep(std::time::Duration::from_millis(millis));
    }

    if let Ok(code) = std::env::var("FAKE_DAM_EXIT") {
        let line = std::env::var("FAKE_DAM_STDERR").unwrap_or_default();
        if !line.is_empty() {
            eprintln!("{line}");
        }
        return std::process::ExitCode::from(code.parse::<u8>().unwrap_or(1));
    }

    if argv.first().map(String::as_str) == Some("--version") {
        print!(
            "{}",
            read("version.txt").unwrap_or_else(|| "dam 0.2.0\n".to_string())
        );
        return std::process::ExitCode::SUCCESS;
    }

    let name = std::env::var("FAKE_DAM_FIXTURE")
        .unwrap_or_else(|_| argv.first().cloned().unwrap_or_default());
    println!(
        "{}",
        read(&format!("{name}.json"))
            .unwrap_or_else(|| "{}".to_string())
            .trim()
    );
    std::process::ExitCode::SUCCESS
}

fn fixture_dir() -> Option<PathBuf> {
    std::env::var_os("FAKE_DAM_FIXTURE_DIR").map(PathBuf::from)
}

fn read(name: &str) -> Option<String> {
    std::fs::read_to_string(fixture_dir()?.join(name)).ok()
}

/// One JSON line of full argv per call, appended so a test reads every call one run made.
fn log(argv: &[String]) {
    append(&serde_json::to_string(argv).unwrap_or_default());
}

fn append(line: &str) {
    let Some(path) = std::env::var_os("FAKE_DAM_LOG") else {
        return;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

/// `dam` maps a cancelled run to exit 3 whatever the abandoned work reported, so the fake does the
/// same and records the signal for the test to assert on.
fn install_interrupt_handler() {
    /// # Safety
    ///
    /// The append and the exit are not async-signal-safe. This binary is a test double whose only
    /// other thread is the sleep in `main`, so the handler cannot re-enter a lock its own process
    /// holds, and the process ends in the handler either way.
    unsafe extern "C" fn on_interrupt(_: libc::c_int) {
        append(r#"{"signal":"SIGINT"}"#);
        std::process::exit(3);
    }
    // Safety: `signal` with a plain function pointer is defined for SIGINT on every platform this
    // repository builds for.
    unsafe {
        libc::signal(
            libc::SIGINT,
            on_interrupt as *const () as libc::sighandler_t,
        );
    }
}
