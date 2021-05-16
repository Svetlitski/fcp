#!/usr/bin/env sh
tree -sJpf --noreport --dirsfirst -o "${1##*/}.json" -- "$1"
