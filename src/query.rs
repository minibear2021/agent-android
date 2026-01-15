use roxmltree::{Document, Node};
use crate::snapshot::{dump_ui, map_class_to_role, parse_bounds, get_xml_path};
use crate::adb;
use crate::state;
use serde_json::{json, Value};
use std::fs;

pub fn find_element(locator: &str, value: &str, serial: Option<&str>) -> Result<Option<Value>, String> {
    // Check if cache exists
    let path = get_xml_path(serial);
    let xml = if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("Failed to read cached dump: {}", e))?
    } else {
        dump_ui(serial)?
    };

    let doc = Document::parse(&xml).map_err(|e| format!("Invalid XML: {}", e))?;
    
    let node = if state::is_ref_format(locator) {
        let target_id = state::parse_ref_id(locator)?;
        let mut counter = 0;
        state::find_node_by_counter(doc.root_element(), &mut counter, target_id)
    } else {
        find_node(&doc, locator, value)
    };
    
    if let Some(n) = node {
        let bounds = parse_bounds(n.attribute("bounds").unwrap_or("[0,0][0,0]"));
        let center = [(bounds[0] + bounds[2]) / 2, (bounds[1] + bounds[3]) / 2];
        
        Ok(Some(json!({
            "found": true,
            "bounds": bounds,
            "center": center,
            "text": n.attribute("text").unwrap_or(""),
            "content-desc": n.attribute("content-desc").unwrap_or(""),
            "resource-id": n.attribute("resource-id").unwrap_or(""),
            "class": n.attribute("class").unwrap_or(""),
            "role": map_class_to_role(n.attribute("class").unwrap_or("")),
            "checked": n.attribute("checked") == Some("true"),
            "enabled": n.attribute("enabled") == Some("true"),
            "selected": n.attribute("selected") == Some("true"),
        })))
    } else {
        Ok(None)
    }
}

fn find_node<'a>(doc: &'a Document, locator: &str, value: &str) -> Option<Node<'a, 'a>> {
    doc.descendants().find(|n| {
        match locator {
            "text" => n.attribute("text").map(|t| t.contains(value)).unwrap_or(false),
            "role" => map_class_to_role(n.attribute("class").unwrap_or("")) == value,
            "resource-id" | "testid" => n.attribute("resource-id").map(|t| t.contains(value)).unwrap_or(false),
            "content-desc" | "label" | "alt" => n.attribute("content-desc").map(|t| t.contains(value)).unwrap_or(false),
            _ => false
        }
    })
}

pub fn handle_find(args: &[&str], serial: Option<&str>) -> Result<Value, String> {
    // usage: find <locator> <value> [action] [input_text] OR find <ref> [action] [input_text]
    if args.is_empty() {
        return Err("Usage: find <locator> <value> [action] [text] OR find <ref> [action] [text]".to_string());
    }
    
    let (locator, value, action, text_arg) = if state::is_ref_format(args[0]) {
        (args[0], "", args.get(1).copied().unwrap_or("click"), args.get(2).copied())
    } else {
        if args.len() < 2 {
            return Err("Usage: find <locator> <value> [action] [text]".to_string());
        }
        (args[0], args[1], args.get(2).copied().unwrap_or("click"), args.get(3).copied())
    };
    
    let result = find_element(locator, value, serial)?;
    
    if let Some(elem) = result {
        let center = elem["center"].as_array().unwrap();
        let x = center[0].as_i64().unwrap() as i32;
        let y = center[1].as_i64().unwrap() as i32;
        
        match action {
            "click" | "tap" => {
                adb::input_tap(x, y, serial)
            }
            "type" | "fill" => {
                let text = text_arg.unwrap_or("");
                adb::input_tap(x, y, serial)?; // Focus first
                adb::input_text(text, serial)
            }
            "longpress" => {
                 adb::input_swipe(x, y, x, y, 1000, serial)
            }
            "info" => {
                Ok(elem)
            }
            _ => Err(format!("Unknown action: {}", action))
        }
    } else {
        Err(format!("Element not found: {}={}", locator, value))
    }
}

