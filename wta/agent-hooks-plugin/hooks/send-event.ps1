# send-event.ps1 — Forward Copilot CLI hook events to WTA via wtcli
param([string]$EventType = "agent.hook")

# Skip if not running inside Windows Terminal
if (-not $env:WT_COM_CLSID) { exit 0 }

# Locate wtcli.exe. Order:
#   1. PATH (works if the package registers a wtcli AppExecutionAlias).
#   2. $env:WTCLI_PATH override (escape hatch for dev builds / debugging).
#   3. The Windows Terminal package InstallLocation (where the build drops it).
$wtcliPath = (Get-Command wtcli -ErrorAction SilentlyContinue).Source
if (-not $wtcliPath -and $env:WTCLI_PATH -and (Test-Path $env:WTCLI_PATH)) {
    $wtcliPath = $env:WTCLI_PATH
}
if (-not $wtcliPath) {
    try {
        $pkgs = Get-AppxPackage -Name "*Terminal*" -ErrorAction SilentlyContinue
        foreach ($pkg in $pkgs) {
            $candidate = Join-Path $pkg.InstallLocation "wtcli.exe"
            if (Test-Path $candidate) { $wtcliPath = $candidate; break }
        }
    } catch { }
}
if (-not $wtcliPath) { exit 0 }

# Read hook JSON from stdin (may be empty for events that don't carry a
# payload, e.g. some CLIs' AfterTool / SessionEnd. We still want those to
# reach WTA so the state can transition out of Working/Working back to Idle.)
$hookData = [Console]::In.ReadToEnd()
if (-not $hookData) { $hookData = "" }

# Wrap payload and send via ProcessStartInfo to avoid PowerShell argument mangling
try {
    # ConvertFrom-Json on empty/whitespace input throws; treat as no payload.
    $parsed = $null
    if ($hookData.Trim()) {
        try { $parsed = $hookData | ConvertFrom-Json } catch { $parsed = $null }
    }

    # Extract agent_session_id from stdin JSON (Claude/Gemini), env (Copilot), or empty.
    $agentSessionId = ""
    if ($parsed -and ($parsed.PSObject.Properties.Name -contains "session_id")) {
        $agentSessionId = [string]$parsed.session_id
    } elseif ($env:COPILOT_SESSION_ID) {
        $agentSessionId = $env:COPILOT_SESSION_ID
    } elseif ($env:CLAUDE_SESSION_ID) {
        $agentSessionId = $env:CLAUDE_SESSION_ID
    } elseif ($env:GEMINI_SESSION_ID) {
        $agentSessionId = $env:GEMINI_SESSION_ID
    }

    # Detect CLI source: prefer WTA_CLI_SOURCE (set by bash hooks); else use the
    # CLI-specific session-id env var (most reliable: only that CLI sets it);
    # only fall back to CLAUDE_PLUGIN_ROOT if no session-id was found, since
    # Copilot CLI also sets CLAUDE_PLUGIN_ROOT (its plugin format borrows from
    # Claude), which would otherwise mis-tag Copilot sessions as Claude.
    $cliSource = $env:WTA_CLI_SOURCE
    if (-not $cliSource) {
        if     ($env:COPILOT_SESSION_ID) { $cliSource = "copilot" }
        elseif ($env:GEMINI_SESSION_ID)  { $cliSource = "gemini" }
        elseif ($env:CLAUDE_SESSION_ID)  { $cliSource = "claude" }
        elseif ($env:GEMINI_CLI)         { $cliSource = "gemini" }
        elseif ($env:COPILOT_CLI)        { $cliSource = "copilot" }
        elseif ($env:CLAUDE_PLUGIN_ROOT) { $cliSource = "claude" }
        else { $cliSource = "copilot" }
    }

    $wrapper = @{
        cli_source       = $cliSource
        agent_session_id = $agentSessionId
        payload          = $parsed
    }

    $payload = $wrapper | ConvertTo-Json -Compress -Depth 5

    # CommandLineToArgvW-correct escape for a quoted argument:
    #   * Every backslash run that precedes a `"` (or end of string) is doubled.
    #   * Every `"` is preceded by a single extra backslash.
    # This is required so messages containing Windows paths (e.g. permission
    # prompts: 'Get-Acl -Path "C:\Windows\..."') don't have their JSON truncated
    # by the child process's argv parser.
    $sb = New-Object System.Text.StringBuilder
    $bsRun = 0
    foreach ($ch in $payload.ToCharArray()) {
        if ($ch -eq '\') {
            $bsRun++
        } elseif ($ch -eq '"') {
            [void]$sb.Append([string]'\' * ($bsRun * 2 + 1))
            [void]$sb.Append('"')
            $bsRun = 0
        } else {
            if ($bsRun -gt 0) { [void]$sb.Append([string]'\' * $bsRun); $bsRun = 0 }
            [void]$sb.Append($ch)
        }
    }
    if ($bsRun -gt 0) { [void]$sb.Append([string]'\' * ($bsRun * 2)) }
    $escaped = $sb.ToString()

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $wtcliPath
    $psi.Arguments = "send-event -e $EventType `"$escaped`""
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardError = $true
    $proc = [System.Diagnostics.Process]::Start($psi)
    $proc.WaitForExit(5000)
} catch {
    # Silently ignore errors — hooks must not block the agent.
}
