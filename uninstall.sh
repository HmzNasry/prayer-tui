#!/bin/bash

# Stop and disable the service
sudo systemctl disable --now prayer-tui.service

# Remove the systemd service file
sudo rm /etc/systemd/system/prayer-tui.service

# Remove the symbolic link
sudo rm /usr/local/bin/pt

# Remove the configuration directory
rm -rf ~/.config/prayer-tui

echo "Prayer TUI has been uninstalled."

