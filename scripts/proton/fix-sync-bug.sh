#!/bin/sh

# first argument is a path to a zip folder containing the source dll files. The expected internal structure is
# dlls/
# - System32/ucrtbase.dll
# - SysWOW64/ucrtbase.dll
# the second argument is the path to the game folder, so for Halo on a steam deck it would be:
# /home/deck/.local/share/Steam/steamapps/compatdata/976730
# At least that's what it is on my steam deck
# So if the zip is called windows-dlls.zip in your downloads folder, you would run:
# ./scripts/proton/fix-sync-bug.sh scripts/proton/dlls.zip /home/deck/.local/share/Steam/steamapps/compatdata/976730

if [ "$#" -lt 2 ]; then
	echo "two arguments required"
	exit 1
fi

ZIP=$1
GAME_DIRECTORY=$2

echo $ZIP
echo $GAME_DIRECTORY

unzip $ZIP -d ~/.cache/fix-dllr-bug

rm "$GAME_DIRECTORY/pfx/drive_c/windows/system32/ucrtbase.dll"
rm "$GAME_DIRECTORY/pfx/drive_c/windows/syswow64/ucrtbase.dll"

cp ~/.cache/fix-dllr-bug/System32/ucrtbase.dll "$GAME_DIRECTORY/pfx/drive_c/windows/system32/ucrtbase.dll"
cp ~/.cache/fix-dllr-bug/SysWOW64/ucrtbase.dll "$GAME_DIRECTORY/pfx/drive_c/windows/syswow64/ucrtbase.dll"

rm -rf ~/.cache/fix-dllr-bug
