//! Child processes for the GUI app. On Windows a console program started from a GUI process opens its own console
//! window unless it is created with CREATE_NO_WINDOW, so every tool (ffmpeg, ffprobe) is launched through here.

use std::ffi::OsStr;
use std::process::Command;

pub fn hidden_command(exe: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}
