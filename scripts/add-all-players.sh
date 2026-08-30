#!/bin/sh

set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

"$script_dir/beta.sh" players add \
    account.0002@example.com \
    account.0003@example.com \
    account.0004@example.com \
    account.0005@example.com \
    account.0006@example.com \
    account.0007@example.com \
    account.0008@example.com \
    account.0009@example.com \
    account.0010@example.com \
    account.0011@example.com \
    account.0012@example.com \
    account.0013@example.com
