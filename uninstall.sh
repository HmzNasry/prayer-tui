#!/bin/bash

# Stop and disable the user service
systemctl --user disable --now prayer-tui.service

# Remove the user service file
rm ~/.config/systemd/user/prayer-tui.service

# Remove the symbolic link
sudo rm /usr/local/bin/pt

# Remove the configuration directory
rm -rf ~/.config/prayer-tui

echo "Prayer TUI has been uninstalled."

