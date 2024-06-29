#!/bin/sh
pacman-key --init
pacman-key --populate archlinux

# If after this it doesn't work
# You may need to go into /etc/pacman.conf
# and update [options].SigLevel to TrustAll
# Then run pt of this script
