#!/bin/bash

CheckStatusCode() {
        code=`curl -s -o /dev/null -w '%{http_code}' $1`
        echo "$1 => $code"
        if [ ! "$code" = "200" ]; then
                $2 $3
                exit 1
        fi
}

Perish() {
        echo refresh failed > {{ var_dir("spotti") }}/status.txt
        botpid=`cat {{ var_dir("spotti-downbot") }}/bot-pid.txt`
        kill -usr1 $botpid
}

Refresh() {
        CheckStatusCode https://grape.surgery/spotti/refresh Perish 'refresh'
        CheckStatusCode https://grape.surgery/spotti Perish 'query'
}

Refresh
