use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::time::Instant;
use std::sync::{Arc, Mutex};

use clap::Parser;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use indicatif::{ProgressBar, ProgressStyle};

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
        
        // Create a mapping from line numbers to sequential indices for the DisjointSet
        let mut all_lines: Vec<u32> = entries.iter().map(|e| e.line).collect();
        all_lines.sort_unstable();
        all_lines.dedup();
        
        let line_to_index: HashMap<u32, usize> = all_lines.iter()
            .enumerate()
            .map(|(idx, &line)| (line, idx))
            .collect();
        
        let index_to_line: Vec<u32> = all_lines;
        
        if debug {
            println!("Found {} unique line numbers", line_to_index.len());
        }
        
        // Step 1: Initial grouping by line number using more efficient HashMap
        let mut line_groups: HashMap<u32, Vec<usize>> = HashMap::with_capacity(line_to_index.len());
        for (i, entry) in entries.iter().enumerate() {
            line_groups.entry(entry.line).or_default().push(i);
        }
        
        // Step 2: Build a map of values to line numbers and analyze their frequency
        println!("Analyzing value frequency across lines...");
        
        // Parse types filter if provided
        let merge_types: Option<HashSet<String>> = args.merge_types.map(|types_str| {
            types_str.split(',').map(|s| s.trim().to_string()).collect()
        });
        
        // Store value frequency statistics
        let mut value_to_lines: HashMap<String, Vec<usize>> = HashMap::new();
        let mut value_frequency: HashMap<String, usize> = HashMap::new();
        
        for entry in &entries {
            // Skip if entry type isn't in the merge_types list (if provided)
            if let Some(ref types) = merge_types {
                if !types.contains(&entry.entry_type) {
                    continue;
                }
            }
            
            let key = format!("{}:{}", entry.entry_type, entry.value);
            let index = line_to_index[&entry.line];
            
            // Add to value_to_lines mapping
            value_to_lines.entry(key.clone()).or_default().push(index);
            
            // Increment frequency counter
            *value_frequency.entry(key).or_insert(0) += 1;
        }
        
        // Calculate max occurrences based on max frequency percentage
        let max_occurrences = (args.max_frequency / 100.0 * line_to_index.len() as f64) as usize;
        println!("Values appearing in more than {} lines ({:.1}%) will be excluded from identity merging", 
            max_occurrences, args.max_frequency);
        
        // Show most common values
        let mut value_freq_vec: Vec<(String, usize)> = value_frequency.iter()
            .map(|(k, &v)| (k.clone(), v))
            .collect();
        
        value_freq_vec.sort_by(|a, b| b.1.cmp(&a.1));
        
        println!("\nTop 10 most common values:");
        for (i, (value, count)) in value_freq_vec.iter().take(10).enumerate() {
            println!("{}. {} - appears in {} lines ({:.2}%)", 
                i+1, value, count, (*count as f64 / line_to_index.len() as f64) * 100.0);
        }
        
        if args.analyze {
            println!("\nAnalysis complete. Use --types and --max-freq to control identity merging.");
            return Ok(());
        }
        
        // Filter out overly common values
        value_to_lines.retain(|key, lines| {
            // Keep values that appear in at least 2 lines but fewer than max_occurrences
            lines.len() >= 2 && lines.len() <= max_occurrences
        });
        
        println!("\nUsing {} values for identity merging", value_to_lines.len());
        
        // Step 3: Merge identities using Union-Find
        let union_start = Instant::now();
        println!("Merging identities...");
        
        let pb = ProgressBar::new(value_to_lines.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} values processed ({eta})")
            .expect("Failed to parse template")
            .progress_chars("#>-"));
        
        let mut disjoint_set = DisjointSet::new(line_to_index.len());
        let mut merge_count = 0;
        
        // First pass: Build a mapping of line indices to the values they contain
        let mut line_to_values: HashMap<usize, HashSet<String>> = HashMap::new();
        for (value, line_indices) in &value_to_lines {
            for &line_idx in line_indices {
                line_to_values.entry(line_idx).or_default().insert(value.clone());
            }
        }
        
        // Second pass: For each value, merge all lines that contain it
        for (value, line_indices) in value_to_lines.iter() {
            if line_indices.len() >= 2 {
                // Merge all pairs of lines that share this value
                for &line_a in line_indices {
                    for &line_b in line_indices {
                        if line_a != line_b && disjoint_set.union(line_a, line_b) {
                            merge_count += 1;
                            if debug {
                                println!("Merged lines {} and {} due to shared value {}", 
                                    index_to_line[line_a], index_to_line[line_b], value);
                            }
                        }
                    }
                }
            }
            pb.inc(1);
        }
        
        // Third pass: Transitive closure - use a true multi-threaded approach
        println!("Applying transitive closure to identity groups...");
        let mut transitive_merges = 0;
        let mut iterations = 0;

        // Configure thread pool for optimal performance
        let num_threads = num_cpus::get();
        println!("Using {} CPU threads for parallel processing", num_threads);

        // Build value-to-root mapping for faster lookups - use more efficient data structure
        let mut value_to_roots: HashMap<String, Vec<usize>> = HashMap::new();
        for (value, _) in &value_to_lines {
            value_to_roots.insert(value.clone(), Vec::new());
        }

        loop {
            let iteration_start = Instant::now();
            iterations += 1;
            
            // Compute all roots in parallel batches
            println!("  Computing current identity roots...");
            
            let chunks: Vec<_> = (0..disjoint_set.size).collect::<Vec<_>>()
                .chunks(disjoint_set.size / num_threads + 1)
                .map(|c| c.to_vec())
                .collect();
            
            let line_roots_chunks: Vec<_> = chunks.par_iter().map(|chunk| {
                let mut local_roots = Vec::with_capacity(chunk.len());
                let mut local_disjoint_set = DisjointSet {
                    parent: disjoint_set.parent.clone(),
                    rank: disjoint_set.rank.clone(),
                    size: disjoint_set.size,
                };
                
                for &i in chunk {
                    let root = local_disjoint_set.find(i);
                    local_roots.push((i, root));
                }
                
                local_roots
            }).collect();
            
            // Merge results and update the main DisjointSet
            let mut line_roots = Vec::with_capacity(disjoint_set.size);
            line_roots.resize(disjoint_set.size, 0);
            
            for chunk in line_roots_chunks {
                for (i, root) in chunk {
                    line_roots[i] = root;
                }
            }
            
            // Update roots set and value-to-roots map in parallel
            let mut roots = HashSet::new();
            let values_chunk_size = value_to_lines.len() / num_threads + 1;
            
            // First collect all roots
            for &root in &line_roots {
                roots.insert(root);
            }
            
            // Clear previous mappings
            for roots_vec in value_to_roots.values_mut() {
                roots_vec.clear();
            }
            
            // Update value-to-roots mapping in parallel
            let value_chunks: Vec<_> = line_to_values.par_iter()
                .map(|(&i, values)| {
                    let root = line_roots[i];
                    let mut local_map = HashMap::new();
                    
                    for value in values {
                        local_map.entry(value.clone())
                            .or_insert_with(Vec::new)
                            .push(root);
                    }
                    
                    local_map
                })
                .collect();
            
            // Merge value-to-roots maps
            let mutex_map = Arc::new(Mutex::new(&mut value_to_roots));
            value_chunks.into_par_iter().for_each(|local_map| {
                let mut lock = mutex_map.lock().unwrap();
                for (value, roots) in local_map {
                    if let Some(global_roots) = lock.get_mut(&value) {
                        global_roots.extend(roots);
                    }
                }
            });
            
            // Deduplicate roots in value_to_roots
            for roots_vec in value_to_roots.values_mut() {
                roots_vec.sort_unstable();
                roots_vec.dedup();
            }
            
            println!("  Current number of distinct identity groups: {}", roots.len());
            
            // Find values shared across multiple roots
            let shared_values: Vec<String> = value_to_roots.par_iter()
                .filter_map(|(value, roots_vec)| {
                    if roots_vec.len() >= 2 {
                        Some(value.clone())
                    } else {
                        None
                    }
                })
                .collect();
            
            println!("  Found {} values shared across different identity groups", shared_values.len());
            
            if shared_values.is_empty() {
                println!("  No more shared values found, stopping iterations");
                break;
            }
            
            // Compute merge operations in parallel
            let merge_ops: HashSet<(usize, usize)> = shared_values.par_iter()
                .flat_map(|value| {
                    let roots_vec = &value_to_roots[value];
                    let mut pairs = HashSet::new();
                    
                    if roots_vec.len() >= 2 {
                        for i in 0..roots_vec.len() {
                            for j in i+1..roots_vec.len() {
                                pairs.insert((roots_vec[i].min(roots_vec[j]), 
                                            roots_vec[i].max(roots_vec[j])));
                            }
                        }
                    }
                    
                    pairs.into_iter().collect::<Vec<_>>()
                })
                .collect();
            
            println!("  Generated {} potential merge operations", merge_ops.len());
            
            // Apply merges in batches with periodic sync
            let mut merged_this_round = 0;
            let batch_size = 10000;
            
            for batch in merge_ops.into_iter().collect::<Vec<_>>().chunks(batch_size) {
                let batch_merges = batch.par_iter()
                    .map(|&(root_a, root_b)| {
                        let mut local_disjoint_set = DisjointSet {
                            parent: disjoint_set.parent.clone(),
                            rank: disjoint_set.rank.clone(),
                            size: disjoint_set.size,
                        };
                        
                        let merged = local_disjoint_set.union(root_a, root_b);
                        if merged {
                            Some((root_a, root_b, local_disjoint_set.parent[root_b]))
                        } else {
                            None
                        }
                    })
                    .filter_map(|x| x)
                    .collect::<Vec<_>>();
                
                // Apply successful merges to main disjoint set
                for (root_a, root_b, _) in batch_merges {
                    if disjoint_set.union(root_a, root_b) {
                        merged_this_round += 1;
                        transitive_merges += 1;
                    }
                }
            }
            
            println!("  Iteration {}: merged {} identity groups in {:.2}s", 
                iterations, merged_this_round, iteration_start.elapsed().as_secs_f64());
            
            // Stop if no more merges or too many iterations
            if merged_this_round == 0 || iterations >= 5 {
                break;
            }
        }

        println!("Applied {} transitive merges in {} iterations", transitive_merges, iterations);
        pb.finish_with_message("Identity merging complete");

        println!("Merged {} identities (plus {} transitive merges) in {:.2}s", 
            merge_count, transitive_merges, union_start.elapsed().as_secs_f64());
        // Step 4: Build merged identities
        let build_start = Instant::now();
        println!("Building identity groups...");
        
        // Map each line to its root identity
        let mut line_to_root: HashMap<usize, usize> = HashMap::with_capacity(line_to_index.len());
        for i in 0..line_to_index.len() {
            let root = disjoint_set.find(i);
            line_to_root.insert(i, root);
        }
        
        // Group entries by identity root
        let mut identity_groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for (idx, entry) in entries.iter().enumerate() {
            let line_idx = line_to_index[&entry.line];
            let root = line_to_root[&line_idx];
            identity_groups.entry(root).or_default().push(idx);
        }
        
        println!("Built {} identity groups in {:.2}s", identity_groups.len(), build_start.elapsed().as_secs_f64());
        
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
            println!("{}", json);
            
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