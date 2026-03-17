mod app;
mod event;
mod protocol;
mod shell;
mod theme;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::sync::Arc;

use shell::ShellManager;

#[derive(Parser, Debug)]
#[command(name = "wta", about = "Windows Terminal Agent — ACP TUI client / MCP tool server")]
struct Cli {
    /// Initial prompt to send to the agent (ACP mode only)
    #[arg(value_name = "PROMPT")]
    prompt: Option<String>,

    /// Agent CLI command (e.g. "copilot --acp --stdio")
    #[arg(long, default_value = "copilot --acp --stdio")]
    agent: String,

    /// Run as MCP server (headless, no TUI)
    #[arg(long, group = "mode")]
    mcp: bool,

    /// Run as ACP client with TUI (default)
    #[arg(long, group = "mode")]
    acp: bool,

    /// Test pipe connection to Windows Terminal (connect, authenticate, list_windows)
    #[arg(long, group = "mode")]
    test_pipe: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.test_pipe {
        return run_test_pipe().await;
    }

    // Debug channel for TUI debug panel (pipe traffic viewer)
    let (debug_tx, debug_rx) = tokio::sync::mpsc::unbounded_channel::<app::DebugMessage>();

    // Try to connect to the Windows Terminal pipe (non-fatal if unavailable).
    let mut shell_mgr = ShellManager::new();
    let wt_connected = match shell::wt_channel::PipeChannel::connect().await {
        Ok(channel) => {
            eprintln!("[wta] Connected to Windows Terminal pipe");
            let channel = channel.with_debug_sender(debug_tx.clone());
            shell_mgr = shell_mgr.with_wt_channel(Arc::new(channel));
            true
        }
        Err(e) => {
            eprintln!("[wta] No WT pipe (local-only mode): {}", e);
            false
        }
    };
    let shell_mgr = Arc::new(shell_mgr);

    // Try to discover our own pane identity by PID matching
    let pane_identity = if wt_connected {
        discover_pane_identity(&shell_mgr).await
    } else {
        None
    };

    if cli.mcp {
        // Headless MCP server mode — no TUI
        protocol::mcp::server::run_mcp_server(shell_mgr).await
    } else {
        // ACP TUI client mode (default)
        run_acp_tui_mode(cli, shell_mgr, wt_connected, debug_rx, pane_identity).await
    }
}

/// Discover our own pane identity by matching our PID against WT's pane list.
async fn discover_pane_identity(shell_mgr: &ShellManager) -> Option<(String, String, String)> {
    let our_pid = std::process::id();

    // List all windows, then tabs, then panes to find our PID
    let windows = shell_mgr.wt_list_windows().await.ok()?;
    let windows_arr = windows.get("windows")?.as_array()?;

    for win in windows_arr {
        let window_id = win.get("window_id")?.as_str()?;
        let tabs = shell_mgr.wt_list_tabs(window_id).await.ok()?;
        let tabs_arr = tabs.get("tabs")?.as_array()?;

        for tab in tabs_arr {
            // tab_id may be a string or number in the JSON
            let tab_id_str = match tab.get("tab_id") {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(serde_json::Value::Number(n)) => n.to_string(),
                _ => continue,
            };
            let panes = shell_mgr.wt_list_panes(&tab_id_str).await.ok()?;
            let panes_arr = panes.get("panes")?.as_array()?;

            for pane in panes_arr {
                if let Some(pid) = pane.get("pid").and_then(|v| v.as_u64()) {
                    if pid == our_pid as u64 {
                        let pane_id = match pane.get("pane_id") {
                            Some(serde_json::Value::String(s)) => s.clone(),
                            Some(serde_json::Value::Number(n)) => n.to_string(),
                            _ => continue,
                        };
                        return Some((pane_id, tab_id_str.clone(), window_id.to_string()));
                    }
                }
            }
        }
    }
    None
}

async fn run_acp_tui_mode(
    cli: Cli,
    shell_mgr: Arc<ShellManager>,
    wt_connected: bool,
    debug_rx: tokio::sync::mpsc::UnboundedReceiver<app::DebugMessage>,
    pane_identity: Option<(String, String, String)>,
) -> Result<()> {
    // Init terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_acp_app(&mut terminal, cli, shell_mgr, wt_connected, debug_rx, pane_identity).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e:?}");
        std::process::exit(1);
    }
    Ok(())
}

