#!/bin/bash

grep --include=*.rs 'unsafe *[{$]' -nrI Kernel/Core/ Kernel/Modules/ Usermode/ -B 1 \
	| awk 'BEGIN{print "--"; issafe = 0; count=0;} { if(match($0, /SAFE:/) != 0) {issafe = 1} else { if(issafe == 0) { if(match($0, /unsafe/)) {count += 1;} print $0} else {}; issafe=0; } } END{ exit (count==0?0:1); }' \
	| uniq | tail -n +2
exit ${PIPESTATUS[1]}
#grep '[^(//)]*unsafe [^(fn)]' -nrI Kernel/Core/ Kernel/Modules/ -B 1 | awk 'BEGIN {issafe = 0;} { if(match($0, /SAFE:/) != 0) { issafe = 1 } else { if(issafe == 0) {print $0} else {}; issafe=0; } }' | uniq
