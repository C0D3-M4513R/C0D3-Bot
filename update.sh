#!/usr/bin/env sh
git stash
docker-compose build
docker push c0d3m4513r1/rust-dc-bot:latest
git stash pop