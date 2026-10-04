//! one instance of the overlay at a time, the overlay has no console or close button,
//! so without this, subsequent launches would overlap (e.g., one with admin rights,
//! the other without – both would be visible at once).
use std::{
    process::{Command, Stdio},
    time::Duration,
};
use sysinfo::{Pid, System};

pub struct Report {
    pub killed: usize,
    pub failed: usize,
}

/// name of the file of the overlay (on Linux, process name is limited to 15 characters).
pub fn overlay_exe_name() -> String {
    format!("hudmon-overlay{}", std::env::consts::EXE_SUFFIX)
}

pub fn start_overlay() -> bool {
    let Ok(me) = std::env::current_exe() else {
        return false;
    };
    Command::new(me.with_file_name(overlay_exe_name()))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

pub fn kill_others() -> Report {
    let mut sys = System::new();
    sys.refresh_processes();
    let me = Pid::from_u32(std::process::id());
    let name = overlay_exe_name();
    let targets: Vec<Pid> = sys
        .processes()
        .iter()
        .filter(|(pid, p)| **pid != me && p.name().eq_ignore_ascii_case(&name))
        .map(|(pid, _)| *pid)
        .collect();
    let mut r = Report {
        killed: 0,
        failed: 0,
    };
    for pid in targets {
        for p in sys.processes().values() {
            if p.parent() == Some(pid) {
                p.kill(); // PresentMon processes are not wanted
            }
        }
        match sys.process(pid) {
            Some(p) if p.kill() => r.killed += 1,
            _ => r.failed += 1,
        }
    }
    if r.killed > 0 {
        std::thread::sleep(Duration::from_millis(300));
    }
    r
}

#[cfg(all(test, unix))]
mod tests {
    #[test]
    fn kills_other_instance_and_its_child() {
        use std::{fs, os::unix::fs::PermissionsExt, process::Command};
        let dir = std::env::temp_dir().join("hudmon-inst-test");
        let _ = fs::create_dir_all(&dir);
        let pidfile = dir.join("child.pid");
        let _ = fs::remove_file(&pidfile);
        let fake = dir.join("hudmon-overlay");
        fs::write(
            &fake,
            format!(
                "#!/bin/sh\nsleep 60 &\necho $! > {}\nwait\n",
                pidfile.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
        let mut parent = Command::new(&fake).spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(400));
        let child_pid: u32 = fs::read_to_string(&pidfile)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let r = super::kill_others();
        let _ = parent.wait();
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert_eq!(r.failed, 0);
        assert!(r.killed >= 1, "powinien zamknąć drugą kopię");
        let alive = fs::read_to_string(format!("/proc/{child_pid}/stat"))
            .map_or(false, |s| !s.contains(") Z"));
        assert!(
            !alive,
            "proces potomny (PresentMon) nie może zostać osierocony"
        );
    }
}
