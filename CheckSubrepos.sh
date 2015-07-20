#!/bin/sh

for dir in $(find -name .git | sed 's/.git$//'); do
	echo "--- $dir"
	(cd $dir; git status -bs)
done
