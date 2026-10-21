use std::{
    ffi::{OsStr, OsString},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
    thread::{self, JoinHandle},
    time::Instant,
};

use agentpet_adapters::{adapter_by_id, ADAPTERS};
use agentpet_core::{default_data_directory, Core, HookInstallRecord, SessionOutcome};
use uuid::Uuid;

mod diagnostics;

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
        Some(command) if command == OsStr::new("hook") => run_hook_command(args),
        Some(command) if command == OsStr::new("pets") => run_pets_command(args),
        Some(command) if command == OsStr::new("status") => run_status(args),
        Some(command) if command == OsStr::new("doctor") => run_doctor(args),
        Some(command) if command == OsStr::new("verify-ledger") => run_verify_ledger(args),
        Some(command) if command == OsStr::new("statusline") => run_statusline(args),
        Some(command) if command == OsStr::new("pack") => run_pack_command(args),
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

fn detect_agent_id(program: &OsStr) -> Option<&'static str> {
    let executable = Path::new(program)
        .file_stem()?
        .to_str()?
        .to_ascii_lowercase();
    ADAPTERS
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
    let Some(subcommand) = args.next() else {
        eprintln!("agentpet: usage: agentpet agents list | agentpet agents install <id> | agentpet agents remove <id> | agentpet agents status [<id>]");
        return 2;
    };
    match subcommand.to_str() {
        Some("list") => run_agents_list(args),
        Some("install") => run_agents_install(args),
        Some("remove") => run_agents_remove(args),
        Some("status") => run_agents_status(args),
        _ => {
            eprintln!("agentpet: usage: agentpet agents list | agentpet agents install <id> | agentpet agents remove <id> | agentpet agents status [<id>]");
            2
        }
    }
}

fn run_agents_list(mut args: impl Iterator<Item = OsString>) -> i32 {
    if args.next().is_some() {
        eprintln!("agentpet: usage: agentpet agents list");
        return 2;
    }
    println!(
        "PATH discovery is informational only; native hook state comes from the local database."
    );
    for agent in ADAPTERS {
        match find_on_path(agent.executable) {
            Some(path) => println!(
                "Found {} PATH entry at {} (launch compatibility is not verified)",
                agent.display_name,
                path.display()
            ),
            None => println!("Not found: {} ({})", agent.display_name, agent.executable),
        }
    }
    let Ok(core) = Core::open_default() else {
        println!("Hook state: local database unavailable");
        return 0;
    };
    match core.hook_installs() {
        Ok(records) if !records.is_empty() => {
            for record in records {
                println!(
                    "Hook {} at {}: {}",
                    record.adapter_id, record.config_path, record.status
                );
            }
        }
        Ok(_) => println!("Hook state: no native hooks recorded"),
        Err(error) => println!("Hook state: unavailable ({error})"),
    }
    0
}

fn hook_command() -> &'static str {
    "agentpet hook --adapter claude_code"
}

fn run_agents_install(mut args: impl Iterator<Item = OsString>) -> i32 {
    let Some(adapter_id) = args.next().and_then(|v| v.into_string().ok()) else {
        eprintln!("agentpet: usage: agentpet agents install <id>");
        return 2;
    };
    if args.next().is_some() {
        eprintln!("agentpet: usage: agentpet agents install <id>");
        return 2;
    }
    let Some(adapter) = adapter_by_id(&adapter_id) else {
        eprintln!("agentpet: unknown agent id `{adapter_id}`");
        return 2;
    };
    if !adapter.native_hook {
        eprintln!(
            "agentpet: {} has no native hook; use `agentpet run`",
            adapter.display_name
        );
        return 2;
    }
    let core = match open_local_core() {
        Ok(core) => core,
        Err(code) => return code,
    };
    install_claude_hook(&core, adapter.display_name)
}

