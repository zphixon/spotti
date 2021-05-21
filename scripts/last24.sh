#!/bin/bash

time=''

if [ -z "$@" ]; then
	time='-24 hours'
else
	time="$@"
fi

sqlite3 ../recents.db "select * from songs where date > datetime('now', '$time');"
