use std::{
    ffi::{OsStr, OsString},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
    thread::{self, JoinHandle},
    time::Instant,
};

use agentpet_core::{default_data_directory, Core, SessionOutcome};
use uuid::Uuid;

const STATUS_SESSION_LIMIT: usize = 20;

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    let mut args = std::env::args_os().skip(1);
    match args.next().as_deref() {
        Some(command) if command == OsStr::new("run") => run_wrapped_command(args),
        Some(command) if command == OsStr::new("agents") => run_agents_command(args),
        Some(command) if command == OsStr::new("pets") => run_pets_command(args),
        Some(command) if command == OsStr::new("status") => run_status(args),
        Some(command) if command == OsStr::new("doctor") => run_doctor(args),
        Some(command) if command == OsStr::new("--help") || command == OsStr::new("help") => {
            print_help();
            0
        }
        _ => {
            print_help();
            2
        }
    }
}

fn run_wrapped_command(mut args: impl Iterator<Item = OsString>) -> i32 {
    if args.next().as_deref() != Some(OsStr::new("--")) {
        eprintln!("agentpet: expected `agentpet run -- <command> [args...]`");
        return 2;
    }
    let Some(program) = args.next() else {
        eprintln!("agentpet: command is required after `--`");
        return 2;
    };
    let child_args: Vec<OsString> = args.collect();
    let session_id = Uuid::new_v4().to_string();
    let core = match Core::open_default() {
        Ok(core) => Some(core),
        Err(error) => {
            eprintln!("agentpet: local tracking is unavailable; running command without tracking: {error}");
            None
        }
    };
    let project_path = match std::env::current_dir() {
        Ok(path) => Some(path.to_string_lossy().into_owned()),
        Err(error) => {
            eprintln!("agentpet: could not read current directory for session metadata: {error}");
            None
        }
    };
    let agent_id = detect_agent_id(&program).unwrap_or("universal");
    let tracking = core.as_ref().and_then(|runtime| {
        match runtime.start_session(&session_id, agent_id, project_path.as_deref()) {
            Ok(()) => Some(()),
            Err(error) => {
                eprintln!("agentpet: session tracking could not start; running command without tracking: {error}");
                None
            }
        }
    });

    let mut child = match Command::new(&program)
        .args(&child_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            eprintln!("agentpet: failed to start requested command: {error}");
            if tracking.is_some() {
                finish_tracking(&core, &session_id, SessionOutcome::Failed, Some(127), 0);
            }
            return 127;
        }
    };
    let started = Instant::now();
    let (stdout_thread, stderr_thread) = match start_forwarders(&mut child) {
        Ok(threads) => threads,
        Err(error) => {
            eprintln!(
                "agentpet: could not capture child output; terminating wrapper process: {error}"
            );
            stop_child(&mut child);
            if tracking.is_some() {
                finish_tracking(
                    &core,
                    &session_id,
                    SessionOutcome::Failed,
                    Some(125),
                    elapsed_ms(started),
                );
            }
            return 125;
        }
    };

    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            eprintln!("agentpet: failed while waiting for child process: {error}");
            stop_child(&mut child);
            join_forwarder(stdout_thread, "stdout");
            join_forwarder(stderr_thread, "stderr");
            if tracking.is_some() {
                finish_tracking(
                    &core,
                    &session_id,
                    SessionOutcome::Interrupted,
                    None,
                    elapsed_ms(started),
                );
            }
            return 125;
        }
    };
    join_forwarder(stdout_thread, "stdout");
    join_forwarder(stderr_thread, "stderr");

    let outcome = if status.success() {
        SessionOutcome::Completed
    } else if status.code().is_none() {
        SessionOutcome::Interrupted
    } else {
        SessionOutcome::Failed
    };
    let exit_code = child_exit_code(&status);
    if tracking.is_some() {
        finish_tracking(
            &core,
            &session_id,
            outcome,
            status.code(),
            elapsed_ms(started),
        );
    }
    exit_code
}

fn stop_child(child: &mut Child) {
    match child.kill() {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {}
        Err(error) => eprintln!("agentpet: could not terminate child after wrapper error: {error}"),
    }
    if let Err(error) = child.wait() {
        eprintln!("agentpet: could not reap child after wrapper error: {error}");
    }
}

type ForwarderThread = JoinHandle<io::Result<u64>>;

