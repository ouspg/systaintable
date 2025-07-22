use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufReader, Read, BufRead};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

use clap::Parser;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use indicatif::{ProgressBar, ProgressStyle};
use num_cpus;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Entry {
    line: u32,
    timestamp: String,
    #[serde(rename = "type")]
    entry_type: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity: Option<String>,
}

// Optimized Union-Find data structure
struct DisjointSet {
    parent: Vec<usize>,
    rank: Vec<usize>,
    size: usize,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        let mut parent = vec![0; size];
        for i in 0..size {
            parent[i] = i;
        }
        DisjointSet {
            parent,
            rank: vec![0; size],
            size,
        }
    }

    #[inline]
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    #[inline]
    fn union(&mut self, x: usize, y: usize) -> bool {
        let root_x = self.find(x);
        let root_y = self.find(y);

        if root_x == root_y {
            return false;
        }

        if self.rank[root_x] < self.rank[root_y] {
            self.parent[root_x] = root_y;
        } else {
            self.parent[root_y] = root_x;
            if self.rank[root_x] == self.rank[root_y] {
                self.rank[root_x] += 1;
            }
        }
        true
    }
}

struct MergeLog {
    group_id: String,
    reason: String,
    group_a_values: Vec<String>,
    group_b_values: Vec<String>,
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Input JSON file path
    #[clap(short, long, parse(from_os_str))]
    input: PathBuf,

    /// Combine classified values by line number
    #[clap(short, long)]
    line: bool,
    
    /// Enable debug output
    #[clap(short, long)]
    debug: bool,
    
    /// Number of entries to process (for testing)
    #[clap(long)]
    limit: Option<usize>,
    
    /// Comma-separated list of types to use for identity merging (e.g., "IP,DNSname")
    #[clap(short = 't', long = "types")]
    merge_types: Option<String>,
    
    /// Maximum frequency (%) for a value to be used for merging (e.g., 5 means ignore values appearing in >5% of lines)
    #[clap(short = 'f', long = "max-freq", default_value = "10")]
    max_frequency: f64,
    
    /// Analyze mode - just show value statistics without merging
    #[clap(short = 'a', long = "analyze")]
    analyze: bool,

    /// Skip transitive closure step for faster processing
    #[clap(long = "fast")]
    fast_mode: bool,
    
    /// Number of threads to use (0 = auto)
    #[clap(short = 'j', long = "threads", default_value = "0")]
    threads: usize,
    
    /// Output file path for selected identity JSON
    #[clap(short = 'o', long = "output")]
    output: Option<PathBuf>,

