# agent-android

A fast, pure Rust CLI tool for controlling Android devices via ADB, designed for AI agents.

## Features

- **Device Management**: List devices, connect/disconnect via TCP/IP.
- **Input Control**: Send key events, taps, and text input.
- **UI Snapshot & Refs**: Get accessible UI tree and interact via references (`click @e1`).
- **App Management**: Install, uninstall, list, start, and stop apps.
- **Screen Operations**: Capture screenshots and record screen.
- **Agent-Friendly**: JSON output support for easy parsing by AI agents.
- **Cross-Platform**: Works on Windows, Linux, and macOS (requires `adb` in PATH).

## Installation

### Download Binary

Download the latest release for your platform from the [GitHub Releases](https://github.com/minibear2021/agent-android/releases) page.

### Build from Source

```bash
cd agent-android
cargo build --release
# Binary will be in target/release/agent-android
```

## Usage

### Basic Commands

```bash
# List devices
agent-android devices
agent-android devices --json

# Connect to a device
agent-android connect 192.168.1.5:5555

# Disconnect
agent-android disconnect
agent-android disconnect 192.168.1.5:5555

# Get device info
agent-android info

# Run shell command
agent-android shell ls -la

# Run raw adb command
agent-android exec reboot
```

### Snapshot & References (Agent Mode)

Get a UI snapshot to understand the screen content. This generates short references (e.g., `e1`, `e2`) that can be used in subsequent commands.

```bash
# Get UI tree (default is compact mode)
agent-android snapshot

# Get full UI tree (including resource-ids and structural elements)
agent-android snapshot -f

# Get interactive elements only (buttons, inputs, etc.)
agent-android snapshot -i
# Output:
# - button "Login" [ref=e1]
# - textbox "Username" [ref=e2]

# Interact using references
agent-android click :e1
agent-android type :e2 "myuser"
```

### Input Control

```bash
# Press Home button
agent-android key HOME

# Tap at coordinates
agent-android tap 500 1000

# Tap element by ref (requires snapshot first)
agent-android tap :e1

# Type text
agent-android input "Hello World"
agent-android input :e2 "Hello World"  # Taps e2 first, then types
```

### Query & Inspection

```bash
# Find element and act on it
# usage: find <locator> <value> [action] [text]
# usage: find <ref> [action] [text]
# locators: text, role, resource-id, content-desc
# actions: click (default), type, info
agent-android find text "Login" click
agent-android find :e1 click
agent-android find role button tap
agent-android find text "Username" type "myuser"

# Check state
# usage: is <state> <locator> <value>
# usage: is <state> <ref>
# states: visible, enabled, checked, selected
agent-android is visible text "Submit"
agent-android is visible :e1

# Get property
# usage: get <prop> <locator> <value>
# usage: get <prop> <ref>
# props: text, content-desc, resource-id, class, bounds, checked, enabled
agent-android get text resource-id com.example:id/title
agent-android get bounds text "Submit"
agent-android get text :e1

# Check/Uncheck element
# usage: check <locator> <value> OR check <ref>
# usage: uncheck <locator> <value> OR uncheck <ref>
agent-android check text "Agree to terms"
agent-android uncheck :e1

# Select element (click by text)
# usage: select <selector> <value> OR select <value>
agent-android select "Option A"
agent-android select :e1 "Option A"
```

### App Management

```bash
# List packages
agent-android list-packages

# Install APK
agent-android install ./app.apk

# Start an app (package name or activity)
agent-android start com.example.app
agent-android start com.example.app/.MainActivity

# Stop an app
agent-android stop com.example.app
```

### File Operations

```bash
# Push file to device
agent-android push ./local_file.txt /sdcard/remote_file.txt

# Pull file from device
agent-android pull /sdcard/remote_file.txt ./local_file.txt
```

### Screen Operations

```bash
# Take a screenshot
agent-android screenshot ./capture.png

# Record screen (5 seconds default)
agent-android record ./video.mp4 10
```

### Global Options

- `--json`: Output results in JSON format.
- `--serial <serial>` or `-s <serial>`: Target a specific device serial.
- `--debug`: Enable debug output.

## Architecture

`agent-android` acts as a thin wrapper around the standard `adb` binary. It parses high-level commands and translates them into `adb` executions, providing structured output.
For snapshots, it uses `uiautomator` to dump the UI hierarchy, parses it, and maintains a temporary state file to resolve element references.

```
User/Agent -> agent-android (Rust) -> adb (Binary) -> Android Device
                                   -> State File (Temp)
```

## JSON Output Format

All commands support `--json` for structured output:

```json
{
  "success": true,
  "data": {
    "message": "Screenshot saved",
    "path": "./capture.png"
  }
}
```

Snapshot output:

```json
{
  "success": true,
  "data": {
    "tree": "- button \"Login\" [ref=e1]\n- textbox \"User\" [ref=e2]",
    "refs": {
      "e1": { "role": "button", "name": "Login" },
      "e2": { "role": "textbox", "name": "User" }
    }
  }
}
```
