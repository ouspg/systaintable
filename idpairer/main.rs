use chrono::{DateTime, Utc};
use serde::Serialize;
use std::io::{self, BufRead};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct InputRow {
    row_number: i32,
    timestamp: DateTime<Utc>,
    identity_type: String,
    identity_value: String,
}

#[derive(Debug, Serialize)]
struct IdentityGroup {
    identities: HashMap<String, String>, // type -> value mapping
    row_numbers: Vec<i32>,
    timestamp_range: (String, String),
}

fn main() -> io::Result<()> {
    // Read input from stdin
    let stdin = io::stdin();
    let mut input_rows = Vec::new();
    
    for line in stdin.lock().lines() {
        let line = line?.trim().to_string();
        if line.is_empty() || line.starts_with("\"") || line.ends_with("\"") {
            continue; // Skip quotes or empty lines
        }
        
        if let Some(row) = parse_input_row(&line) {
            input_rows.push(row);
        }
    }
    
    // Sort input rows by timestamp
    input_rows.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    
    // Group rows by timestamp proximity (within 1 second)
    let identity_groups = group_by_timestamp_proximity(&input_rows);
    
    // Output as JSON
    let json = serde_json::to_string_pretty(&identity_groups)?;
    println!("{}", json);
    
    Ok(())
}

fn parse_input_row(line: &str) -> Option<InputRow> {
    let parts: Vec<&str> = line.splitn(4, ',').collect();
    if parts.len() != 4 {
        eprintln!("Invalid input format: {}", line);
        return None;
    }
    
    let row_number = match parts[0].trim().parse::<i32>() {
        Ok(num) => num,
        Err(_) => {
            eprintln!("Invalid row number: {}", parts[0]);
            return None;
        }
    };
    
    let timestamp = match chrono::DateTime::parse_from_rfc3339(parts[1].trim()) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => {
            // Try other common formats if RFC3339 fails
            match chrono::NaiveDateTime::parse_from_str(parts[1].trim(), "%Y-%m-%d %H:%M:%S%.f") {
                Ok(dt) => DateTime::<Utc>::from_utc(dt, Utc),
                Err(_) => {
                    eprintln!("Invalid timestamp format: {}", parts[1]);
                    return None;
                }
            }
        }
    };
    
    let identity_type = parts[2].trim().to_string();
    let identity_value = parts[3].trim().to_string();
    
    Some(InputRow {
        row_number,
        timestamp,
        identity_type,
        identity_value,
    })
}

fn group_by_timestamp_proximity(rows: &[InputRow]) -> Vec<IdentityGroup> {
    if rows.is_empty() {
        return Vec::new();
    }
    
    let mut groups = Vec::new();
    let mut current_group_identities = HashMap::new();
    let mut current_group_rows = Vec::new();
    let mut current_min_ts = rows[0].timestamp;
    let mut current_max_ts = rows[0].timestamp;
    
    for row in rows {
        // If this row's timestamp is more than 1 second after the current group's max timestamp,
        // start a new group
        if (row.timestamp - current_max_ts).num_milliseconds().abs() > 1000 {
            if !current_group_identities.is_empty() {
                groups.push(IdentityGroup {
                    identities: current_group_identities,
                    row_numbers: current_group_rows,
                    timestamp_range: (current_min_ts.to_rfc3339(), current_max_ts.to_rfc3339()),
                });
                current_group_identities = HashMap::new();
                current_group_rows = Vec::new();
            }
            current_min_ts = row.timestamp;
            current_max_ts = row.timestamp;
        } else {
            // Update the current group's timestamp range
            if row.timestamp > current_max_ts {
                current_max_ts = row.timestamp;
            }
            if row.timestamp < current_min_ts {
                current_min_ts = row.timestamp;
            }
        }
        
        // Add the row to the current group
        current_group_identities.insert(row.identity_type.clone(), row.identity_value.clone());
        current_group_rows.push(row.row_number);
    }
    
    // Don't forget the last group
    if !current_group_identities.is_empty() {
        groups.push(IdentityGroup {
            identities: current_group_identities,
            row_numbers: current_group_rows,
            timestamp_range: (current_min_ts.to_rfc3339(), current_max_ts.to_rfc3339()),
        });
    }
    
    groups
}