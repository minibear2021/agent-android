use serde_json::{json, Value};
use crate::flags::Flags;
use crate::adb;
use crate::snapshot;
use crate::state;
use crate::query;
use std::thread;
use std::time::Duration;

pub fn execute_command(args: &[String], flags: &Flags) -> Result<Value, String> {
    if args.is_empty() {
        return Err("No command specified".to_string());
    }

    let cmd = args[0].as_str();
    let rest: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
    let serial = flags.serial.as_deref();

    match cmd {
        "devices" => adb::list_devices(),
        
        "connect" => {
            if let Some(host) = rest.get(0) {
                adb::connect(host, serial)
            } else {
                Err("Usage: connect <host>".to_string())
            }
        }
        
        "disconnect" => {
            adb::disconnect(rest.get(0).copied(), serial)
        }
        
        "shell" => {
            if rest.is_empty() {
                Err("Usage: shell <command>".to_string())
            } else {
                adb::shell(&rest.join(" "), serial)
            }
        }
        
        "info" => adb::get_device_info(serial),
        
        "key" | "press" => {
            if let Some(code) = rest.get(0) {
                adb::input_key(code, serial)
            } else {
                Err("Usage: key <keycode>".to_string())
            }
        }
        
        "back" => adb::input_key("BACK", serial),
        
        "home" => adb::input_key("HOME", serial),
        
        "tap" | "click" => {
            if let Some(raw_arg) = rest.get(0) {
                let arg = raw_arg.trim().trim_matches('\'').trim_matches('"');
                
                // Check if it's a ref
                if arg.starts_with(":") || arg.starts_with("ref=") || (arg.starts_with("e") && arg[1..].chars().all(char::is_numeric)) {
                     let ref_data = state::resolve_ref(arg, serial)?;
                     adb::input_tap(ref_data.center[0], ref_data.center[1], serial)
                } else if let (Some(y_arg), Ok(x)) = (rest.get(1), arg.parse::<i32>()) {
                    if let Ok(y) = y_arg.parse::<i32>() {
                        adb::input_tap(x, y, serial)
                    } else {
                        Err("Invalid coordinates".to_string())
                    }
                } else {
                    Err(format!("Usage: tap <x> <y> OR tap :ref. Invalid argument: '{}'", raw_arg))
                }
            } else {
                Err("Usage: tap <x> <y> OR tap :ref".to_string())
            }
        }
        
        "input" | "type" => {
             // Support: input "hello" OR input :ref "hello" (tap then type)
             if rest.is_empty() {
                 return Err("Usage: input <string> OR input :ref <string>".to_string());
             }
             
             let first_raw = rest[0];
             let first = first_raw.trim().trim_matches('\'').trim_matches('"');
             
             if (first.starts_with(":") || first.starts_with("ref=") || (first.starts_with("e") && first[1..].chars().all(char::is_numeric))) && rest.len() >= 2 {
                 // Tap ref then type
                 let ref_data = state::resolve_ref(first, serial)?;
                 adb::input_tap(ref_data.center[0], ref_data.center[1], serial)?;
                 // Need small delay? ADB usually queues.
                 let text_to_type = rest[1..].join(" ");
                 adb::input_text(&text_to_type, serial)
             } else {
                 // Just type
                 adb::input_text(&rest.join(" "), serial)
             }
        }
        
        "scroll" => {
            // Usage: scroll [direction] [amount] OR scroll <x1> <y1> <x2> <y2>
            // directions: up, down, left, right. default down.
            // amount: pixels (default 500) or duration (default 300ms)
            
            // Simplified scroll logic for common case: scroll down (content moves up)
            // Center of screen usually good start
            // We need screen size for relative scrolling, but let's assume standard 1080x1920 for defaults or safe area
            // Safe center area: 500, 1000.
            
            let direction = rest.get(0).copied().unwrap_or("down");
            let _amount = rest.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(500); // distance
            
            // Parse manual coords
             if rest.len() >= 4 {
                 if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) = (
                     rest[0].parse::<i32>(), rest[1].parse::<i32>(), rest[2].parse::<i32>(), rest[3].parse::<i32>()
                 ) {
                     return adb::input_swipe(x1, y1, x2, y2, 300, serial);
                 }
             }

            // Directional scroll (swipe)
            // Scroll DOWN means content moves UP (finger moves UP) -> swipe y1 > y2
            // Scroll UP means content moves DOWN (finger moves DOWN) -> swipe y1 < y2
            let (x1, y1, x2, y2) = match direction {
                "down" => (500, 1500, 500, 500), // Swipe up to scroll down
                "up" => (500, 500, 500, 1500),   // Swipe down to scroll up
                "right" => (200, 1000, 800, 1000), // Swipe right (move content right? usually "scroll right" means see right content -> swipe left)
                                                   // Wait, standard UI: "scroll right" -> move view to right -> content moves left -> swipe left (x1 > x2)
                "left" => (800, 1000, 200, 1000),
                _ => return Err("Invalid direction. Use up, down, left, right or coords x1 y1 x2 y2".to_string())
            };
            
            adb::input_swipe(x1, y1, x2, y2, 500, serial)
        }
        
        "wait" => {
            if let Some(arg) = rest.get(0) {
                if let Ok(ms) = arg.parse::<u64>() {
                    thread::sleep(Duration::from_millis(ms));
                    Ok(json!({ "message": "Waited", "duration": ms }))
                } else {
                    Err("Usage: wait <ms>".to_string())
                }
            } else {
                 Err("Usage: wait <ms>".to_string())
            }
        }

        "install" => {
            if let Some(path) = rest.get(0) {
                adb::install(path, serial)
            } else {
                Err("Usage: install <path>".to_string())
            }
        }
        
        "uninstall" => {
            if let Some(pkg) = rest.get(0) {
                adb::uninstall(pkg, serial)
            } else {
                Err("Usage: uninstall <package>".to_string())
            }
        }
        
        "list-packages" => adb::list_packages(serial),
        
        "start" => {
            if let Some(target) = rest.get(0) {
                adb::start_activity(target, serial)
            } else {
                Err("Usage: start <package/activity>".to_string())
            }
        }
        
        "stop" => {
            if let Some(pkg) = rest.get(0) {
                adb::stop_package(pkg, serial)
            } else {
                Err("Usage: stop <package>".to_string())
            }
        }
        
        "screenshot" => {
            let path = rest.get(0).copied();
            adb::screenshot(path, serial)
        }
        
        "record" => {
            let path = rest.get(0).copied().ok_or("Usage: record <path> [duration]")?;
            let duration = rest.get(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(5);
            adb::record(path, duration, serial)
        }
        
        "push" => {
            if let (Some(local), Some(remote)) = (rest.get(0), rest.get(1)) {
                adb::push(local, remote, serial)
            } else {
                Err("Usage: push <local> <remote>".to_string())
            }
        }
        
        "pull" => {
            if let Some(remote) = rest.get(0) {
                let local = rest.get(1).copied();
                adb::pull(remote, local, serial)
            } else {
                Err("Usage: pull <remote> [local]".to_string())
            }
        }
        
        "snapshot" => {
             let interactive = args.iter().any(|a| a == "-i" || a == "--interactive");
             let compact = args.iter().any(|a| a == "-c" || a == "--compact");
             
             // Parse depth
             let mut max_depth = None;
             if let Some(idx) = args.iter().position(|a| a == "-d" || a == "--depth") {
                 if let Some(val) = args.get(idx + 1) {
                     if let Ok(d) = val.parse::<usize>() {
                         max_depth = Some(d);
                     }
                 }
             }
             
             // Parse selector
             let mut selector = None;
             if let Some(idx) = args.iter().position(|a| a == "-s" || a == "--selector") {
                 if let Some(val) = args.get(idx + 1) {
                     selector = Some(val.as_str());
                 }
             }
             
             snapshot::get_snapshot(serial, interactive, compact, max_depth, selector)
        }

        "find" => query::handle_find(&rest, serial),
        
        "is" => query::handle_is(&rest, serial),
        
        "get" => query::handle_get(&rest, serial),

        "check" => query::handle_check(&rest, serial, true),

        "uncheck" => query::handle_check(&rest, serial, false),

        "select" => query::handle_select(&rest, serial),
        
        // "exec" command forwards arguments to adb.
        // Example: agent-android exec shell ls -la
        "exec" => {
             // Pass all rest args to adb
             // args[0] is "exec", so use rest
             let output = adb::run_adb(&rest, serial).map_err(|e| e)?;
             Ok(json!({ "stdout": output }))
        }

        _ => Err(format!("Unknown command: {}", cmd)),
    }
}
