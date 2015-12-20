#!/bin/sh

find Kernel/ *.rs Usermode/ -name \*.rs ! -path */target/* -print0 | wc -l --files0-from=- | sort -n
