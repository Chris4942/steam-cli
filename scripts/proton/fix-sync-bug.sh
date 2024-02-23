#!/bin/sh

if [ "$#" -lt 2 ]
then
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

cp ~/.cache/fix-dllr-bug/dlls/System32/ucrtbase.dll "$GAME_DIRECTORY/pfx/drive_c/windows/system32/ucrtbase.dll"
cp ~/.cache/fix-dllr-bug/dlls/SysWOW64/ucrtbase.dll "$GAME_DIRECTORY/pfx/drive_c/windows/syswow64/ucrtbase.dll"

rm -rf ~/.cache/fix-dllr-bug