fn start_forwarders(child: &mut Child) -> io::Result<(ForwarderThread, ForwarderThread)> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("child stdout pipe is unavailable"))?;
    let stdout_thread = thread::Builder::new()
        .name("agentpet-stdout".into())
        .spawn(move || forward_stdout(stdout))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("child stderr pipe is unavailable"))?;
    match thread::Builder::new()
        .name("agentpet-stderr".into())
        .spawn(move || forward_stderr(stderr))
    {
        Ok(stderr_thread) => Ok((stdout_thread, stderr_thread)),
        Err(error) => {
            drop(stdout_thread);
            Err(error)
        }
    }
}

fn forward_stdout(mut reader: ChildStdout) -> io::Result<u64> {
    let mut output = io::stdout().lock();
    let copied = io::copy(&mut reader, &mut output)?;
    output.flush()?;
    Ok(copied)
}

fn forward_stderr(mut reader: ChildStderr) -> io::Result<u64> {
    let mut output = io::stderr().lock();
    let copied = io::copy(&mut reader, &mut output)?;
    output.flush()?;
    Ok(copied)
}

fn join_forwarder(handle: JoinHandle<io::Result<u64>>, stream: &str) {
    match handle.join() {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => {
            eprintln!("agentpet: {stream} forwarding ended with an I/O error: {error}")
        }
        Err(_) => eprintln!("agentpet: {stream} forwarding thread failed"),
    }
}

fn finish_tracking(
    core: &Option<Core>,
    session_id: &str,
    outcome: SessionOutcome,
    exit_code: Option<i32>,
    duration_ms: u64,
) {
    if let Some(core) = core {
        if let Err(error) = core.finish_session(session_id, outcome, exit_code, duration_ms) {
            eprintln!("agentpet: session result could not be persisted: {error}");
        }
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn child_exit_code(status: &std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        return 128 + status.signal().unwrap_or(1);
    }
    #[cfg(not(unix))]
    {
        1
    }
}

#[derive(Clone, Copy)]
struct AgentCli {
    id: &'static str,
    label: &'static str,
    executable: &'static str,
}

const AGENT_CLIS: &[AgentCli] = &[
    AgentCli {
        id: "claude_code",
        label: "Claude Code",
        executable: "claude",
    },
    AgentCli {
        id: "codex",
        label: "OpenAI Codex CLI",
        executable: "codex",
    },
    AgentCli {
        id: "gemini_cli",
        label: "Gemini CLI",
        executable: "gemini",
    },
    AgentCli {
        id: "aider",
        label: "Aider",
        executable: "aider",
    },
    AgentCli {
        id: "opencode",
        label: "OpenCode",
        executable: "opencode",
    },
];

fn detect_agent_id(program: &OsStr) -> Option<&'static str> {
    let executable = Path::new(program)
        .file_stem()?
        .to_str()?
        .to_ascii_lowercase();
    AGENT_CLIS
        .iter()
        .find(|agent| agent.executable == executable)
        .map(|agent| agent.id)
}

