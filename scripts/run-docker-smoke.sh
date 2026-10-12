#!/usr/bin/env bash

set -euo pipefail

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SMOKE_ROOT="$(mktemp -d /tmp/dockermon-smoke.XXXXXX)"
SMOKE_ENGINE_NAME="dockermon-smoke-engine-$$"
SMOKE_SOCKET_PATH="$SMOKE_ROOT/docker.sock"
SMOKE_DOCKER_HOST="unix://$SMOKE_SOCKET_PATH"
SMOKE_COMPOSE_FILE="$REPOSITORY_ROOT/tests/fixtures/docker-smoke.compose.yml"
DIND_IMAGE="${DOCKERMON_DIND_IMAGE:-docker:27-dind}"
FIXTURE_IMAGE="${DOCKERMON_SMOKE_FIXTURE_IMAGE:-alpine:3.21}"
HOST_DOCKER_HOST="${DOCKER_HOST:-}"

host_docker() {
  if [[ -n "$HOST_DOCKER_HOST" ]]; then
    DOCKER_HOST="$HOST_DOCKER_HOST" docker "$@"
  else
    env -u DOCKER_HOST docker "$@"
  fi
}

cleanup() {
  if [[ "$SMOKE_ENGINE_NAME" == dockermon-smoke-engine-* ]]; then
    host_docker rm --force "$SMOKE_ENGINE_NAME" >/dev/null 2>&1 || true
  fi
  if [[ "$SMOKE_ROOT" == /tmp/dockermon-smoke.* ]]; then
    rm -rf -- "$SMOKE_ROOT"
  fi
}
trap cleanup EXIT

smoke_docker() {
  DOCKER_HOST="$SMOKE_DOCKER_HOST" docker "$@"
}

wait_for_engine() {
  local attempt
  for attempt in $(seq 1 60); do
    if host_docker exec "$SMOKE_ENGINE_NAME" test -S /var/run/dockermon/docker.sock >/dev/null 2>&1; then
      host_docker exec "$SMOKE_ENGINE_NAME" chmod 666 /var/run/dockermon/docker.sock
      if smoke_docker info >/dev/null 2>&1; then
        return 0
      fi
    fi
    sleep 1
  done

  host_docker logs "$SMOKE_ENGINE_NAME"
  echo "The isolated Docker Engine did not become ready." >&2
  return 1
}

reset_engine() {
  if [[ "$DOCKER_HOST" != "$SMOKE_DOCKER_HOST" ]]; then
    echo "Refusing to reset a Docker Engine outside the smoke-test socket." >&2
    return 1
  fi
  smoke_docker system prune --all --force --volumes >/dev/null
  smoke_docker pull "$FIXTURE_IMAGE" >/dev/null
}

run_ignored_test() {
  local test_name="$1"
  DOCKER_HOST="$SMOKE_DOCKER_HOST" \
    DOCKERMON_SMOKE_DOCKER_HOST="$SMOKE_DOCKER_HOST" \
    DOCKERMON_SMOKE_COMPOSE_FILE="$SMOKE_COMPOSE_FILE" \
    XDG_CONFIG_HOME="$SMOKE_ROOT/config" \
    cargo test \
      --manifest-path "$REPOSITORY_ROOT/src-tauri/Cargo.toml" \
      --locked \
      "$test_name" \
      -- \
      --exact \
      --ignored \
      --nocapture \
      --test-threads=1
}

host_docker run \
  --detach \
  --privileged \
  --name "$SMOKE_ENGINE_NAME" \
  --env DOCKER_TLS_CERTDIR= \
  --volume "$SMOKE_ROOT:/var/run/dockermon" \
  "$DIND_IMAGE" \
  --host="unix:///var/run/dockermon/docker.sock" \
  --storage-driver=vfs >/dev/null

wait_for_engine
export DOCKER_HOST="$SMOKE_DOCKER_HOST"
reset_engine

smoke_docker compose -f "$SMOKE_COMPOSE_FILE" -p dockermon-smoke-known-a up -d worker
smoke_docker compose -f "$SMOKE_COMPOSE_FILE" -p dockermon-smoke-known-b up -d worker
smoke_docker run --detach --name dockermon-smoke-standalone "$FIXTURE_IMAGE" \
  /bin/sh -c "trap 'exit 0' TERM INT; while true; do sleep 1; done" >/dev/null
smoke_docker pause dockermon-smoke-standalone >/dev/null
run_ignored_test \
  docker_runtime::tests::stops_all_containers_in_an_isolated_engine_and_reports_partial_failure

reset_engine
smoke_docker compose -f "$SMOKE_COMPOSE_FILE" -p dockermon-smoke up -d --scale logger=2 logger
run_ignored_test docker_runtime::tests::streams_and_merges_logs_from_a_live_compose_service