fn install_claude_hook(core: &Core, display_name: &str) -> i32 {
    use agentpet_adapters::claude_code;
    let Some(settings) = claude_code::default_settings_path() else {
        eprintln!(
            "agentpet: user home directory is unavailable; cannot locate Claude Code settings"
        );
        return 1;
    };
    match claude_code::install_hook(&settings, hook_command()) {
        Ok(report) => {
            let backup = report.backup_path.map(|p| p.display().to_string());
            let record = HookInstallRecord {
                adapter_id: claude_code::ADAPTER_ID.to_owned(),
                config_path: report.config_path.display().to_string(),
                backup_path: backup.clone(),
                installed_at: chrono::Utc::now().to_rfc3339(),
                status: "installed".to_owned(),
            };
            if let Err(error) = core.record_hook_install(&record) {
                eprintln!("agentpet: hook updated but state was not recorded: {error}");
                return 1;
            }
            match backup {
                Some(path) => println!("{display_name} hook installed (backup at {path})"),
                None => println!("{display_name} hook installed"),
            }
            0
        }
        Err(error) => {
            eprintln!("agentpet: hook install failed: {error}");
            1
        }
    }
}

fn run_agents_remove(mut args: impl Iterator<Item = OsString>) -> i32 {
    let Some(adapter_id) = args.next().and_then(|v| v.into_string().ok()) else {
        eprintln!("agentpet: usage: agentpet agents remove <id>");
        return 2;
    };
    if args.next().is_some() {
        eprintln!("agentpet: usage: agentpet agents remove <id>");
        return 2;
    }
    let Some(adapter) = adapter_by_id(&adapter_id) else {
        eprintln!("agentpet: unknown agent id `{adapter_id}`");
        return 2;
    };
    let core = match open_local_core() {
        Ok(core) => core,
        Err(code) => return code,
    };
    remove_claude_hook(&core, adapter.display_name)
}

fn remove_claude_hook(core: &Core, display_name: &str) -> i32 {
    use agentpet_adapters::claude_code;
    let Some(settings) = claude_code::default_settings_path() else {
        eprintln!(
            "agentpet: user home directory is unavailable; cannot locate Claude Code settings"
        );
        return 1;
    };
    match claude_code::uninstall_hook(&settings, hook_command()) {
        Ok(true) => println!("{display_name} hook removed; unrelated settings preserved"),
        Ok(false) => println!("{display_name} hook was not installed; nothing changed"),
        Err(error) => {
            eprintln!("agentpet: hook removal failed: {error}");
            return 1;
        }
    }
    let record = HookInstallRecord {
        adapter_id: claude_code::ADAPTER_ID.to_owned(),
        config_path: settings.display().to_string(),
        backup_path: None,
        installed_at: chrono::Utc::now().to_rfc3339(),
        status: "removed".to_owned(),
    };
    if let Err(error) = core.record_hook_install(&record) {
        eprintln!("agentpet: hook state could not be recorded: {error}");
        return 1;
    }
    0
}

fn run_agents_status(mut args: impl Iterator<Item = OsString>) -> i32 {
    let requested = args.next().and_then(|v| v.into_string().ok());
    if args.next().is_some() {
        eprintln!("agentpet: usage: agentpet agents status [<id>]");
        return 2;
    }
    if requested
        .as_deref()
        .is_some_and(|id| adapter_by_id(id).is_none())
    {
        eprintln!("agentpet: unknown agent id");
        return 2;
    }
    let core = match open_local_core() {
        Ok(core) => core,
        Err(code) => return code,
    };
    let installs = match core.hook_installs() {
        Ok(installs) => installs,
        Err(error) => {
            eprintln!("agentpet: hook state is unavailable: {error}");
            return 1;
        }
    };
    for agent in ADAPTERS {
        if requested.as_deref().is_some_and(|id| id != agent.id) {
            continue;
        }
        let recorded = installs
            .iter()
            .find(|entry| entry.adapter_id == agent.id)
            .map(|entry| entry.status.as_str())
            .unwrap_or("never-installed");
        let live = claude_live_state(agent.id);
        println!(
            "{} ({}): record `{recorded}`, settings `{live}`",
            agent.display_name, agent.id
        );
    }
    0
}

fn claude_live_state(adapter_id: &str) -> &'static str {
    use agentpet_adapters::claude_code;
    if adapter_id != claude_code::ADAPTER_ID {
        return "n/a";
    }
    let Some(settings) = claude_code::default_settings_path() else {
        return "unknown";
    };
    match claude_code::is_installed(&settings, hook_command()) {
        Ok(true) => "present",
        Ok(false) => "absent",
        Err(_) => "unreadable",
    }
}

