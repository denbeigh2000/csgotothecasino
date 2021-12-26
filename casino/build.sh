#!/bin/sh

set -eu

export DOCKER_BUILDKIT=1

BASE_IMG="denbeigh2000/casino:base"
STUB_IMG="denbeigh2000/casino:stub"

docker build -t "$BASE_IMG" -f base.Dockerfile .
docker build -t "$STUB_IMG" .
docker-compose build

echo "run stub: docker run -p 7000:7000 $STUB_IMG"
echo "run real: docker-compose up"
