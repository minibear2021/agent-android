use std::collections::HashMap;
use roxmltree::Node;
use crate::adb;
use crate::state::RefData;
use serde_json::{json, Value};

pub fn get_xml_path(serial: Option<&str>) -> std::path::PathBuf {
    let filename = format!("window_dump_{}.xml", serial.unwrap_or("default"));
    std::env::temp_dir().join(filename)
}

pub fn dump_ui(serial: Option<&str>) -> Result<String, String> {
    // Ensure screen is on for UI dump
    adb::wake_screen(serial)?;

    // 1. Dump UI hierarchy
    let remote_path = "/sdcard/window_dump.xml";
    let local_path = get_xml_path(serial);
    let local_path_str = local_path.to_str().unwrap();

    // Clean up old dump
    let _ = adb::run_adb(&["shell", "rm", remote_path], serial);
    
    // Dump (uiautomator dump sometimes fails or returns empty, need retry logic in robust app, simplified here)
    let dump_out = adb::run_adb(&["shell", "uiautomator", "dump", remote_path], serial)?;
    if dump_out.contains("ERROR") {
        return Err(format!("Failed to dump UI: {}", dump_out));
    }

    // Pull
    adb::run_adb(&["pull", remote_path, local_path_str], serial)?;

    // Read XML
    std::fs::read_to_string(&local_path).map_err(|e| format!("Failed to read dump: {}", e))
}

pub fn get_snapshot(serial: Option<&str>, full_mode: bool, max_depth: Option<usize>, selector: Option<&str>) -> Result<Value, String> {
    let xml_content = dump_ui(serial)?;

    // Parse
    let doc = roxmltree::Document::parse(&xml_content).map_err(|e| format!("Invalid XML: {}", e))?;

    // Determine root node based on selector
    let root = if let Some(sel) = selector {
        find_node_by_selector(&doc, sel).ok_or_else(|| format!("Element not found: {}", sel))?
    } else {
        doc.root_element()
    };

    // Process
    let mut ref_counter = 0;
    let mut refs = HashMap::new();
    let mut tree_lines = Vec::new();

    // Use full doc traversal to ensure global stable IDs
    // But we only start *printing* from the selector root if provided
    let start_node = doc.root_element();
    
    // We need to know if the current node is within the selected subtree to decide whether to print it
    // But we must traverse everything to keep IDs stable.
    
    // Compact mode is active when NOT in full mode
    let compact_mode = !full_mode;

    process_node(
        start_node, 
        0, 
        &mut ref_counter, 
        &mut refs, 
        &mut tree_lines, 
        compact_mode, 
        max_depth,
        selector.map(|s| (s, root)) // Pass selector and the resolved root node
    );

    // No need to save state anymore
    // save_state(serial, refs.clone())?;

    // Format output similar to agent-browser
    let tree_str = tree_lines.join("\n");
    
    // Construct RefMap for JSON output
    let mut ref_map_json = serde_json::Map::new();
    for (k, v) in refs {
        ref_map_json.insert(k, json!({
            "role": v.role,
            "name": v.name,
            // Android specific fields can be added here
            // bounds and center removed to save tokens, available via 'get' command if needed
        }));
    }

    Ok(json!({
        "snapshot": tree_str,
        "refs": ref_map_json
    }))
}

fn find_node_by_selector<'a>(doc: &'a roxmltree::Document, selector: &str) -> Option<Node<'a, 'a>> {
    // Basic selector support:
    // "text=foo" -> text contains foo
    // "id=foo" -> resource-id contains foo
    // "role=foo" -> role equals foo
    // "foo" -> text contains foo (default)
    
    let (key, value) = if let Some(idx) = selector.find('=') {
        (&selector[..idx], &selector[idx+1..])
    } else {
        ("text", selector)
    };

    doc.descendants().find(|n| {
        match key {
            "text" => n.attribute("text").map(|t| t.contains(value)).unwrap_or(false),
            "id" | "resource-id" => n.attribute("resource-id").map(|t| t.contains(value)).unwrap_or(false),
            "role" => map_class_to_role(n.attribute("class").unwrap_or("")) == value,
            "desc" | "content-desc" => n.attribute("content-desc").map(|t| t.contains(value)).unwrap_or(false),
            "class" => n.attribute("class").map(|t| t.contains(value)).unwrap_or(false),
            _ => false
        }
    })
}