fn run_hook_command(mut args: impl Iterator<Item = OsString>) -> i32 {
    let mut adapter_id: Option<String> = None;
    while let Some(arg) = args.next() {
        if arg == OsStr::new("--adapter") {
            let Some(value) = args.next().and_then(|v| v.into_string().ok()) else {
                diagnostics::append_diagnostic_error("hook:args", &"missing adapter value");
                return 0;
            };
            adapter_id = Some(value);
        } else {
            diagnostics::append_diagnostic_error("hook:args", &"unknown hook argument");
            return 0;
        }
    }
    if adapter_id.as_deref() != Some(agentpet_adapters::claude_code::ADAPTER_ID) {
        diagnostics::append_diagnostic_error("hook:adapter", &"unknown or missing adapter id");
        return 0;
    }
    use std::io::Read;
    let mut input = Vec::new();
    if let Err(error) = std::io::stdin()
        .lock()
        .take(1_048_576)
        .read_to_end(&mut input)
    {
        diagnostics::append_diagnostic_error("hook:read_stdin", &error);
        return 0;
    }
    let event = match agentpet_adapters::claude_code::parse_hook_payload(&input) {
        Ok(event) => event,
        Err(error) => {
            diagnostics::append_diagnostic_error("hook:parse_payload", &error);
            return 0;
        }
    };
    let core = match Core::open_default() {
        Ok(core) => core,
        Err(error) => {
            diagnostics::append_diagnostic_error("hook:open_core", &error);
            return 0;
        }
    };
    let hook_event = agentpet_adapters::ipc::to_hook_event(&event);
    if let Err(error) = core.record_hook_event(&hook_event) {
        diagnostics::append_diagnostic_error("hook:record_event", &error);
        return 0;
    }
    0
}

#[allow(dead_code)]
fn outcome_label(outcome: &agentpet_core::HookEventOutcome) -> &'static str {
    match outcome {
        agentpet_core::HookEventOutcome::Accepted => "accepted",
        agentpet_core::HookEventOutcome::Duplicate => "duplicate",
        agentpet_core::HookEventOutcome::SessionStarted => "session_started",
        agentpet_core::HookEventOutcome::SessionCompleted { .. } => "session_completed",
        agentpet_core::HookEventOutcome::SessionFailed => "session_failed",
    }
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
    for agent in ADAPTERS {
        match find_on_path(agent.executable) {
            Some(path) => println!(
                "[OK] Agent CLI PATH entry found: {} ({})",
                agent.display_name,
                path.display()
            ),
            None => println!(
                "[WARN] Optional agent CLI not found: {}",
                agent.display_name
            ),
        }
    }
    match Core::open_default() {
        Ok(core) => match core.hook_installs() {
            Ok(records) => {
                for record in records.iter().filter(|r| r.status == "installed") {
                    println!(
                        "[OK] Native hook installed: {} ({})",
                        record.adapter_id, record.config_path
                    );
                }
                if !records.iter().any(|r| r.status == "installed") {
                    println!("[WARN] No native hooks installed");
                }
            }
            Err(error) => println!("[WARN] Hook state unavailable: {error}"),
        },
        Err(_) => println!("[WARN] Hook state unavailable: local database could not be opened"),
    }
    let recent_errors = diagnostics::read_recent_errors(5);
    if recent_errors.is_empty() {
        println!("[OK] Diagnostics log: no recent errors reported");
    } else {
        println!("[WARN] Diagnostics log has {} recent error(s):", recent_errors.len());
        for err in recent_errors {
            println!("       {err}");
        }
    }
    println!("[WARN] Cloud sync and credential-store checks are not implemented.");
    if failed {
        1
    } else {
        0
    }
}

fn run_verify_ledger(mut _args: impl Iterator<Item = OsString>) -> i32 {
    let core = match open_local_core() {
        Ok(core) => core,
        Err(code) => return code,
    };
    match core.verify_ledger() {
        Ok(result) => {
            if result.is_valid {
                println!(
                    "[OK] XP Ledger verified: {} entries in cryptographic chain (head: {})",
                    result.total_entries, result.head_hash
                );
                0
            } else {
                eprintln!(
                    "[FAIL] XP Ledger verification failed! Broken at entry {:?}: {}",
                    result.broken_at_index,
                    result.error_message.unwrap_or_default()
                );
                1
            }
        }
        Err(error) => {
            eprintln!("agentpet: ledger verification failed: {error}");
            1
        }
    }
}

