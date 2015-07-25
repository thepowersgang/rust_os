#!/bin/bash

DSM=Kernel/bin/kernel-amd64.bin.dsm
echo "- Disassembling kernel"
objdump -S Kernel/bin/kernel-amd64.bin > $DSM
echo "- Annotated backtrace"
BT=$(echo $1 | sed 's/.*: //' | sed 's/ > / /g' | sed 's/0x//g')
for addr in $BT; do
	echo "-- $addr"
	grep '>:$\|'$addr':' $DSM | grep $addr -B 1
done


