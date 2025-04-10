#!/usr/bin/env python3
# filepath: /Users/kasperkyllonen/Desktop/systaintable/corpusdownloader/corpusdownloader.py

import os
import json
import requests
import argparse
import time
from datetime import datetime
from tqdm import tqdm

def create_directory(directory):
    """Create directory if it doesn't exist"""
    if not os.path.exists(directory):
        os.makedirs(directory)

def download_file(url, destination):
    """Download a file from URL to destination with progress bar"""
    response = requests.get(url, stream=True)
    total_size = int(response.headers.get('content-length', 0))
    
    with open(destination, 'wb') as f, tqdm(
        desc=os.path.basename(destination),
        total=total_size,
        unit='B',
        unit_scale=True,
        unit_divisor=1024,
    ) as progress_bar:
        for data in response.iter_content(chunk_size=1024):
            size = f.write(data)
            progress_bar.update(size)
            
    return destination

def get_system_directories():
    """Get list of system directories from the LogHub repository"""
    response = requests.get("https://api.github.com/repos/logpai/loghub/contents/")
    if response.status_code != 200:
        print(f"Failed to fetch repository contents: {response.status_code}")
        return []
    
    contents = response.json()
    directories = []
    
    for item in contents:
        # Only include directories and exclude hidden ones
        if item['type'] == 'dir' and not item['name'].startswith('.'):
            directories.append(item['name'])
    
    return directories

def get_log_files(system_dir):
    """Get list of log files in a system directory"""
    response = requests.get(f"https://api.github.com/repos/logpai/loghub/contents/{system_dir}")
    if response.status_code != 200:
        print(f"Failed to fetch directory contents for {system_dir}: {response.status_code}")
        return []
    
    contents = response.json()
    log_files = []
    
    for item in contents:
        if item['type'] == 'file' and item['name'].endswith('.log'):
            log_files.append({
                'name': item['name'],
                'download_url': item['download_url'],
                'html_url': item['html_url']
            })
    
    return log_files

def get_file_size(filepath):
    """Get file size in bytes"""
    return os.path.getsize(filepath)

def count_events(filepath):
    """Count number of log events (lines) in file"""
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            return sum(1 for _ in f)
    except:
        print(f"Warning: Could not count events in {filepath}, using estimate")
        # If we can't read the file as text, estimate based on file size with average line length
        return get_file_size(filepath) // 100  # Rough estimate

def create_logbook_entry(system_name, log_file, file_path, source_url, username):
    """Create logbook entry JSON for the downloaded log file"""
    now = datetime.now().isoformat()
    file_size = get_file_size(file_path)
    event_count = count_events(file_path)
    
    # Create a logbook entry following the schema
    logbook_entry = {
        "entry": {
            "timestamp": now,
            "author": username,
            "processing type": "collection",
            "processing description": f"Automated download of {system_name} log corpus from LogHub repository"
        },
        "data facts": {
            "description": f"Log data from {system_name} system, file: {log_file}",
            "storage": {
                "source location": source_url,
                "location": file_path,
                "location other": "",
                "retention": {
                    "deadline": "",
                    "removal policy": "delete"
                }
            },
            "metrics": {
                "start time": "",
                "end time": "",
                "collection time": now,
                "size": file_size,
                "event count": event_count,
                "other metrics": []
            },
            "rights": {
                "license": "other",
                "other license": "Original repository license applies",
                "owner": {
                    "owner name": "LogPAI",
                    "contact name": "",
                    "contact email": "",
                    "contact phone": "",
                    "contact other": "https://github.com/logpai/loghub",
                    "citation": "Please cite the LogHub paper if used in academic research"
                }
            },
            "PII": {
                "sanitation": "pseudonymized",
                "may contain": ["system identifiers", "user identifiers", "IP addresses"],
                "may contain other": ["Log data might contain sensitive information"],
                "confirmed to contain": [],
                "confirmed to contain other": []
            }
        }
    }
    
    return logbook_entry

def main():
    parser = argparse.ArgumentParser(description='Download log files from LogHub repository and create logbook entries')
    parser.add_argument('--output', default='./test_data', help='Output directory for downloaded files')
    parser.add_argument('--logbook', default='./logbook', help='Output directory for logbook entries')
    parser.add_argument('--username', default=os.getenv('USER', 'anonymous'), help='Your name for the logbook entries')
    parser.add_argument('--systems', nargs='+', help='Specific systems to download (default: all)')
    args = parser.parse_args()
    
    output_dir = args.output
    logbook_dir = args.logbook
    username = args.username
    
    create_directory(output_dir)
    create_directory(logbook_dir)
    
    print("Fetching system directories from LogHub repository...")
    system_dirs = get_system_directories()
    
    if not system_dirs:
        print("No directories found or error occurred.")
        return
    
    # Filter systems if specified
    if args.systems:
        system_dirs = [d for d in system_dirs if d in args.systems]
        if not system_dirs:
            print("None of the specified systems were found.")
            return
    
    print(f"Found {len(system_dirs)} system directories to process.")
    
    total_files = 0
    
    for system in system_dirs:
        system_output_dir = os.path.join(output_dir, system)
        system_logbook_dir = os.path.join(logbook_dir, system)
        
        create_directory(system_output_dir)
        create_directory(system_logbook_dir)
        
        print(f"\nScanning {system} for log files...")
        log_files = get_log_files(system)
        
        if not log_files:
            print(f"No log files found in {system}")
            continue
        
        print(f"Found {len(log_files)} log files in {system}")
        total_files += len(log_files)
        
        for log_file_info in log_files:
            file_name = log_file_info['name']
            download_url = log_file_info['download_url']
            source_url = log_file_info['html_url']
            destination = os.path.join(system_output_dir, file_name)
            
            print(f"Downloading {file_name}...")
            download_file(download_url, destination)
            
            # Create logbook entry JSON
            logbook_entry = create_logbook_entry(
                system_name=system, 
                log_file=file_name, 
                file_path=destination, 
                source_url=source_url, 
                username=username
            )
            
            logbook_file = os.path.join(system_logbook_dir, f"{os.path.splitext(file_name)[0]}_logbook.json")
            
            with open(logbook_file, 'w', encoding='utf-8') as f:
                json.dump(logbook_entry, f, indent=2)
                print(f"Created logbook entry: {logbook_file}")
            
            # Be nice to GitHub API - add a small delay between requests
            time.sleep(0.5)
    
    print(f"\nDownload complete! Downloaded {total_files} log files to {output_dir}")
    print(f"Created {total_files} logbook entry files in {logbook_dir}")

if __name__ == "__main__":
    main()