# Corpus downloader ia a script that downloads public corpuses and creates logbook entries.

Corpus downloader is a script for downloading public corpuses and creating according logbook entries. You can find spec from corpusdownloader_spec.txt.

# Corpus downloader Guide

This guide explains how to use the corpus downloader script to download log files from the LogPAI LogHub repository and generate logbook entries for each corpus.

## Description

The `corpusdownloader.py` script:
1. Downloads log files from the [LogPAI LogHub repository](https://github.com/logpai/loghub)
2. Organizes them by system in a test data folder
3. Creates structured logbook entries in JSON format for each downloaded log file
4. Records metadata including file size, event count, and timestamps

## Requirements

- Python 3.6 or higher
- Required Python packages:
  - requests
  - tqdm (for progress bars)

## Installation

1. Clone this repository or download the `corpusdownloader.py` script

2. Install the required Python packages:

```bash
pip install requests tqdm
````
