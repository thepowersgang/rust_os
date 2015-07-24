#!/bin/sh

for dir in $(find -name .git | sed 's/.git$//'); do
	echo "--- $dir"
	(cd $dir; git pull origin master)
done