async fn run_test_pipe() -> Result<()> {
    use shell::wt_channel::{PipeChannel, WtChannel};

    println!("Connecting to Windows Terminal pipe...");
    let channel: PipeChannel = PipeChannel::connect().await?;
    println!("Connected and authenticated!\n");

    let result: serde_json::Value = channel
        .request("list_windows", serde_json::json!({}))
        .await?;
    println!("list_windows:");
    println!("{}\n", serde_json::to_string_pretty(&result)?);

    let result: serde_json::Value = channel
        .request("get_capabilities", serde_json::json!({}))
        .await?;
    println!("get_capabilities:");
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

/// Generate an MCP config JSON file that points to `wta --mcp`,
/// passing through WT_PIPE_NAME and WT_MCP_TOKEN so the MCP server
/// can connect to the same WT pipe. Returns the path to the config file.
fn write_wta_mcp_config() -> Result<std::path::PathBuf> {
    let wta_exe = std::env::current_exe()?.to_string_lossy().replace('\\', "/");
    let pipe_name = std::env::var("WT_PIPE_NAME").unwrap_or_default().replace('\\', "/");
    let token = std::env::var("WT_MCP_TOKEN").unwrap_or_default();

    let config = serde_json::json!({
        "mcpServers": {
            "windows-terminal": {
                "type": "stdio",
                "command": wta_exe,
                "args": ["--mcp"],
                "env": {
                    "WT_PIPE_NAME": pipe_name,
                    "WT_MCP_TOKEN": token
                }
            }
        }
    });

    let config_path = std::env::temp_dir().join("wta-mcp-config.json");
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(config_path)
}

async fn run_acp_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    cli: Cli,
    shell_mgr: Arc<ShellManager>,
    wt_connected: bool,
    mut debug_rx: tokio::sync::mpsc::UnboundedReceiver<app::DebugMessage>,
    pane_identity: Option<(String, String, String)>,
) -> Result<()> {
    // If WT pipe is connected, generate MCP config and inject into agent command
    // so the agent gets WT tools (list_windows, read_pane_output, etc.)
    let agent_cmd = if wt_connected {
        match write_wta_mcp_config() {
            Ok(config_path) => {
                let config_str = config_path.to_string_lossy();
                // Copilot uses --additional-mcp-config, Claude uses --mcp-config
                let base = &cli.agent;
                if base.contains("copilot") {
                    format!("{} --additional-mcp-config @{}", base, config_str)
                } else if base.contains("claude") {
                    format!("{} --mcp-config {}", base, config_str)
                } else {
                    // Generic: try additional-mcp-config
                    format!("{} --additional-mcp-config @{}", base, config_str)
                }
            }
            Err(e) => {
                eprintln!("[wta] Failed to write MCP config: {}", e);
                cli.agent.clone()
            }
        }
    } else {
        cli.agent.clone()
    };

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
            let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();

            // Start crossterm event reader
            let evt_tx = event_tx.clone();
            tokio::task::spawn_local(event::read_crossterm_events(evt_tx));

            // Forward pipe debug messages to the app event loop
            let dbg_event_tx = event_tx.clone();
            tokio::task::spawn_local(async move {
                while let Some(msg) = debug_rx.recv().await {
                    let _ = dbg_event_tx.send(app::AppEvent::DebugPipeMessage(msg));
                }
            });

            // Start ACP client
            let acp_event_tx = event_tx.clone();
            tokio::task::spawn_local(protocol::acp::client::run_acp_client(
                agent_cmd,
                cli.prompt.clone(),
                acp_event_tx,
                prompt_rx,
                shell_mgr,
            ));

            // Run main event loop
            let mut app_state = app::App::new(prompt_tx, wt_connected);
            if let Some((pane_id, tab_id, window_id)) = pane_identity {
                app_state.pane_id = Some(pane_id);
                app_state.tab_id = Some(tab_id);
                app_state.window_id = Some(window_id);
            }
            app_state.run(terminal, event_rx).await
        })
        .await
}
