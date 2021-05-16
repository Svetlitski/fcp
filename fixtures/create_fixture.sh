#!/usr/bin/env sh
tree -sJpf --noreport --dirsfirst -o "fixtures/${1##*/}.json" -- "$1"
