#!/bin/sh
pacman-key --init
pacman-key --populate archlinux
sudo pacman -S holo-keyring archlinux-keyring
# If after this it doesn't work
# You may need to go into /etc/pacman.conf
# and update [options].SigLevel to TrustAll
# Then run pt of this script
