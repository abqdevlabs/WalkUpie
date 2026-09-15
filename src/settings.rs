use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub work_minutes: u64,
    pub break_minutes: u64,
    pub always_on_top: bool,
    pub auto_start: bool,
    pub timer_phase: Option<String>,
    pub timer_remaining_secs: Option<u64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            work_minutes: 60,
            break_minutes: 5,
            always_on_top: true,
            auto_start: false,
            timer_phase: None,
            timer_remaining_secs: None,
        }
    }
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("WalkUpie")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load() -> Settings {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

fn ps_quote(s: &str) -> String {
    s.replace('\'', "''")
}

pub fn set_auto_start(enabled: bool, exe: &Path) -> std::io::Result<()> {
    if enabled {
        create_startup_shortcut(exe)
    } else {
        let script = "$s=[Environment]::GetFolderPath('Startup'); \
                      $lnk=Join-Path $s 'WalkUpie.lnk'; \
                      if(Test-Path $lnk){{Remove-Item $lnk}}";
        run_powershell(&script)
    }
}

pub fn create_desktop_shortcut(exe: &Path) -> std::io::Result<()> {
    create_shortcut(exe, "WalkUpie.lnk", "[Environment]::GetFolderPath('Desktop')")
}

fn create_startup_shortcut(exe: &Path) -> std::io::Result<()> {
    create_shortcut(exe, "WalkUpie.lnk", "[Environment]::GetFolderPath('Startup')")
}

fn create_shortcut(exe: &Path, name: &str, folder_expr: &str) -> std::io::Result<()> {
    let exe = exe.to_string_lossy();
    let dir = exe[..exe.rfind('\\').map(|i| i + 1).unwrap_or(0)].to_string();
    let script = format!(
        "$s={folder_expr}; \
         $lnk=Join-Path $s '{name}'; \
         $w=New-Object -ComObject WScript.Shell; \
         $sc=$w.CreateShortcut($lnk); \
         $sc.TargetPath='{}'; \
         $sc.WorkingDirectory='{}'; \
         $sc.Save()",
        ps_quote(&exe),
        ps_quote(&dir)
    );
    run_powershell(&script)
}

fn run_powershell(script: &str) -> std::io::Result<()> {
    std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", script])
        .status()
        .map(|_| ())
}