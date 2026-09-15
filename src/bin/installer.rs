use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use walkupie::settings;

fn read_line() -> String {
    let mut s = String::new();
    io::stdin().read_line(&mut s).ok();
    s.trim().to_string()
}

fn ask(question: &str, default: &str) -> String {
    print!("{question} [default: {default}]: ");
    let _ = io::stdout().flush();
    let answer = read_line();
    if answer.is_empty() {
        default.to_string()
    } else {
        answer
    }
}

fn parse_minutes(value: &str, default: u64) -> u64 {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|v| *v >= 1)
        .unwrap_or(default)
}

fn parse_bool(value: &str, default: bool) -> bool {
    match value.trim().to_lowercase().as_str() {
        "y" | "yes" | "true" | "1" => true,
        "n" | "no" | "false" | "0" => false,
        _ => default,
    }
}

fn default_install_dir() -> PathBuf {
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    local.join("Programs").join("WalkUpie")
}

fn banner() {
    println!("========================================");
    println!("  WalkUpie — break-timer widget installer");
    println!("========================================");
}

fn main() -> ExitCode {
    banner();

    let install_dir = ask(
        "Install folder",
        &default_install_dir().to_string_lossy(),
    );
    let install_dir = PathBuf::from(install_dir.trim());

    let work = parse_minutes(&ask("Work duration (minutes)", "60"), 60);
    let brk = parse_minutes(&ask("Break duration (minutes)", "5"), 5);
    let auto = parse_bool(&ask("Start with Windows (y/n)", "n"), false);

    println!("\nInstalling with: work {work} min, break {brk} min, auto-start {auto} ...\n");

    let source = match std::env::current_exe() {
        Ok(exe) => exe
            .parent()
            .unwrap_or(Path::new("."))
            .join("walkupie.exe"),
        Err(e) => {
            eprintln!("Could not locate installer: {e}");
            return ExitCode::FAILURE;
        }
    };
    if !source.exists() {
        eprintln!(
            "Missing 'walkupie.exe' next to the installer. Put both files in the same folder."
        );
        return ExitCode::FAILURE;
    }

    if let Err(e) = std::fs::create_dir_all(&install_dir) {
        eprintln!("Could not create install folder '{}': {e}", install_dir.display());
        return ExitCode::FAILURE;
    }
    let dest = install_dir.join("walkupie.exe");
    if let Err(e) = std::fs::copy(&source, &dest) {
        eprintln!("Could not copy walkupie.exe: {e}");
        return ExitCode::FAILURE;
    }
    println!("  copied -> {}", dest.display());

    let mut s = settings::Settings::default();
    s.work_minutes = work;
    s.break_minutes = brk;
    s.auto_start = auto;
    s.timer_phase = None;
    s.timer_remaining_secs = None;
    match settings::save(&s) {
        Ok(()) => println!("  wrote -> {}", settings::settings_path().display()),
        Err(e) => {
            eprintln!("Could not write settings: {e}");
            return ExitCode::FAILURE;
        }
    }

    if auto {
        match settings::set_auto_start(true, &dest) {
            Ok(()) => println!("  added startup shortcut"),
            Err(e) => eprintln!("  warning: could not create startup shortcut: {e}"),
        }
    } else {
        let _ = settings::set_auto_start(false, &dest);
    }

    match settings::create_desktop_shortcut(&dest) {
        Ok(()) => println!("  added desktop shortcut"),
        Err(e) => eprintln!("  warning: could not create desktop shortcut: {e}"),
    }

    println!();
    println!("Installation complete.");
    println!("  App:      {}", dest.display());
    println!("  Settings: {}", settings::settings_path().display());
    println!();
    println!("You can change work/break minutes and auto-start anytime from the");
    println!("app's Settings window.");
    println!();
    let launch = ask("Press Enter to launch WalkUpie now, or type 'n' to skip", "launch");
    if !launch.to_lowercase().starts_with('n') {
        let _ = std::process::Command::new(&dest).spawn();
    }
    ExitCode::SUCCESS
}