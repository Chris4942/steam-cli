#!/bin/sh

sudo pacman -S gcc mingw-w64-gcc openssl pkgconfig linux-api-headers npm

# You'll probably also need to something like this:
# pacman -S "$package" --overwrite "*" --noconfirm
# on glibc
