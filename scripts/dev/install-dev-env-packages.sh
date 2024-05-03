#!/bin/sh
# installs various packages that I like to include on my steam deck.
# Whenever updates occur, I have to reinstall these, so I want to keep them all together somewhere.

sudo pacman -Sgq nerd-fonts

sudo pacman -S code neovim tff-nerd-fonts-symbols-common
