# Systaintable

Systaintable is a OUSPG research project aiming to discover potential vulnerabilities from system level event log and traces through input propagation analysis. This Systaintable project repository includes following tooling for doing the research project:

* OUSPG Logbook
    * tracking event trace material collection, facts, processing and removal over its whole life-cycle (“chain of custody”)
* OUSPG ID.ID
    * Identifying and classifying identities (natural or artificial) from the event trace material
    normalization of the event traces -> isolating the variables -> identifying identities from the variable values / classifying the identities into identity categories
* OUSPG ID.ALIAS
    * Identifying aliases of the same underlying “entities” from the identity values
    span over different abstraction levels (mac, ip, dns, url, …) (name, userid, public key,…)
* OUSPG ID.TRACE
    * getting vectors of ID occurrences (extracting the traces) (@ time, locus, component, …)
* OUSPG Mermetro
    * Visualization of data from OUSPG ID.ID's or ID.Trace's output. Creates an interactive metro map from standardized log file accessible from your own browser. Currently has two separate tools, both still dependent on each other. See [README.md](mermetro/README.md) for more information.

Additionally this project repository contains guides how to use other tools for doing system level taint analysis.

## Getting started

1. **Clone the repository**
    ```bash
   git clone git@github.com:ouspg/systaintable.git
   cd systaintable
   ```

2. **Open in Dev Container**
    * Open VS Code in the project directory ```code .```
    * When prompted, click "Reopen in Container"
    * Or use Command Palette: ```Dev Containers: Reopen in Container```

## Basic usage

1. **Prepare your data**

    * Create a data directory and add your log files:

    ```bash
    mkdir -p data
    cp /path/to/your/*.log data/
    ```
2. **Run the Analysis Pipeline**

    * You can use logprocess.sh to run the analysis pipeline

    * Basic usage - search for an identifier in logs
    ```bash
    ./logprocess.sh "192.168.1.1"
    ```


3. ## Example Workflow