fn run_statusline(mut args: impl Iterator<Item = OsString>) -> i32 {
    let mut chain_cmd: Option<String> = None;
    while let Some(arg) = args.next() {
        if arg == OsStr::new("--chain") {
            chain_cmd = args.next().and_then(|v| v.into_string().ok());
        }
    }
    use std::io::Read;
    let mut input = Vec::new();
    if let Err(error) = std::io::stdin()
        .lock()
        .take(256 * 1024)
        .read_to_end(&mut input)
    {
        diagnostics::append_diagnostic_error("statusline:stdin", &error);
    }
    if let Ok(data_dir) = default_data_directory() {
        if let Err(error) = std::fs::create_dir_all(&data_dir) {
            diagnostics::append_diagnostic_error("statusline:mkdir", &error);
        } else if let Err(error) = std::fs::write(data_dir.join("claude_statusline.json"), &input) {
            diagnostics::append_diagnostic_error("statusline:write", &error);
        }
    }
    if chain_cmd.is_none() {
        if let Some(settings_path) = agentpet_adapters::claude_code::default_settings_path() {
            let chain_path = agentpet_adapters::claude_code::default_chain_file_path(&settings_path);
            if let Some(record) = agentpet_adapters::claude_code::read_chain_record(&chain_path) {
                chain_cmd = record.original_command;
            }
        }
    }
    if let Some(cmd_str) = chain_cmd {
        run_chained_command(&cmd_str, &input);
    }
    0
}

fn run_chained_command(cmd_str: &str, input: &[u8]) {
    use std::time::Duration;
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(2);

    #[cfg(windows)]
    let mut child = match Command::new("cmd")
        .args(["/c", cmd_str])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            diagnostics::append_diagnostic_error("statusline:spawn", &e);
            return;
        }
    };

    #[cfg(not(windows))]
    let mut child = match Command::new("sh")
        .args(["-c", cmd_str])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            diagnostics::append_diagnostic_error("statusline:spawn", &e);
            return;
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(input) {
            diagnostics::append_diagnostic_error("statusline:pipe_stdin", &e);
        }
    }

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    diagnostics::append_diagnostic_error(
                        "statusline:child_exit",
                        &format!("exit code {:?}", status.code()),
                    );
                }
                break;
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    diagnostics::append_diagnostic_error(
                        "statusline:timeout",
                        &"chained command exceeded 2s timeout; killed",
                    );
                    let _ = child.kill();
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                diagnostics::append_diagnostic_error("statusline:wait", &e);
                break;
            }
        }
    }
}

fn run_pack_command(mut args: impl Iterator<Item = OsString>) -> i32 {
    let Some(sub) = args.next() else {
        eprintln!("agentpet: usage: agentpet pack validate <path>");
        return 2;
    };
    if sub == OsStr::new("validate") {
        let Some(path_arg) = args.next() else {
            eprintln!("agentpet: usage: agentpet pack validate <path>");
            return 2;
        };
        let pack_path = Path::new(&path_arg);
        match agentpet_pet_pack::PetPackManifest::load_and_validate(
            pack_path,
            agentpet_pet_pack::ValidationLimits::default(),
        ) {
            Ok(manifest) => {
                println!(
                    "[OK] Valid PetPack: {} ({}) by {}",
                    manifest.name, manifest.id, manifest.creator
                );
                println!("  Version: {}", manifest.version);
                println!("  License: {}", manifest.license);
                println!("  Assets declared: {}", manifest.assets.len());
                println!("  Animations declared: {}", manifest.animations.len());
                println!("  State mappings: {}", manifest.state_mappings.len());
                0
            }
            Err(error) => {
                eprintln!("[FAIL] PetPack validation failed: {error}");
                1
            }
        }
    } else {
        eprintln!("agentpet: unknown pack subcommand");
        2
    }
}

fn print_help() {
    println!("AgentPet local CLI\n\nUSAGE:\n  agentpet run -- <command> [args...]\n  agentpet status\n  agentpet pets list\n  agentpet agents list\n  agentpet agents install <id>\n  agentpet agents remove <id>\n  agentpet agents status [<id>]\n  agentpet hook --adapter claude_code < payload.json\n  agentpet verify-ledger\n  agentpet statusline [--chain \"<cmd>\"] < status.json\n  agentpet pack validate <path>\n  agentpet doctor\n");
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