pub fn handle_check(args: &[&str], serial: Option<&str>, target_state: bool) -> Result<Value, String> {
    // usage: check <locator> <value> OR check <ref>
    if args.is_empty() {
        return Err("Usage: check/uncheck <locator> <value> OR check/uncheck <ref>".to_string());
    }

    let (locator, value) = if state::is_ref_format(args[0]) {
        (args[0], "")
    } else {
        if args.len() < 2 {
             return Err("Usage: check/uncheck <locator> <value>".to_string());
        }
        (args[0], args[1])
    };

    let result = find_element(locator, value, serial)?;

    if let Some(elem) = result {
        let current_state = elem["checked"].as_bool().unwrap_or(false);
        if current_state == target_state {
             Ok(json!({ "result": true, "message": format!("Already {}", if target_state { "checked" } else { "unchecked" }) }))
        } else {
            let center = elem["center"].as_array().unwrap();
            let x = center[0].as_i64().unwrap() as i32;
            let y = center[1].as_i64().unwrap() as i32;
            adb::input_tap(x, y, serial)?;
            Ok(json!({ "result": true, "message": format!("Tapped to {}", if target_state { "check" } else { "uncheck" }) }))
        }
    } else {
        Err(format!("Element not found: {}={}", locator, value))
    }
}

pub fn handle_select(args: &[&str], serial: Option<&str>) -> Result<Value, String> {
    // usage: select <value> (1-step)
    // usage: select <locator> <value> (1-step, explicit)
    // usage: select <ref> <value> (2-step: click ref, wait, click value)
    // usage: select <locator> <loc_val> <value> (2-step: click trigger, wait, click value)
    
    if args.is_empty() {
        return Err("Usage: select <value> OR select <ref> <value>".to_string());
    }

    // Helper for clicking a found element
    let click_elem = |elem: Value| -> Result<Value, String> {
        let center = elem["center"].as_array().unwrap();
        let x = center[0].as_i64().unwrap() as i32;
        let y = center[1].as_i64().unwrap() as i32;
        adb::input_tap(x, y, serial)?;
        Ok(json!({ "result": true, "message": "Clicked element" }))
    };

    // 1-step: select "Option A"
    if args.len() == 1 {
        let value = args[0];
        let result = find_element("text", value, serial)?;
        if let Some(elem) = result {
             click_elem(elem)?;
             return Ok(json!({ "result": true, "message": format!("Selected '{}'", value) }));
        } else {
             return Err(format!("Element with text '{}' not found", value));
        }
    }

    // 1-step explicit: select text "Option A"
    let locators = ["text", "role", "resource-id", "content-desc", "class", "id", "label", "desc"];
    if args.len() == 2 && locators.contains(&args[0]) {
         let result = find_element(args[0], args[1], serial)?;
         if let Some(elem) = result {
             click_elem(elem)?;
             return Ok(json!({ "result": true, "message": format!("Selected element matching {}='{}'", args[0], args[1]) }));
         } else {
             return Err(format!("Element not found: {}={}", args[0], args[1]));
         }
    }

    // 2-step
    let (trigger_loc, trigger_val, target_value) = if state::is_ref_format(args[0]) && args.len() >= 2 {
        (args[0], "", args[1])
    } else if args.len() >= 3 {
        (args[0], args[1], args[2])
    } else {
        return Err("Usage: select <value> OR select <ref> <value> OR select <locator> <loc_val> <value>".to_string());
    };

    // 1. Find and click trigger
    let trigger = find_element(trigger_loc, trigger_val, serial)?;
    if let Some(elem) = trigger {
        click_elem(elem)?;
        
        // 2. Wait for dropdown animation
        std::thread::sleep(std::time::Duration::from_millis(1000));
        
        // 3. Force refresh UI dump
        let _ = crate::snapshot::dump_ui(serial)?;
        
        // 4. Find and click option
        let option_res = find_element("text", target_value, serial)?;
        if let Some(opt) = option_res {
             click_elem(opt)?;
             Ok(json!({ "result": true, "message": format!("Selected '{}' after clicking trigger", target_value) }))
        } else {
             Err(format!("Trigger clicked, but option '{}' not found", target_value))
        }
    } else {
        Err(format!("Trigger element not found: {}={}", trigger_loc, trigger_val))
    }
}

