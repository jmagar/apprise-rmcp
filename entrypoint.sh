#!/bin/sh
set -eu

BINARY="/usr/local/bin/rapprise"
DATA_DIR="${DATA_DIR:-/data}"

if [ -z "${APPRISE_URL:-}" ]; then
    echo "ERROR: APPRISE_URL is not set" >&2
    exit 1
fi
if [ ! -d "${DATA_DIR}" ] || [ ! -w "${DATA_DIR}" ]; then
    echo "ERROR: ${DATA_DIR} must exist and be writable by uid $(id -u)" >&2
    exit 1
fi
if [ -f "${DATA_DIR}/.env" ]; then
    chmod 600 "${DATA_DIR}/.env" || {
        echo "ERROR: ${DATA_DIR}/.env must be owned by uid $(id -u) so permissions can be secured" >&2
        exit 1
    }
fi

exec "${BINARY}" "$@"
