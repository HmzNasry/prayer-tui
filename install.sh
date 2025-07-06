#!/bin/bash

# Build the application
cargo build --release

# Create a symbolic link
sudo ln -sf $(pwd)/target/release/prayer-tui /usr/local/bin/pt

# Create the systemd service file
sudo cp prayer-tui.service /etc/systemd/system/

# Enable and start the service
sudo systemctl enable --now prayer-tui.service

echo "Prayer TUI has been installed and started."

