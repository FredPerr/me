use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct ListeningPort {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub cwd: String,
}

#[tauri::command]
pub async fn list_listening_ports() -> Result<Vec<ListeningPort>, String> {
    let output = std::process::Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-P", "-n", "-F", "pcn"])
        .output()
        .map_err(|e| format!("Failed to run lsof: {}", e))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries: Vec<(u32, String, u16)> = Vec::new();
    let mut current_pid: Option<u32> = None;
    let mut current_name: Option<String> = None;

    for line in stdout.lines() {
        if let Some(pid_str) = line.strip_prefix('p') {
            current_pid = pid_str.parse().ok();
            current_name = None;
        } else if let Some(name) = line.strip_prefix('c') {
            current_name = Some(name.to_string());
        } else if let Some(addr) = line.strip_prefix('n') {
            if let (Some(pid), Some(ref name)) = (current_pid, &current_name) {
                if let Some(port) = parse_port_from_address(addr) {
                    entries.push((pid, name.clone(), port));
                }
            }
        }
    }

    let unique_pids: Vec<u32> = entries.iter().map(|(pid, _, _)| *pid).collect::<std::collections::HashSet<_>>().into_iter().collect();
    let cwd_map = resolve_cwds(&unique_pids);

    let results: Vec<ListeningPort> = entries
        .into_iter()
        .filter_map(|(pid, process_name, port)| {
            let cwd = cwd_map.get(&pid)?.clone();
            Some(ListeningPort {
                port,
                pid,
                process_name,
                cwd,
            })
        })
        .collect();

    Ok(results)
}

fn parse_port_from_address(addr: &str) -> Option<u16> {
    // Handle IPv6 format "[::1]:port" or "*:port" or "host:port"
    if let Some(bracket_end) = addr.rfind(']') {
        // IPv6: [::1]:port
        let after_bracket = &addr[bracket_end + 1..];
        let port_str = after_bracket.strip_prefix(':')?;
        return port_str.parse().ok();
    }
    let colon_pos = addr.rfind(':')?;
    addr[colon_pos + 1..].parse().ok()
}

fn resolve_cwds(pids: &[u32]) -> HashMap<u32, String> {
    let mut map = HashMap::new();

    for &pid in pids {
        if let Some(cwd) = get_process_cwd(pid) {
            map.insert(pid, cwd);
        }
    }

    map
}

fn get_process_cwd(pid: u32) -> Option<String> {
    let output = std::process::Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-F", "n"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix('n') {
            if path.starts_with('/') {
                return Some(path.to_string());
            }
        }
    }

    None
}