pub fn handle_is(args: &[&str], serial: Option<&str>) -> Result<Value, String> {
    // usage: is <state> <locator> <value> OR is <state> <ref>
    if args.len() < 2 {
        return Err("Usage: is <state> <locator> <value> OR is <state> <ref>".to_string());
    }
    
    let state = args[0];
    let (locator, value) = if state::is_ref_format(args[1]) {
        (args[1], "")
    } else {
        if args.len() < 3 {
             return Err("Usage: is <state> <locator> <value>".to_string());
        }
        (args[1], args[2])
    };
    
    let result = find_element(locator, value, serial)?;
    
    if let Some(elem) = result {
        match state {
            "visible" => {
                // If found and bounds > 0, it's visible
                let bounds = elem["bounds"].as_array().unwrap();
                let w = bounds[2].as_i64().unwrap() - bounds[0].as_i64().unwrap();
                let h = bounds[3].as_i64().unwrap() - bounds[1].as_i64().unwrap();
                let res = w > 0 && h > 0;
                Ok(json!({ "result": res, "message": res.to_string() }))
            }
            "enabled" => {
                let res = elem["enabled"].as_bool().unwrap_or(false);
                Ok(json!({ "result": res, "message": res.to_string() }))
            }
            "checked" => {
                let res = elem["checked"].as_bool().unwrap_or(false);
                Ok(json!({ "result": res, "message": res.to_string() }))
            }
            "selected" => {
                let res = elem["selected"].as_bool().unwrap_or(false);
                Ok(json!({ "result": res, "message": res.to_string() }))
            }
            _ => Err(format!("Unknown state: {}", state))
        }
    } else {
        // Not found means not visible
        if state == "visible" {
            Ok(json!({ "result": false, "message": "false" }))
        } else {
            Err(format!("Element not found: {}={}", locator, value))
        }
    }
}

pub fn handle_get(args: &[&str], serial: Option<&str>) -> Result<Value, String> {
    // usage: get <prop> <locator> <value> OR get <prop> <ref>
    if args.len() < 2 {
        return Err("Usage: get <prop> <locator> <value> OR get <prop> <ref>".to_string());
    }
    
    let prop = args[0];
    let (locator, value) = if state::is_ref_format(args[1]) {
        (args[1], "")
    } else {
        if args.len() < 3 {
            return Err("Usage: get <prop> <locator> <value>".to_string());
        }
        (args[1], args[2])
    };
    
    let result = find_element(locator, value, serial)?;
    
    if let Some(elem) = result {
        let val = match prop {
            "text" => elem["text"].clone(),
            "content-desc" | "label" => elem["content-desc"].clone(),
            "resource-id" | "id" => elem["resource-id"].clone(),
            "class" | "role" => elem["role"].clone(),
            "box" | "bounds" => elem["bounds"].clone(),
            "checked" => elem["checked"].clone(),
            "enabled" => elem["enabled"].clone(),
            _ => return Err(format!("Unknown property: {}", prop))
        };
        
        let msg = if val.is_string() {
            val.as_str().unwrap().to_string()
        } else {
            val.to_string()
        };
        
        Ok(json!({ "result": val, "message": msg }))
    } else {
        Err(format!("Element not found: {}={}", locator, value))
    }
}