    #[clap(long)]
    verbose_merges: bool,
}
fn process_batch_and_update_disjoint(
    batch: &mut HashMap<String, Vec<u32>>,
    _person_groups: &mut Vec<HashSet<u32>>,  // Unused but kept for compatibility 
    max_occurrences: usize,
    transitive_merges: &mut usize,
    merge_logs: &mut Vec<String>,
    verbose: bool,
    line_to_index: &HashMap<u32, usize>,
    disjoint_set: &mut DisjointSet
) {
    // Hard limit on lines per value to prevent memory issues
    const ABSOLUTE_MAX_LINES: usize = 2000;
    let effective_max = max_occurrences.min(ABSOLUTE_MAX_LINES);
    
    // Process each value directly and update disjoint set only (skip person_groups)
    for (value, lines) in batch.iter() {
        // Skip overly common values completely
        if lines.len() > effective_max {
            continue;
        }
        
        // Process all pairs sharing this value directly by updating the disjoint set
        if lines.len() >= 2 {
            // For very common values, just connect sequential pairs
            if lines.len() > 1000 {
                let mut prev_idx = None;
                
                for &line in lines {
                    if let Some(&line_idx) = line_to_index.get(&line) {
                        if let Some(prev) = prev_idx {
                            if disjoint_set.union(prev, line_idx) {
                                *transitive_merges += 1;
                            }
                        }
                        prev_idx = Some(line_idx);
                    }
                }
                
                // Log if verbose
                if verbose && merge_logs.len() < 100 {
                    merge_logs.push(format!(
                        "MERGED: Connected {} lines sequentially for common value {}", 
                        lines.len(), value
                    ));
                }
            } else {
                // For smaller values, we can afford to process all pairs
                for i in 0..lines.len() {
                    if let Some(&idx_i) = line_to_index.get(&lines[i]) {
                        for j in i+1..lines.len() {
                            if let Some(&idx_j) = line_to_index.get(&lines[j]) {
                                if disjoint_set.union(idx_i, idx_j) {
                                    *transitive_merges += 1;
                                }
                            }
                        }
                    }
                }
                
                // Log if verbose
                if verbose && merge_logs.len() < 100 {
                    merge_logs.push(format!(
                        "MERGED: Connected all {} lines sharing value {}", 
                        lines.len(), value
                    ));
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let debug = args.debug;

    // Start timing
    let start = Instant::now();
    println!("Reading from file: {}", args.input.display());
    
    // Configure thread pool if specified
    if args.threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global()
            .unwrap();
        println!("Using {} threads as specified", args.threads);
    } else {
        println!("Using {} threads (auto-detected)", num_cpus::get());
    }

    // Load and parse JSON data with progress indicator
    let file = File::open(&args.input)?;
    let file_size = file.metadata()?.len();
    let reader = BufReader::new(file);
    
    // Use streaming deserializer for better performance with large files
    println!("Parsing JSON data...");
    let pb = ProgressBar::new(file_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .expect("Failed to parse template")
        .progress_chars("#>-"));
    
    // Parse JSON data
    let mut entries: Vec<Entry> = serde_json::from_reader(reader)?;
    if let Some(limit) = args.limit {
        entries.truncate(limit);
        println!("Limited to first {} entries", limit);
    }
    pb.finish_with_message("JSON parsing complete");
    
    println!("Loaded {} entries in {:.2}s", entries.len(), start.elapsed().as_secs_f64());
    
    if args.line {
        let process_start = Instant::now();
        
        println!("Building line number mappings...");
        
        // More efficient way to build mappings - use a more direct approach
        let mut all_lines: Vec<u32> = entries.iter()
            .map(|e| e.line)
            .collect();
            
        println!("Collected all line numbers in {:.2}s", process_start.elapsed().as_secs_f64());
        
        // Sort and dedup in place is much faster than using a HashSet
        all_lines.sort_unstable();
        all_lines.dedup();
        
        let line_count = all_lines.len();
        println!("Found {} unique line numbers in {:.2}s", line_count, process_start.elapsed().as_secs_f64());
        
        // More efficient construction of line_to_index
        let mut line_to_index = HashMap::with_capacity(line_count);
        for (idx, &line) in all_lines.iter().enumerate() {
            line_to_index.insert(line, idx);
        }
        
        // Use the existing all_lines vector directly as index_to_line
        let index_to_line = all_lines;
        
        println!("Built line mappings in {:.2}s", process_start.elapsed().as_secs_f64());
        
        // Parse types filter if provided - use as_ref() to avoid moving the value
        let merge_types: Option<HashSet<String>> = args.merge_types.as_ref().map(|types_str| {
            types_str.split(',').map(|s| s.trim().to_string()).collect()
        });
        
        // More efficient value-to-lines mapping - process in smaller chunks
        println!("Building value-to-line mappings (this may take a while for large datasets)...");
        
        // Use smaller chunk size for better memory management
        let optimal_chunk_size = std::cmp::min(100000, entries.len() / (num_cpus::get() * 4));
        
        // Pre-allocate capacity for better performance - with proper type annotations
        let value_to_lines_mutex = Arc::new(Mutex::new(HashMap::<String, Vec<usize>>::with_capacity(100000)));
        let value_frequency_mutex = Arc::new(Mutex::new(HashMap::<String, usize>::with_capacity(100000)));
        
        // Process entries in parallel with progress bar
        let pb = ProgressBar::new(entries.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} entries ({eta})")
            .expect("Failed to parse template")
            .progress_chars("#>-"));
        
        // Process in smaller chunks with less frequent locking
        entries.par_chunks(optimal_chunk_size)
            .for_each(|chunk| {
                // Use more memory-efficient local maps with explicit type annotations
                let mut local_value_to_lines: HashMap<String, Vec<usize>> = HashMap::new();
                let mut local_value_frequency: HashMap<String, usize> = HashMap::new();
                
                for entry in chunk {
                    // Make a clone of merge_types reference for the closure
                    let merge_types_ref = &merge_types;
                    
                    // Skip if entry type isn't in the merge_types list (if provided)
                    if let Some(ref types) = merge_types_ref {
                        if !types.contains(&entry.entry_type) {
                            continue;
                        }
                    }
                    
                    let key = format!("{}:{}", entry.entry_type, entry.value);
                    
                    // Handle invalid line numbers gracefully
                    if let Some(&index) = line_to_index.get(&entry.line) {
                        local_value_to_lines.entry(key.clone()).or_default().push(index);
                        *local_value_frequency.entry(key).or_insert(0) += 1;
                    }
                }
                
                // Lock only once after processing the entire chunk
                let mut global_value_to_lines = value_to_lines_mutex.lock().unwrap();
                let mut global_value_frequency = value_frequency_mutex.lock().unwrap();
                
                for (key, mut indices) in local_value_to_lines {
                    global_value_to_lines.entry(key).or_default().append(&mut indices);
                }
                
                for (key, count) in local_value_frequency {
                    *global_value_frequency.entry(key).or_insert(0) += count;
                }
                
                // Update progress
                pb.inc(chunk.len() as u64);
            });
        
        pb.finish_with_message("Value mapping complete");
    
        // Extract results from the mutexes
        let mut value_to_lines = Arc::try_unwrap(value_to_lines_mutex)
            .expect("Failed to unwrap value_to_lines_mutex")
            .into_inner()
            .unwrap();
        
        let value_frequency = Arc::try_unwrap(value_frequency_mutex)
            .expect("Failed to unwrap value_frequency_mutex")
            .into_inner()
            .unwrap();
                    
        // Continue with value processing - REMOVE THE DUPLICATE CODE BELOW
        println!("Grouping entries by line number...");
        
        // Calculate max occurrences based on max frequency percentage
        let max_occurrences = (args.max_frequency / 100.0 * line_to_index.len() as f64) as usize;
        println!("Values appearing in more than {} lines ({:.1}%) will be excluded from identity merging", 
            max_occurrences, args.max_frequency);
        
        // Show most common values
        // Find top 10 most common values without collecting the entire sorted vector
        let mut top_values = Vec::with_capacity(10);
        for (k, &v) in &value_frequency {
            if top_values.len() < 10 {
                top_values.push((k.clone(), v));
                // Keep it sorted
                top_values.sort_by(|a, b| b.1.cmp(&a.1));
            } else if v > top_values.last().unwrap().1 {
                // Replace the smallest entry if this one is larger
                top_values.pop();
                top_values.push((k.clone(), v));
                top_values.sort_by(|a, b| b.1.cmp(&a.1));
            }
        }

        println!("\nTop 10 most common values:");
        for (i, (value, count)) in top_values.iter().enumerate() {
            println!("{}. {} - appears in {} lines ({:.2}%)", 
                i+1, value, count, (*count as f64 / line_to_index.len() as f64) * 100.0);
        }

        if args.analyze {
            println!("\nAnalysis complete. Use --types and --max-freq to control identity merging.");
            return Ok(());
        }

        // Filter out overly common values
        value_to_lines.retain(|_key, lines| {
            // Keep values that appear in at least 2 lines but fewer than max_occurrences
            lines.len() >= 2 && lines.len() <= max_occurrences
        });
        
        drop(value_frequency);
        
        println!("\nUsing {} values for identity merging", value_to_lines.len());
        
        // Step 3: Merge identities using Union-Find
        let union_start = Instant::now();
        println!("Merging identities...");
        
        let pb = ProgressBar::new(value_to_lines.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} values processed ({eta})")
            .expect("Failed to parse template")
            .progress_chars("#>-"));
        
        // Create DisjointSet for tracking identity groups
        let mut disjoint_set = DisjointSet::new(line_to_index.len());
        
        // First pass: Build a mapping of line indices to the values they contain
        let line_to_values_mutex = Arc::new(Mutex::new(HashMap::<usize, HashSet<String>>::new()));
        
        // Build line-to-values mapping in parallel
        value_to_lines.par_iter()
            .for_each(|(value, line_indices)| {
                let mut local_map = HashMap::<usize, HashSet<String>>::new();
                
                for &line_idx in line_indices {
                    local_map.entry(line_idx).or_default().insert(value.clone());
                }
                
                // Merge local map into global map
                let mut global_map = line_to_values_mutex.lock().unwrap();
                for (line_idx, values) in local_map {
                    global_map.entry(line_idx).or_default().extend(values);
                }
            });
        
        let line_to_values = Arc::try_unwrap(line_to_values_mutex)
            .expect("Failed to unwrap line_to_values_mutex")
            .into_inner()
            .unwrap();
        
        // Second pass: For each value, merge all lines that contain it
        let mut merge_count = 0;
        
        // Split values into chunks for parallel processing
        let value_chunks: Vec<Vec<String>> = {
            let value_keys: Vec<String> = value_to_lines.keys().cloned().collect();
            let chunk_size = std::cmp::max(1, value_keys.len() / num_cpus::get());
            
            let mut chunks = Vec::new();
            let mut current_chunk = Vec::new();
            
            for key in value_keys {
                current_chunk.push(key);
                
                if current_chunk.len() >= chunk_size {
                    chunks.push(std::mem::replace(&mut current_chunk, Vec::new()));
                }
            }
            
            if !current_chunk.is_empty() {
                chunks.push(current_chunk);
            }
            
            chunks
        };
        
        // Process each chunk of values in parallel
        let merge_ops: Vec<Vec<(usize, usize)>> = value_chunks.par_iter()
            .map(|chunk| {
                let mut local_merges = Vec::new();
                
                for value in chunk {
                    let line_indices = &value_to_lines[value];
                    
                    if line_indices.len() >= 2 {
                        // For each pair of lines that share this value
                        for i in 0..line_indices.len() {
                            for j in i+1..line_indices.len() {
                                local_merges.push((line_indices[i], line_indices[j]));
                            }
                        }
                    }
                    
                    // Update progress bar
                    pb.inc(1);
                }
                
                local_merges
            })
            .collect();
        
        // Apply all merge operations to the DisjointSet
        for merges in merge_ops {
            for (a, b) in merges {
                if disjoint_set.union(a, b) {
                    merge_count += 1;
                    if debug {
                        println!("Merged lines {} and {} due to shared value", 
                            index_to_line[a], index_to_line[b]);
                    }
                }
            }
        }
        
        println!("\nApplied {} initial merges", merge_count);
        
        // Apply transitive closure unless fast mode is enabled
        let mut transitive_merges = 0;
        let mut identity_groups: HashMap<usize, Vec<usize>> = HashMap::new();


        if !args.fast_mode {
            println!("Building identity groups with transitive closure...");
            let closure_start = Instant::now();
            
            // We need to implement proper transitive closure by processing the values again
            // and ensuring all lines that share ANY value are connected
            
            println!("Applying transitive closure to merge identities sharing values...");
            
            // Calculate frequency threshold
            let max_occurrences = if args.max_frequency > 0.0 {
                (args.max_frequency / 100.0 * line_count as f64) as usize
            } else {
                usize::MAX
            };
            
            // Collect all value-line relationships for non-common values
            let mut value_to_lines: HashMap<String, Vec<usize>> = HashMap::new();
            
            for (idx, entry) in entries.iter().enumerate() {
                // Apply type filter if specified
                if let Some(ref types_str) = args.merge_types {
                    let allowed_types: HashSet<&str> = types_str.split(',').map(|s| s.trim()).collect();
                    if !allowed_types.contains(entry.entry_type.as_str()) {
                        continue;
                    }
                }
                
                let key = format!("{}:{}", entry.entry_type, entry.value);
                value_to_lines.entry(key).or_default().push(idx);
            }
            
            // Filter out common values
            let original_count = value_to_lines.len();
            value_to_lines.retain(|_, indices| indices.len() <= max_occurrences);
            println!("Using {} values for transitive closure (filtered from {})", 
                     value_to_lines.len(), original_count);
            
            // Now apply transitive closure: if two entries share ANY value, they should be in the same identity
            let mut transitive_merges = 0;
            let pb = ProgressBar::new(value_to_lines.len() as u64);
            pb.set_message("Applying transitive closure...");
            
            for (value, entry_indices) in value_to_lines {
                // For each value, get the line indices for all entries that have this value
                let mut line_indices = Vec::new();
                for &entry_idx in &entry_indices {
                    if let Some(&line_idx) = line_to_index.get(&entries[entry_idx].line) {
                        line_indices.push(line_idx);
                    }
                }
                
                // Connect all lines that share this value
                if line_indices.len() >= 2 {
                    let first_line_idx = line_indices[0];
                    for &other_line_idx in &line_indices[1..] {
                        if disjoint_set.union(first_line_idx, other_line_idx) {
                            transitive_merges += 1;
                        }
                    }
                }
                
                pb.inc(1);
            }
            
            pb.finish_with_message(format!("Applied {} transitive merges", transitive_merges));
            
            // Now build identity groups from the updated disjoint set
            println!("Building final identity groups from updated disjoint set...");
            
            // Map each line to its root identity using the updated disjoint_set
            let line_to_root: HashMap<usize, usize> = (0..disjoint_set.size)
                .map(|i| (i, disjoint_set.find(i)))
                .collect();
            
            // Group entries by identity root
            let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
            for (idx, entry) in entries.iter().enumerate() {
                if let Some(&line_idx) = line_to_index.get(&entry.line) {
                    let root = line_to_root[&line_idx];
                    groups.entry(root).or_default().push(idx);
                }
            }
            
            println!("Built {} identity groups with {} transitive merges in {:.2}s", 
                     groups.len(), transitive_merges, closure_start.elapsed().as_secs_f64());
            identity_groups = groups;
        } else {
            println!("Fast mode enabled - skipping transitive closure step");
            
            // Use the original identity building logic for fast mode
            let build_start = Instant::now();
            println!("Building identity groups (fast mode)...");
            
            // Map each line to its root identity
            let line_to_root: HashMap<usize, usize> = (0..disjoint_set.size)
                .map(|i| (i, disjoint_set.find(i)))
                .collect();
            
            // Group entries by identity root
            let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
            for (idx, entry) in entries.iter().enumerate() {
                if let Some(&line_idx) = line_to_index.get(&entry.line) {
                    let root = line_to_root[&line_idx];
                    groups.entry(root).or_default().push(idx);
                }
            }
            
            println!("Built {} identity groups in {:.2}s", groups.len(), build_start.elapsed().as_secs_f64());
            identity_groups = groups;
        }
                
        println!("Total number of identity groups: {}", identity_groups.len());        // Map roots to sequential identity numbers
        let mut roots: Vec<usize> = identity_groups.keys().cloned().collect();
        roots.sort_unstable();
        
        let root_to_identity: HashMap<usize, u32> = roots.iter()
            .enumerate()
            .map(|(i, &root)| (root, (i + 1) as u32))
            .collect();
        
        // Display available identities
        println!("\nAvailable identities:");
        for &root in &roots {
            let identity_number = root_to_identity[&root];
            let entry_indices = &identity_groups[&root];
            
            // Find all unique line numbers in this identity
            let mut lines = HashSet::new();
            let mut value_types = HashSet::new();
            
            for &idx in entry_indices {
                let entry = &entries[idx];
                lines.insert(entry.line);
                value_types.insert(format!("{}:{}", entry.entry_type, entry.value));
            }
            
            let line_str = if lines.len() <= 5 {
                lines.iter()
                    .map(|&l| l.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                format!("{} unique lines", lines.len())
            };
            
            let value_sample = value_types.iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            
            let value_str = if value_types.len() > 3 {
                format!("{} (+ {} more)", value_sample, value_types.len() - 3)
            } else {
                value_sample
            };
            
            println!("Identity_{}: {} entries across {} - {}",
                identity_number,
                entry_indices.len(),
                line_str,
                value_str
            );
        }
        
        // Get user selection
        println!("\nEnter the identity number to trace (e.g. 1 for Identity_1):");
        let mut selection = String::new();
        io::stdin().read_line(&mut selection)?;
        let selected_id: u32 = match selection.trim().parse() {
            Ok(id) => id,
            Err(_) => {
                println!("Invalid number. Using identity 1.");
                1
            }
        };
        
        // Find selected root
        let selected_root = roots.iter()
            .find(|&&root| root_to_identity[&root] == selected_id)
            .cloned();
        
        if let Some(root) = selected_root {
            // Output the selected identity
            let output_start = Instant::now();
            
            // Get all entry indices for this identity
            let entry_indices = &identity_groups[&root];
            let mut filtered_entries = Vec::with_capacity(entry_indices.len());
            
            // Add identity field to entries
            for &idx in entry_indices {
                let mut entry = entries[idx].clone();
                entry.identity = Some(format!("Identity_{}", selected_id));
                filtered_entries.push(entry);
            }
            
            println!("\nEntries for Identity_{}: ({} entries)", selected_id, filtered_entries.len());
            let json = serde_json::to_string_pretty(&filtered_entries)?;
            
            // Write to file or print to console
            if let Some(ref output_path) = args.output {
                std::fs::write(output_path, &json)?;
                println!("Output written to {}", output_path.display());
            } else {
                println!("{}", json);
            }
            
            println!("Output generated in {:.2}s", output_start.elapsed().as_secs_f64());
        } else {
            println!("Invalid identity number: {}", selected_id);
        }
    } else {
        println!("Use -l or --line to enable identity tracing");
    }
    
    println!("Total execution time: {:.2}s", start.elapsed().as_secs_f64());
    Ok(())
}