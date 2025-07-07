#!/bin/bash

# Build the application
cargo build --release

# Create a symbolic link
sudo ln -sf $(pwd)/target/release/prayer-tui /usr/local/bin/pt

# Create the systemd user service directory if it doesn't exist
mkdir -p ~/.config/systemd/user/

# Create the systemd user service file
cp prayer-tui.service ~/.config/systemd/user/

# Enable and start the user service
systemctl --user daemon-reload
systemctl --user enable --now prayer-tui.service

echo "Prayer TUI has been installed and started."

