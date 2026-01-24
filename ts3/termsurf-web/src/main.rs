use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Represents the profile mode for CEF
#[derive(Debug, Clone)]
enum ProfileMode {
    /// Named profile with persistent storage
    Named(String),
    /// Incognito mode - no persistent storage
    Incognito,
}

impl ProfileMode {
    fn cache_path(&self) -> Option<PathBuf> {
        match self {
            ProfileMode::Named(name) => {
                let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                Some(PathBuf::from(format!("{}/.config/termsurf/cef/{}", home, name)))
            }
            ProfileMode::Incognito => None,
        }
    }

    fn display_name(&self) -> &str {
        match self {
            ProfileMode::Named(name) => name.as_str(),
            ProfileMode::Incognito => "incognito",
        }
    }
}

/// Validates a profile name: lowercase alphanumeric, must start with a letter
fn validate_profile_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Profile name cannot be empty".into());
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_lowercase() {
        return Err("Profile name must start with a lowercase letter".into());
    }

    for c in name.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() {
            return Err(format!(
                "Profile name must be lowercase alphanumeric, found '{}'",
                c
            ));
        }
    }

    Ok(())
}

/// Parses command line arguments and returns the profile mode
fn parse_args() -> Result<(bool, ProfileMode), String> {
    let args: Vec<String> = env::args().collect();

    let is_subprocess = args.iter().any(|a| a == "--browser-subprocess");
    let has_incognito = args.iter().any(|a| a == "--incognito");

    // Find --profile value
    let profile_value = args
        .iter()
        .position(|a| a == "--profile")
        .map(|i| args.get(i + 1).cloned())
        .flatten();

    // Check for mutual exclusivity
    if has_incognito && profile_value.is_some() {
        return Err("Cannot specify both --incognito and --profile".into());
    }

    let profile_mode = if has_incognito {
        ProfileMode::Incognito
    } else if let Some(name) = profile_value {
        validate_profile_name(&name)?;
        ProfileMode::Named(name)
    } else {
        // Default profile
        ProfileMode::Named("default".to_string())
    };

    Ok((is_subprocess, profile_mode))
}

#[cfg(target_os = "macos")]
fn load_cef(profile: &ProfileMode) -> Result<(), String> {
    use cef::args::Args;
    use cef::library_loader::LibraryLoader;
    use cef::{api_hash, execute_process, initialize, sys, CefString, Settings};

    let exe = env::current_exe().map_err(|e| format!("current_exe: {e}"))?;

    // Load CEF framework
    let loader = LibraryLoader::new(&exe, false);
    if !loader.load() {
        return Err("Failed to load CEF framework".into());
    }

    // Configure CEF API version
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let args = Args::new();

    // Check if we're a subprocess (renderer, GPU, etc.)
    let ret = execute_process(
        Some(args.as_main_args()),
        None::<&mut cef::App>,
        std::ptr::null_mut(),
    );
    if ret >= 0 {
        // We're a CEF subprocess, exit with the return code
        std::process::exit(ret);
    }

    // Set up cache path based on profile
    let cache_path_str = match profile.cache_path() {
        Some(path) => {
            let _ = std::fs::create_dir_all(&path);
            path.to_string_lossy().to_string()
        }
        None => {
            // Incognito: use empty string (CEF will use in-memory storage)
            String::new()
        }
    };

    // Compute path to helper binary
    // exe is: .../wezterm-gui.app/Contents/MacOS/web
    // helper is: .../wezterm-gui.app/Contents/Frameworks/WezTerm Helper.app/Contents/MacOS/WezTerm Helper
    let helper_path = exe
        .parent()
        .unwrap() // MacOS
        .parent()
        .unwrap() // Contents
        .join("Frameworks")
        .join("WezTerm Helper.app")
        .join("Contents")
        .join("MacOS")
        .join("WezTerm Helper");
    let helper_path_str = helper_path.to_string_lossy().to_string();

    let settings = Settings {
        windowless_rendering_enabled: 1,
        external_message_pump: 1,
        no_sandbox: 1,
        root_cache_path: CefString::from(cache_path_str.as_str()),
        browser_subprocess_path: CefString::from(helper_path_str.as_str()),
        ..Default::default()
    };

    if initialize(
        Some(args.as_main_args()),
        Some(&settings),
        None::<&mut cef::App>,
        std::ptr::null_mut(),
    ) != 1
    {
        return Err("CEF initialize failed".into());
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn load_cef(_profile: &ProfileMode) -> Result<(), String> {
    Err("CEF loading not yet implemented for this platform".into())
}

fn run_browser_subprocess(profile: ProfileMode) {
    println!("Browser subprocess starting with profile={}", profile.display_name());

    match load_cef(&profile) {
        Ok(()) => {
            println!("loaded CEF with profile={}", profile.display_name());
            // Shutdown CEF
            #[cfg(target_os = "macos")]
            cef::shutdown();
        }
        Err(e) => {
            eprintln!("Failed to load CEF: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_coordinator(profile: ProfileMode) {
    let exe = env::current_exe().expect("Failed to get current executable path");

    println!(
        "Coordinator: spawning browser subprocess with profile={}...",
        profile.display_name()
    );

    let mut cmd = Command::new(&exe);
    cmd.arg("--browser-subprocess");

    // Pass profile to subprocess
    match &profile {
        ProfileMode::Named(name) => {
            cmd.arg("--profile").arg(name);
        }
        ProfileMode::Incognito => {
            cmd.arg("--incognito");
        }
    }

    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn browser subprocess");

    let output = child
        .wait_with_output()
        .expect("Failed to wait for subprocess");

    println!(
        "Subprocess stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    if !output.stderr.is_empty() {
        eprintln!(
            "Subprocess stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    println!("Subprocess exited with: {}", output.status);
}

fn main() {
    match parse_args() {
        Ok((is_subprocess, profile)) => {
            if is_subprocess {
                run_browser_subprocess(profile);
            } else {
                run_coordinator(profile);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Usage: web [--profile <name>] [--incognito]");
            eprintln!();
            eprintln!("Options:");
            eprintln!("  --profile <name>  Use named profile (default: 'default')");
            eprintln!("  --incognito       Use incognito mode (no persistent storage)");
            eprintln!();
            eprintln!("Profile names must be lowercase alphanumeric and start with a letter.");
            std::process::exit(1);
        }
    }
}