fn find_on_path(executable: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for directory in
        std::env::split_paths(&path).filter(|directory| !directory.as_os_str().is_empty())
    {
        #[cfg(windows)]
        let candidates = vec![
            directory.join(format!("{executable}.exe")),
            directory.join(format!("{executable}.cmd")),
            directory.join(format!("{executable}.bat")),
            directory.join(executable),
        ];
        #[cfg(not(windows))]
        let candidates = vec![directory.join(executable)];

        for candidate in candidates {
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn run_agents_command(mut args: impl Iterator<Item = OsString>) -> i32 {
    if args.next().as_deref() != Some(OsStr::new("list")) || args.next().is_some() {
        eprintln!("agentpet: usage: agentpet agents list");
        return 2;
    }
    println!("PATH discovery only; this does not install or configure native agent hooks.");
    for agent in AGENT_CLIS {
        match find_on_path(agent.executable) {
            Some(path) => println!(
                "Found {} PATH entry at {} (launch compatibility is not verified)",
                agent.label,
                path.display()
            ),
            None => println!("Not found: {} ({})", agent.label, agent.executable),
        }
    }
    0
}

fn open_local_core() -> Result<Core, i32> {
    Core::open_default().map_err(|error| {
        eprintln!("agentpet: local runtime is unavailable: {error}");
        1
    })
}

fn run_pets_command(mut args: impl Iterator<Item = OsString>) -> i32 {
    if args.next().as_deref() != Some(OsStr::new("list")) || args.next().is_some() {
        eprintln!("agentpet: usage: agentpet pets list");
        return 2;
    }
    let core = match open_local_core() {
        Ok(core) => core,
        Err(exit_code) => return exit_code,
    };
    let snapshot = match core.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("agentpet: could not read selected companion: {error}");
            return 1;
        }
    };
    let packs = match core.list_installed_packs() {
        Ok(packs) => packs,
        Err(error) => {
            eprintln!("agentpet: could not list installed companions: {error}");
            return 1;
        }
    };
    if packs.is_empty() {
        println!("No PetPacks are installed.");
        return 0;
    }
    for pack in packs {
        let selected = if snapshot.selected_pet_id.as_deref() == Some(pack.selection_id.as_str()) {
            " [selected]"
        } else {
            ""
        };
        println!(
            "{} {} by {} ({}){}",
            pack.selection_id, pack.name, pack.creator, pack.license, selected
        );
    }
    0
}

fn run_status(mut args: impl Iterator<Item = OsString>) -> i32 {
    if args.next().is_some() {
        eprintln!("agentpet: usage: agentpet status");
        return 2;
    }
    let core = match open_local_core() {
        Ok(core) => core,
        Err(exit_code) => return exit_code,
    };
    let snapshot = match core.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("agentpet: could not read runtime status: {error}");
            return 1;
        }
    };
    let sessions = match core.session_history(STATUS_SESSION_LIMIT, 0) {
        Ok(sessions) => sessions,
        Err(error) => {
            eprintln!("agentpet: could not read session status: {error}");
            return 1;
        }
    };
    let running_sessions = sessions
        .iter()
        .filter(|session| session.status == "running")
        .count();
    println!("Pet state: {:?}", snapshot.pet_state);
    println!(
        "Selected companion: {}",
        snapshot.selected_pet_id.as_deref().unwrap_or("none")
    );
    println!("Running sessions: {running_sessions}");
    println!("Recent sessions inspected: {}", sessions.len());
    match snapshot.last_event_at {
        Some(timestamp) => println!("Last accepted event: {timestamp}"),
        None => println!("Last accepted event: none"),
    }
    0
}

fn run_doctor(mut args: impl Iterator<Item = OsString>) -> i32 {
    if args.next().is_some() {
        eprintln!("agentpet: usage: agentpet doctor");
        return 2;
    }
    println!("AgentPet local diagnostics");
    println!(
        "[OK] Platform: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    match std::env::current_exe() {
        Ok(path) => println!("[OK] CLI: {}", path.display()),
        Err(error) => println!("[WARN] CLI path unavailable: {error}"),
    }
    match default_data_directory() {
        Ok(path) => println!("[OK] Data directory: {}", path.display()),
        Err(error) => println!("[WARN] Data directory unavailable: {error}"),
    }
    let mut failed = false;
    match Core::open_default() {
        Ok(core) => match core.list_installed_packs() {
            Ok(packs) => println!(
                "[OK] Local database: accessible ({} installed pet pack(s))",
                packs.len()
            ),
            Err(error) => {
                println!("[FAIL] Local database query failed: {error}");
                failed = true;
            }
        },
        Err(error) => {
            println!("[FAIL] Local database unavailable: {error}");
            failed = true;
        }
    }
    for agent in AGENT_CLIS {
        match find_on_path(agent.executable) {
            Some(path) => println!(
                "[OK] Agent CLI PATH entry found: {} ({})",
                agent.label,
                path.display()
            ),
            None => println!("[WARN] Optional agent CLI not found: {}", agent.label),
        }
    }
    println!("[WARN] Native hooks, cloud sync, and credential-store checks are not implemented.");
    if failed {
        1
    } else {
        0
    }
}

fn print_help() {
    println!("AgentPet local CLI\n\nUSAGE:\n  agentpet run -- <command> [args...]\n  agentpet status\n  agentpet pets list\n  agentpet agents list\n  agentpet doctor\n");
}

#[cfg(test)]
mod tests {
    use super::detect_agent_id;
    use std::ffi::OsStr;

    #[test]
    fn identifies_known_agent_executable_names() {
        assert_eq!(detect_agent_id(OsStr::new("claude")), Some("claude_code"));
        assert_eq!(detect_agent_id(OsStr::new("codex.exe")), Some("codex"));
        assert_eq!(detect_agent_id(OsStr::new("gemini")), Some("gemini_cli"));
    }

    #[test]
    fn leaves_unknown_commands_as_universal() {
        assert_eq!(detect_agent_id(OsStr::new("custom-agent")), None);
    }
}
