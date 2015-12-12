#!/bin/bash

BACKTRACE=$1
if [[ $# -ge 2 ]]; then
	DSM=$2
else
	BINFILE=Kernel/bin/kernel-amd64.bin
	echo "- Disassembling kernel"
	objdump -S Kernel/bin/kernel-amd64.bin > ${BINFILE}.dsm
	DSM=${BINFILE}.dsm
fi

echo "- Annotated backtrace"
BT=$(echo ${BACKTRACE} | sed 's/.*: //' | sed 's/ > / /g' | sed 's/0x//g')
for addr in $BT; do
	echo "-- $addr"
	grep '>:$\|^ *'$addr':' $DSM | grep '^ *'$addr -B 1
done


