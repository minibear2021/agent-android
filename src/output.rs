use serde::Serialize;

#[derive(Serialize)]
pub struct Response {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Response {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            error: None,
            data: Some(data),
        }
    }

    pub fn err(msg: String) -> Self {
        Self {
            success: false,
            error: Some(msg),
            data: None,
        }
    }
}

pub fn print_response(resp: &Response, json_mode: bool) {
    if json_mode {
        println!("{}", serde_json::to_string(resp).unwrap_or_default());
        return;
    }

    if !resp.success {
        eprintln!(
            "\x1b[31m✗\x1b[0m {}",
            resp.error.as_deref().unwrap_or("Unknown error")
        );
        return;
    }

    if let Some(data) = &resp.data {
        // Devices list
        if let Some(devices) = data.get("devices").and_then(|v| v.as_array()) {
            if devices.is_empty() {
                println!("No devices connected");
            } else {
                println!("Connected devices:");
                for dev in devices {
                    let serial = dev.get("serial").and_then(|s| s.as_str()).unwrap_or("?");
                    let state = dev.get("state").and_then(|s| s.as_str()).unwrap_or("?");
                    let model = dev.get("model").and_then(|s| s.as_str()).unwrap_or("");
                    println!("{} ({}) {}", serial, state, model);
                }
            }
            return;
        }

        // Standard output (stdout from adb)
        if let Some(stdout) = data.get("stdout").and_then(|v| v.as_str()) {
            if !stdout.is_empty() {
                println!("{}", stdout);
            } else {
                println!("\x1b[32m✓\x1b[0m Done");
            }
            return;
        }

        // Generic message
        if let Some(msg) = data.get("message").and_then(|v| v.as_str()) {
            println!("\x1b[32m✓\x1b[0m {}", msg);
            return;
        }

        // Info
        if let Some(info) = data.get("info").and_then(|v| v.as_object()) {
             for (k, v) in info {
                 println!("{}: {}", k, v.as_str().unwrap_or(""));
             }
             return;
        }

        // Snapshot
        if let Some(snapshot) = data.get("snapshot").and_then(|v| v.as_str()) {
             println!("{}", snapshot);
             return;
        }
        
        // Result (is/get) - Handled by message, but fallback just in case
        if let Some(result) = data.get("result") {
             if data.get("message").is_none() {
                 println!("{}", result);
                 return;
             }
        }
        
        // Default success
        println!("\x1b[32m✓\x1b[0m Done");
    } else {
        println!("\x1b[32m✓\x1b[0m Done");
    }
}

pub fn print_help() {
    println!(
        r#"
agent-android - Android ADB CLI for AI agents

Usage: agent-android <command> [args] [options]

Connection & Device:
  devices                    List connected devices
  connect <host>             Connect to TCP/IP device
  disconnect                 Disconnect from all or specific device
  info                       Get device info
  push <local> <remote>      Push file to device
  pull <remote> [local]      Pull file from device
  shell <cmd>                Run shell command
  exec <args...>             Execute raw adb command (e.g. exec reboot)

Interaction:
  tap <x> <y>                Tap at coordinates
  tap :ref                   Tap at element reference (from snapshot)
  text <string>              Input text
  key <code|name>            Press key (home, back, enter, etc.)
  home                       Press Home button
  back                       Press Back button
  scroll [dir]               Scroll (up/down/left/right, default: down)
  scroll <x1> <y1> <x2> <y2> Swipe from (x1,y1) to (x2,y2)

Query & Inspection:
  snapshot                   Dump UI hierarchy
    -f, --full               Full output (include resource-ids and structural elements)
    -d, --depth <n>          Limit recursion depth
    --selector <sel>         Focus on specific element (text, id, role)
  
  find <locator> <value> [action] [text]
    Locators: text, role, resource-id, content-desc, etc.
    Actions: click, type, info (default: info)
    Example: find text "Login" click
    Example: find "text=Login" click
  
  is <state> <locator> <value>
    States: visible, enabled, checked, selected
    Example: is visible text "Submit"
  
  get <prop> <locator> <value>
    Props: text, content-desc, id, class, bounds, checked, enabled
    Example: get text id "user_name"

App Management:
  install <path>             Install APK
  uninstall <pkg>            Uninstall package
  list-packages              List installed packages
  start <pkg>             Start package
  stop <pkg>                 Force stop package

Media:
  screenshot [path]          Take screenshot
  record <path> [sec]        Record screen video

Options:
  --json                     Output as JSON
  --serial, -s <serial>      Target specific device
"#
    );
}
