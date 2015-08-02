#!/bin/sh

grep 'unsafe *[{$]' -nrI Kernel/Core/ Kernel/Modules/ -B 1 | awk 'BEGIN {issafe = 0;} { if(match($0, /SAFE:/) != 0) { issafe = 1 } else { if(issafe == 0) {print $0} else {}; issafe=0; } }' | uniq
#grep '[^(//)]*unsafe [^(fn)]' -nrI Kernel/Core/ Kernel/Modules/ -B 1 | awk 'BEGIN {issafe = 0;} { if(match($0, /SAFE:/) != 0) { issafe = 1 } else { if(issafe == 0) {print $0} else {}; issafe=0; } }' | uniq
