//! On Windows a child process started from the GUI app opens a visible console window unless it is created with
//! CREATE_NO_WINDOW. Every tool launch in the library must therefore go through `proc::hidden_command`.

use std::path::Path;

#[cfg(windows)]
#[test]
fn hidden_child_has_no_console_window() {
    let script = r#"Add-Type -MemberDefinition '[System.Runtime.InteropServices.DllImport("kernel32.dll")] public static extern System.IntPtr GetConsoleWindow();' -Name ConsoleProbe -Namespace FireCrackle; [FireCrackle.ConsoleProbe]::GetConsoleWindow().ToInt64()"#;
    let output = studio_core::proc::hidden_command("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .output()
        .expect("start hidden Windows console probe");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0");
}

fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn no_source_starts_a_process_without_the_hidden_helper() {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(files.len() > 5, "found no sources");
    let mut bad = Vec::new();
    for f in files {
        if f.file_name().is_some_and(|n| n == "proc.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap();
        if text.contains("Command::new(") {
            bad.push(f.display().to_string());
        }
    }
    assert!(
        bad.is_empty(),
        "use proc::hidden_command instead of Command::new in: {bad:?}"
    );
}

#[test]
fn the_helper_runs_a_program_and_captures_its_output() {
    let exe = if cfg!(windows) { "cmd" } else { "sh" };
    let args: &[&str] = if cfg!(windows) {
        &["/C", "echo hi"]
    } else {
        &["-c", "echo hi"]
    };
    let out = studio_core::proc::hidden_command(exe)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("hi"));
}
