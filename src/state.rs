use std::fs;
use serde::{Deserialize, Serialize};
use roxmltree::Node;
use crate::snapshot;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RefData {
    pub bounds: [i32; 4], // [x1, y1, x2, y2]
    pub center: [i32; 2], // [x, y]
    pub role: String,
    pub name: Option<String>,
}

pub fn is_ref_format(s: &str) -> bool {
    s.starts_with("@") || s.starts_with("ref=") || (s.starts_with("e") && s[1..].chars().all(char::is_numeric))
}

pub fn parse_ref_id(ref_id: &str) -> Result<i32, String> {
    let id_str = if ref_id.starts_with('@') {
        &ref_id[1..]
    } else if ref_id.starts_with("ref=") {
        &ref_id[4..]
    } else {
        ref_id
    };
    
    // Parse numeric ID (e.g. "e123" -> 123)
    if id_str.starts_with('e') {
        id_str[1..].parse::<i32>().map_err(|_| format!("Invalid ref ID format: {}", ref_id))
    } else {
        // Fallback for raw numbers if used
        id_str.parse::<i32>().map_err(|_| format!("Invalid ref ID format: {}", ref_id))
    }
}

pub fn resolve_ref(ref_id: &str, serial: Option<&str>) -> Result<RefData, String> {
    let target_id = parse_ref_id(ref_id)?;

    // Load XML
    let xml_path = snapshot::get_xml_path(serial);
    if !xml_path.exists() {
        return Err("No snapshot found. Run 'snapshot' first.".to_string());
    }
    
    let xml_content = fs::read_to_string(&xml_path).map_err(|e| format!("Failed to read snapshot: {}", e))?;
    let doc = roxmltree::Document::parse(&xml_content).map_err(|e| format!("Invalid XML: {}", e))?;
    
    // Traverse and find the N-th element
    let mut counter = 0;
    
    // We need to use the exact same traversal logic as snapshot.rs to ensure IDs match
    // snapshot.rs: process_node iterates children, increments counter for every element node
    
    // Use recursive search to find the matching ID
    if let Some(node) = find_node_by_counter(doc.root_element(), &mut counter, target_id) {
        // Found it! Extract data
        let class = node.attribute("class").unwrap_or("");
        let text = node.attribute("text").unwrap_or("");
        let content_desc = node.attribute("content-desc").unwrap_or("");
        let bounds_str = node.attribute("bounds").unwrap_or("[0,0][0,0]");
        
        let role = snapshot::map_class_to_role(class);
        let name = if !text.is_empty() {
            text
        } else if !content_desc.is_empty() {
            content_desc
        } else {
            ""
        };
        
        let bounds = snapshot::parse_bounds(bounds_str);
        let center = [(bounds[0] + bounds[2]) / 2, (bounds[1] + bounds[3]) / 2];
        
        Ok(RefData {
            bounds,
            center,
            role: role.to_string(),
            name: if name.is_empty() { None } else { Some(name.to_string()) },
        })
    } else {
        Err(format!("Ref '{}' not found in current snapshot (max ID: {})", ref_id, counter))
    }
}

pub fn find_node_by_counter<'a>(node: Node<'a, 'a>, counter: &mut i32, target: i32) -> Option<Node<'a, 'a>> {
    *counter += 1;
    
    if *counter == target {
        return Some(node);
    }
    
    for child in node.children() {
        if child.is_element() {
            if let Some(found) = find_node_by_counter(child, counter, target) {
                return Some(found);
            }
        }
    }
    
    None
}