fn process_node<'a>(
    node: Node<'a, 'a>, 
    depth: usize, 
    counter: &mut i32, 
    refs: &mut HashMap<String, RefData>, 
    lines: &mut Vec<String>,
    compact_mode: bool,
    max_depth: Option<usize>,
    selector_info: Option<(&str, Node<'a, 'a>)>
) {
    // Global counter increment for every element node
    *counter += 1;
    let current_id = *counter;
    let ref_id = format!("e{}", current_id);

    // Determine if this node is within the selected subtree
    let in_scope = if let Some((_, root)) = selector_info {
        // Simple check: is this node the selector root or a descendant of it?
        // We can check if the node is equal to root or has root as ancestor
        // Optimization: We can check if node range is within root range
        node.range().start >= root.range().start && node.range().end <= root.range().end
    } else {
        true
    };

    // Calculate depth relative to the display root
    let display_depth = if let Some((_, root)) = selector_info {
        // If we are printing a subtree, the root of that subtree should be depth 0
        if in_scope {
            // This is an approximation, but effectively we want relative depth
            // We can calculate it by tracing ancestors until we hit root
            let mut d = 0;
            let mut curr = node;
            while curr != root {
                 if let Some(p) = curr.parent() {
                     curr = p;
                     d += 1;
                 } else {
                     break; 
                 }
            }
            d
        } else {
            0 // Won't be printed anyway
        }
    } else {
        depth
    };

    // Check max depth relative to display
    if let Some(max) = max_depth {
        if display_depth > max {
            // We still traverse children to keep IDs stable! 
            // But we don't print or add to refs map if out of depth scope?
            // Actually, if we don't traverse children, the counter won't increment for them.
            // So we MUST traverse children, but just not print them.
        }
    }
    
    // Only proceed with extraction if in scope
    if !in_scope {
        // Recurse to keep IDs stable
        for child in node.children() {
            if child.is_element() {
                process_node(child, depth + 1, counter, refs, lines, compact_mode, max_depth, selector_info);
            }
        }
        return;
    }

    // Check max depth for printing
    let depth_allowed = max_depth.map(|m| display_depth <= m).unwrap_or(true);

    let class = node.attribute("class").unwrap_or("");
    let resource_id = node.attribute("resource-id").unwrap_or("");
    let text = node.attribute("text").unwrap_or("");
    let content_desc = node.attribute("content-desc").unwrap_or("");
    let bounds_str = node.attribute("bounds").unwrap_or("[0,0][0,0]");
    let clickable = node.attribute("clickable") == Some("true");
    let enabled = node.attribute("enabled") == Some("true");
    let checkable = node.attribute("checkable") == Some("true");
    
    // Determine role
    let role = map_class_to_role(class);
    let is_structural = is_structural_role(role);
    
    // Determine name
    let name = if !text.is_empty() {
        text
    } else if !content_desc.is_empty() {
        content_desc
    } else {
        ""
    };

    // Determine if relevant
    let is_interactive = clickable || checkable || enabled && (role == "textbox" || role == "slider");
    let has_content = !name.is_empty();
    
    let should_include = if compact_mode && is_structural && !has_content && !is_interactive {
        // In compact mode, skip unnamed structural elements
        false
    } else {
        // In full mode, include everything that is not purely structural wrapper with no ID/name
        // But for clarity, let's keep it dense.
        // Include if interactive OR has text OR has resource-id
        is_interactive || has_content || !resource_id.is_empty()
    };

    if should_include && depth_allowed {
        let bounds = parse_bounds(bounds_str);
        let center = [(bounds[0] + bounds[2]) / 2, (bounds[1] + bounds[3]) / 2];

        // Always assign refs to included elements (interactive or not)
        // *counter is already incremented at top for stable ID generation
        refs.insert(ref_id.clone(), RefData {
            bounds,
            center,
            role: role.to_string(),
            name: if name.is_empty() { None } else { Some(name.to_string()) },
        });
        
        let ref_part = format!(" [ref={}]", ref_id);

        // Build line: - role "name" [ref=e1]
        let indent = "  ".repeat(display_depth);
        let name_part = if !name.is_empty() { format!(" \"{}\"", name) } else { "".to_string() };
        
        let line = if compact_mode {
            // Compact: - role "name" [ref=e1]
            // Format aligned with agent-browser (keep ref= prefix)
            // But remove ID for compactness
            let short_ref = format!(" [ref={}]", ref_id);
            format!("{}- {}{}{}", indent, role, name_part, short_ref)
        } else {
            // Standard: - role "name" [ref=e1] [id=foo]
            let id_part = if !resource_id.is_empty() { 
                // Extract simplified ID (after /)
                let simple_id = resource_id.split('/').last().unwrap_or(resource_id);
                format!(" [id={}]", simple_id) 
            } else { 
                "".to_string() 
            };
            format!("{}- {}{}{}{}", indent, role, name_part, ref_part, id_part)
        };
        
        lines.push(line);
    }

    // Recurse
    // If we skipped printing this node due to compaction, we pass the SAME depth to children
    // to flatten the tree. Otherwise, we increase depth.
    let next_depth = if should_include && depth_allowed { depth + 1 } else { depth };
    
    for child in node.children() {
        if child.is_element() {
            process_node(child, next_depth, counter, refs, lines, compact_mode, max_depth, selector_info);
        }
    }
}

pub fn map_class_to_role(class: &str) -> &str {
    if class.ends_with(".Button") || class.ends_with(".ImageButton") {
        "button"
    } else if class.ends_with(".EditText") {
        "textbox"
    } else if class.ends_with(".CheckBox") {
        "checkbox"
    } else if class.ends_with(".RadioButton") {
        "radio"
    } else if class.ends_with(".ImageView") {
        "image"
    } else if class.ends_with(".TextView") {
        "text"
    } else if class.ends_with(".ScrollView") || class.ends_with(".ListView") || class.ends_with(".RecyclerView") {
        "list"
    } else {
        // Fallback: simplified class name
        class.split('.').last().unwrap_or("element")
    }
}

pub fn is_structural_role(role: &str) -> bool {
    matches!(role, 
        "LinearLayout" | "FrameLayout" | "RelativeLayout" | "ConstraintLayout" | 
        "View" | "ViewGroup" | "DrawerLayout" | "CoordinatorLayout" | 
        "list" | "scroll" | "element"
    )
}

pub fn parse_bounds(bounds: &str) -> [i32; 4] {
    // [x1,y1][x2,y2]
    let parts: Vec<&str> = bounds.split(|c| c == '[' || c == ']' || c == ',').filter(|s| !s.is_empty()).collect();
    if parts.len() == 4 {
        [
            parts[0].parse().unwrap_or(0),
            parts[1].parse().unwrap_or(0),
            parts[2].parse().unwrap_or(0),
            parts[3].parse().unwrap_or(0),
        ]
    } else {
        [0, 0, 0, 0]
    }
}
