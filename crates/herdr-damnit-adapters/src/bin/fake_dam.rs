use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

const DAMS_EXIT_WHEN_CANCELLED: i32 = 3;

const KNOB_PREFIX: &str = "FAKE_DAM_";

static KNOBS_BEFORE_THE_ARGV: OnceLock<Vec<(String, String)>> = OnceLock::new();

fn main() -> std::process::ExitCode {
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    let knobs = argv
        .iter()
        .map_while(|word| {
            word.split_once('=')
                .filter(|(name, _)| name.starts_with(KNOB_PREFIX))
        })
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect::<Vec<_>>();
    argv.drain(..knobs.len());
    let _ = KNOBS_BEFORE_THE_ARGV.set(knobs);
    install_interrupt_handler();
    append_argv_as_one_json_line(&argv);

    if let Some(millis) = knob("FAKE_DAM_SLEEP_MS") {
        let millis = millis.parse().unwrap_or(0);
        std::thread::sleep(std::time::Duration::from_millis(millis));
    }

    if let Some(code) = knob("FAKE_DAM_EXIT") {
        let line = knob("FAKE_DAM_STDERR").unwrap_or_default();
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

    let name =
        knob("FAKE_DAM_FIXTURE").unwrap_or_else(|| argv.first().cloned().unwrap_or_default());
    println!(
        "{}",
        read(&format!("{name}.json"))
            .unwrap_or_else(|| "{}".to_string())
            .trim()
    );
    std::process::ExitCode::SUCCESS
}

fn knob(name: &str) -> Option<String> {
    let given = KNOBS_BEFORE_THE_ARGV.get().into_iter().flatten();
    given
        .filter(|(given, _)| given == name)
        .map(|(_, value)| value.clone())
        .next()
        .or_else(|| std::env::var(name).ok())
}

fn fixture_dir() -> Option<PathBuf> {
    knob("FAKE_DAM_FIXTURE_DIR").map(PathBuf::from)
}

fn read(name: &str) -> Option<String> {
    std::fs::read_to_string(fixture_dir()?.join(name)).ok()
}

fn append_argv_as_one_json_line(argv: &[String]) {
    append(&serde_json::to_string(argv).unwrap_or_default());
}

fn append(line: &str) {
    let Some(path) = knob("FAKE_DAM_LOG") else {
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

fn install_interrupt_handler() {
    if knob("FAKE_DAM_IGNORE_SIGINT").is_some() {
        ignore_interrupts_which_hands_over_no_code();
        return;
    }
    log_and_exit_on_interrupt_which_the_tests_send_only_while_asleep();
}

extern "C" fn record_the_interrupt_and_exit_cancelled(_: libc::c_int) {
    append(r#"{"signal":"SIGINT"}"#);
    std::process::exit(DAMS_EXIT_WHEN_CANCELLED);
}

fn ignore_interrupts_which_hands_over_no_code() {
    unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
}

fn log_and_exit_on_interrupt_which_the_tests_send_only_while_asleep() {
    let handler = record_the_interrupt_and_exit_cancelled as *const () as libc::sighandler_t;
    unsafe { libc::signal(libc::SIGINT, handler) };
}
