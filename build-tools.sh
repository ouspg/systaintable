#!/bin/bash
# filepath: /Users/kasperkyllonen/Desktop/systaintable/build-tools.sh

set -e

echo "Building all tools..."

# Build classifier
echo "Building classifier..."
cd classifier
cargo build --release
cd ..

# Build idtrace (if it exists)
if [ -d "idtrace" ]; then
    echo "Building idtrace..."
    cd idtrace
    cargo build --release
    cd ..
else
    echo "Idtrace directory not found, skipping..."
fi

# Setup mermetro2 (if it exists)
if [ -d "mermetro2" ]; then
    echo "Setting up mermetro2..."
    cd mermetro2
    if [ -f "requirements.txt" ]; then
        pip3 install --break-system-packages -r requirements.txt
    fi
    cd ..
else
    echo "Mermetro2 directory not found, skipping..."
fi

# Make scripts executable
chmod +x logprocess.sh 2>/dev/null || echo "logprocess.sh not found"
chmod +x build-tools.sh 2>/dev/null || echo "build-tools.sh already executable"

echo "All tools built successfully!"

# Show what was built
echo "Available binaries:"
find . -name "target" -type d -exec find {} -name "*.exe" -o -name "classifier" -o -name "regex-classifier" -o -name "idtrace" \; 2>/dev/null || echo "No binaries found yet"