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
                    
        // Continue with value processing
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
        let mut merge_count = 0;

        // Process values in smaller batches to control memory usage
        const BATCH_SIZE: usize = 1000; // Process 1000 values at a time
        let value_keys: Vec<String> = value_to_lines.keys().cloned().collect();

        println!("Processing {} values in batches of {}", value_keys.len(), BATCH_SIZE);
        
        for batch_start in (0..value_keys.len()).step_by(BATCH_SIZE) {
            let batch_end = (batch_start + BATCH_SIZE).min(value_keys.len());
            let batch_keys = &value_keys[batch_start..batch_end];
            
            // Process this batch of values
            for value in batch_keys {
                let line_indices = &value_to_lines[value];
                
                if line_indices.len() >= 2 {
                    // For each pair of lines that share this value, merge them
                    let first_line = line_indices[0];
                    for &other_line in &line_indices[1..] {
                        if disjoint_set.union(first_line, other_line) {
                            merge_count += 1;
                            if debug {
                                println!("Merged lines {} and {} due to shared value {}", 
                                    index_to_line[first_line], index_to_line[other_line], value);
                            }
                        }
                    }
                }
                
                pb.inc(1);
            }
            
            // Force garbage collection after each batch and show progress
            if batch_start % (BATCH_SIZE * 10) == 0 {
                println!("Processed {} values, applied {} merges so far", batch_start + batch_keys.len(), merge_count);
            }
        }
        
        pb.finish_with_message("Merging complete");
        println!("\nApplied {} initial merges", merge_count);
            
        // Apply transitive closure unless fast mode is enabled
        let mut identity_groups: HashMap<usize, Vec<usize>> = HashMap::new();

        let use_streaming = entries.len() > 1_000_000; // Use streaming for files > 1M entries

        if !args.fast_mode {
            println!("Building identity groups with transitive closure...");
            let closure_start = Instant::now();
            
            if use_streaming {
                println!("Large file detected - applying memory-efficient identity merging");
                
                // Apply the same transitive closure logic but more memory efficiently
                let max_occurrences = if args.max_frequency > 0.0 {
                    (args.max_frequency / 100.0 * line_count as f64) as usize
                } else {
                    1000
                };
                
                // Step 1: Build value frequency map in chunks to identify qualifying values
                println!("Step 1: Analyzing value frequencies for qualifying values...");
                let mut value_frequency: HashMap<String, usize> = HashMap::new();
                
                const CHUNK_SIZE: usize = 100_000;
                for chunk_start in (0..entries.len()).step_by(CHUNK_SIZE) {
                    let chunk_end = (chunk_start + CHUNK_SIZE).min(entries.len());
                    let chunk = &entries[chunk_start..chunk_end];
                    
                    for entry in chunk {
                        if let Some(ref types_str) = args.merge_types {
                            let allowed_types: HashSet<&str> = types_str.split(',').map(|s| s.trim()).collect();
                            if !allowed_types.contains(entry.entry_type.as_str()) {
                                continue;
                            }
                        }
                        
                        let key = format!("{}:{}", entry.entry_type, entry.value);
                        *value_frequency.entry(key).or_default() += 1;
                    }
                    
                    if chunk_start % 1_000_000 == 0 {
                        println!("Processed {} entries for frequency analysis", chunk_start);
                    }
                }
                
                // Filter qualifying values
                let qualifying_values: HashSet<String> = value_frequency
                    .iter()
                    .filter(|(_, &count)| count > 1 && count <= max_occurrences)
                    .map(|(key, _)| key.clone())
                    .collect();
                
                println!("Using {} qualifying values for identity merging", qualifying_values.len());
                drop(value_frequency); // Free memory
                
                // Step 2: Build identity mapping using the same logic as smaller files
                println!("Step 2: Building identity mapping...");
                let mut value_to_identity: HashMap<String, usize> = HashMap::new();
                let mut identity_counter = 0;
                let mut line_to_identity: HashMap<u32, usize> = HashMap::new();
                
                let pb = ProgressBar::new(entries.len() as u64);
                pb.set_message("Building identities...");
                
                for chunk_start in (0..entries.len()).step_by(CHUNK_SIZE) {
                    let chunk_end = (chunk_start + CHUNK_SIZE).min(entries.len());
                    let chunk = &entries[chunk_start..chunk_end];
                    
                    for entry in chunk {
                        if let Some(ref types_str) = args.merge_types {
                            let allowed_types: HashSet<&str> = types_str.split(',').map(|s| s.trim()).collect();
                            if !allowed_types.contains(entry.entry_type.as_str()) {
                                pb.inc(1);
                                continue;
                            }
                        }
                        
                        let key = format!("{}:{}", entry.entry_type, entry.value);
                        
                        if !qualifying_values.contains(&key) {
                            pb.inc(1);
                            continue;
                        }
                        
                        // Same identity logic as smaller files
                        if let Some(&existing_identity) = value_to_identity.get(&key) {
                            line_to_identity.insert(entry.line, existing_identity);
                        } else {
                            if let Some(&existing_identity) = line_to_identity.get(&entry.line) {
                                value_to_identity.insert(key, existing_identity);
                            } else {
                                let new_identity = identity_counter;
                                identity_counter += 1;
                                value_to_identity.insert(key, new_identity);
                                line_to_identity.insert(entry.line, new_identity);
                            }
                        }
                        
                        pb.inc(1);
                    }
                    
                    if chunk_start % 1_000_000 == 0 {
                        pb.set_message(format!("Processed {} entries, {} identities", chunk_start, identity_counter));
                    }
                }
                
                pb.finish_with_message(format!("Processed {} entries, created {} base identities", entries.len(), identity_counter));
                
                // Step 3: Build final identity groups
                println!("Step 3: Building final identity groups...");
                for (idx, entry) in entries.iter().enumerate() {
                    if let Some(&identity) = line_to_identity.get(&entry.line) {
                        identity_groups.entry(identity).or_default().push(idx);
                    }
                    
                    if idx % 1_000_000 == 0 {
                        println!("Processed {} entries for group building", idx);
                    }
                }
                
                println!("Built {} final identity groups using memory-efficient approach in {:.2}s", 
                         identity_groups.len(), closure_start.elapsed().as_secs_f64());
                
            } else {
                // Keep your existing working code for smaller files
                println!("Applying transitive closure to merge identities sharing values...");
                
                let max_occurrences = if args.max_frequency > 0.0 {
                    (args.max_frequency / 100.0 * line_count as f64) as usize
                } else {
                    1000
                };
                
                // Process entries in smaller chunks to save memory
                const CHUNK_SIZE: usize = 100_000;
                
                // First pass: build value frequency map in chunks
                let mut value_frequency: HashMap<String, usize> = HashMap::new();
                
                for chunk_start in (0..entries.len()).step_by(CHUNK_SIZE) {
                    let chunk_end = (chunk_start + CHUNK_SIZE).min(entries.len());
                    let chunk = &entries[chunk_start..chunk_end];
                    
                    for entry in chunk {
                        if let Some(ref types_str) = args.merge_types {
                            let allowed_types: HashSet<&str> = types_str.split(',').map(|s| s.trim()).collect();
                            if !allowed_types.contains(entry.entry_type.as_str()) {
                                continue;
                            }
                        }
                        
                        let key = format!("{}:{}", entry.entry_type, entry.value);
                        *value_frequency.entry(key).or_default() += 1;
                    }
                }
                
                let qualifying_values: HashSet<String> = value_frequency
                    .iter()
                    .filter(|(_, &count)| count > 1 && count <= max_occurrences)
                    .map(|(key, _)| key.clone())
                    .collect();
                
                println!("Using {} qualifying values for identity merging", qualifying_values.len());
                drop(value_frequency);
                
                let mut value_to_identity: HashMap<String, usize> = HashMap::new();
                let mut identity_counter = 0;
                let mut line_to_identity: HashMap<u32, usize> = HashMap::new();
                
                let pb = ProgressBar::new(entries.len() as u64);
                pb.set_message("Building identities...");
                
                for chunk_start in (0..entries.len()).step_by(CHUNK_SIZE) {
                    let chunk_end = (chunk_start + CHUNK_SIZE).min(entries.len());
                    let chunk = &entries[chunk_start..chunk_end];
                    
                    for (relative_idx, entry) in chunk.iter().enumerate() {
                        if let Some(ref types_str) = args.merge_types {
                            let allowed_types: HashSet<&str> = types_str.split(',').map(|s| s.trim()).collect();
                            if !allowed_types.contains(entry.entry_type.as_str()) {
                                pb.inc(1);
                                continue;
                            }
                        }
                        
                        let key = format!("{}:{}", entry.entry_type, entry.value);
                        
                        if !qualifying_values.contains(&key) {
                            pb.inc(1);
                            continue;
                        }
                        
                        if let Some(&existing_identity) = value_to_identity.get(&key) {
                            line_to_identity.insert(entry.line, existing_identity);
                        } else {
                            if let Some(&existing_identity) = line_to_identity.get(&entry.line) {
                                value_to_identity.insert(key, existing_identity);
                            } else {
                                let new_identity = identity_counter;
                                identity_counter += 1;
                                value_to_identity.insert(key, new_identity);
                                line_to_identity.insert(entry.line, new_identity);
                            }
                        }
                        
                        pb.inc(1);
                    }
                }
                
                pb.finish_with_message(format!("Processed {} entries, created {} base identities", entries.len(), identity_counter));
                
                // Build final identity groups
                let mut final_groups: HashMap<usize, Vec<usize>> = HashMap::new();
                
                for (idx, entry) in entries.iter().enumerate() {
                    if let Some(&identity) = line_to_identity.get(&entry.line) {
                        final_groups.entry(identity).or_default().push(idx);
                    }
                }
                
                println!("Built {} final identity groups with transitive closure in {:.2}s", 
                         final_groups.len(), closure_start.elapsed().as_secs_f64());
                identity_groups = final_groups;
            }
        } else {
            // Fast mode code remains the same
            println!("Fast mode enabled - skipping transitive closure step");
            
            let build_start = Instant::now();
            println!("Building identity groups (fast mode)...");
            
            let line_to_root: HashMap<usize, usize> = (0..disjoint_set.size)
                .map(|i| (i, disjoint_set.find(i)))
                .collect();
            
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

        println!("Total number of identity groups: {}", identity_groups.len());
        
        // Map roots to sequential identity numbers
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