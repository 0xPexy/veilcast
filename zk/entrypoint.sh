#!/bin/bash
set -e
export PATH="/home/dev/.nargo/bin:/home/dev/.bbup/bin:${PATH}"
exec "$@"
