---
name: agent-android
description: Automates Android device interactions for app testing, UI navigation, screenshots, and data extraction. Use when the user needs to control Android devices, inspect app UI, test Android apps, or extract device info.
---

# Android Automation with agent-android

## Quick start

```bash
agent-android connect 192.168.1.5    # Connect to device (optional if USB)
agent-android snapshot               # Get elements with refs
agent-android tap :e1                # Tap element by ref
agent-android input "hello"            # Type text
agent-android back                   # Press back button
```

## Core workflow

1. Connect: `agent-android connect <host>` or ensure USB device is listed in `agent-android devices`
2. Snapshot: `agent-android snapshot` (returns elements with refs like `e1`, `e2`)
3. Interact using refs from the snapshot
4. Re-snapshot after navigation or significant UI changes

## UI Synchronization

Whenever an action causes a UI change (e.g., navigating to a new screen, opening a dialog), you **MUST** run the `snapshot` command again to update the element references and page state before attempting further interactions. Using stale references will lead to errors.

## Token Usage Optimization

To conserve tokens and improve performance:
- **Prefer `snapshot` over `screenshot`**: The `snapshot` command returns a structured text representation of the UI, which consumes significantly fewer tokens than processing raw images. Only use `screenshot` when visual verification is strictly necessary (e.g., checking layout rendering or image content).
- **Batch interactions**: When possible, plan multiple interactions based on a single snapshot rather than taking a new snapshot after every single tap, unless the UI state changes drastically.

## Commands

### Connection & Device
```bash
agent-android devices                    # List connected devices
agent-android connect <host>             # Connect to TCP/IP device
agent-android push <local> <remote>      # Push file to device
agent-android pull <remote> [local]      # Pull file from device
agent-android disconnect                 # Disconnect from all or specific device
agent-android info                       # Get device info
agent-android shell <cmd>                # Run shell command
agent-android exec <args>                # Run raw adb command (e.g. exec reboot)
```

### Snapshot (UI analysis)
```bash
agent-android snapshot                   # Get UI hierarchy (compact mode by default)
agent-android snapshot -f                # Full output (include resource-ids and structural elements)
agent-android snapshot -d 3              # Limit recursion depth to 3
agent-android snapshot --selector "Login"        # Filter by text content (subtree)
agent-android snapshot --selector "role=list"    # Filter by role (e.g., list, button, textbox)
agent-android snapshot --selector "id=header"    # Filter by resource-id
```

### Interactions (use :refs from snapshot)
```bash
agent-android tap :e1                    # Tap element
agent-android tap 500 1000               # Tap coordinates
agent-android input "hello"              # Type text
agent-android key HOME                   # Press Home button
agent-android back                       # Press Back button
agent-android scroll down                # Scroll down
agent-android scroll left                # Scroll left
agent-android scroll 100 500 100 100     # Swipe manually
agent-android wait 2000                  # Wait milliseconds
```

### Query & Inspection
```bash
agent-android find text "Login"          # Find element info (no click)
agent-android find "text=Login"          # Same as above (key=value syntax)
agent-android find text "Login" click    # Find element by text and click
agent-android find "text=Login" click    # Find element by text and click
agent-android find :e1 click             # Find by ref and click
agent-android find role button tap       # Find first button and tap
agent-android is visible text "Submit"   # Check visibility
agent-android is visible :e1             # Check visibility by ref
agent-android get text resource-id com.app:id/title # Get text content
agent-android get bounds text "Login"    # Get element bounds
agent-android get text :e1               # Get text by ref
agent-android check text "I agree"       # Check checkbox
agent-android uncheck :e1                # Uncheck element
agent-android select "Option A"          # Select option (click text)
agent-android select :e1 "Option A"      # Click dropdown :e1 then click "Option A"
```

### App Management
```bash
agent-android install app.apk            # Install APK
agent-android uninstall com.example.app  # Uninstall package
agent-android list-packages              # List installed packages
agent-android start com.example.app      # Start application
agent-android stop com..example.app      # Force stop package
```

### Media
```bash
agent-android screenshot                 # Screenshot to screenshot.png
agent-android screenshot out.png         # Save to specific file
agent-android record video.mp4 10        # Record 10s video
```

## Example: Login Flow

```bash
agent-android start com.example.app
agent-android snapshot
# Output shows: textbox "Email" [ref=e1], textbox "Password" [ref=e2], button "Login" [ref=e3]

agent-android input :e1 "user@example.com"
agent-android input :e2 "secret"
agent-android tap :e3
agent-android wait 2000
agent-android snapshot   # Check result
```

## JSON output (for parsing)

Add `--json` for machine-readable output:
```bash
agent-android snapshot --json
agent-android devices --json
```

## Multi-device support

Target specific device by serial:
```bash
agent-android -s serial123 tap :e1
agent-android -s 192.168.1.5:5555 snapshot
```
