#!/bin/bash

printhelp() {
	echo "-r --reinstall       reinstall the services"
	echo "-h --help            show this message"
	echo "-b --bot-only        only change the bot service"
	echo "-s --server-only     only change the server service"
}

if [ `id -u` != "0" ]; then
	echo "please run as root"
	exit 1
fi

restartbot=0
restartserver=0
reinstall=0

case "$1" in
	"-s"|"--server-only")
		restartserver=1
		;;
	"-b"|"--bot-only")
		restartbot=1
		;;
	"-r"|"--reinstall")
		reinstall=1
		;;
	"-h"|"--help")
		printhelp
		exit 0
		;;
	"")
		restartbot=1
		restartserver=1
		;;
	*)
		printhelp
		exit 1
		;;
esac

if [ "$restartserver" ]; then
	systemctl stop spotti
fi

if [ "$restartbot" ]; then
	systemctl stop spotti-downbot
fi

if [ "$reinstall" ]; then
	cp spotti-downbot.service spotti.service /etc/systemd/system/
fi

if [ "$restartserver" ]; then
	systemctl start spotti
	systemctl enable spotti
fi

if [ "$restartbot" ]; then
	systemctl start spotti-downbot
	systemctl enable spotti-downbot
fi

systemctl daemon-reload
