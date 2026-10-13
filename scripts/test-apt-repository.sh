#!/usr/bin/env bash

set -euo pipefail

if (( $# > 2 )); then
  printf 'Usage: %s [PACKAGE.deb [REPOSITORY_DIRECTORY]]\n' "$0" >&2
  exit 2
fi

readonly script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly repository_root="$(cd -- "${script_directory}/.." && pwd)"

if (( $# >= 1 )); then
  package_path="$1"
else
  shopt -s nullglob
  package_candidates=("${repository_root}/src-tauri/target/release/bundle/deb/"*.deb)
  shopt -u nullglob
  if (( ${#package_candidates[@]} != 1 )); then
    printf 'Expected exactly one built Debian package, found %s.\n' \
      "${#package_candidates[@]}" >&2
    exit 1
  fi
  package_path="${package_candidates[0]}"
fi

readonly package_path
readonly test_directory="$(mktemp -d "${TMPDIR:-/tmp}/dockermon-apt-test.XXXXXX")"
trap 'rm -rf -- "${test_directory}"' EXIT

if (( $# == 2 )); then
  readonly generated_repository="$(realpath -- "$2")"
else
  export GNUPGHOME="${test_directory}/gnupg"
  mkdir -m 0700 "${GNUPGHOME}"

  readonly signing_passphrase="dockermon-apt-test-only"
  gpg \
    --batch \
    --pinentry-mode loopback \
    --passphrase "${signing_passphrase}" \
    --quick-generate-key \
    'Dockermon APT Test <apt-test@dockermon.invalid>' \
    ed25519 \
    sign \
    1d

  signing_fingerprint="$(
    gpg --batch --with-colons --list-secret-keys --fingerprint \
      | awk -F: '$1 == "fpr" { print $10; exit }'
  )"
  readonly signing_fingerprint

  if [[ -z "${signing_fingerprint}" ]]; then
    printf 'Could not determine the temporary signing key fingerprint.\n' >&2
    exit 1
  fi

  export APT_REPOSITORY_URL="file:${test_directory}/repository"
  export APT_SIGNING_KEY_FINGERPRINT="${signing_fingerprint}"
  export APT_SIGNING_KEY_PASSPHRASE="${signing_passphrase}"

  "${script_directory}/build-apt-repository.sh" \
    "${package_path}" \
    "${test_directory}/repository"

  readonly generated_repository="${test_directory}/repository"
fi

readonly apt_root="${test_directory}/apt"
readonly apt_lists="${apt_root}/state/lists"
readonly apt_archives="${apt_root}/cache/archives"
readonly apt_sources="${apt_root}/etc/apt/sources.list.d"

mkdir -p \
  "${apt_lists}/partial" \
  "${apt_archives}/partial" \
  "${apt_sources}" \
  "${test_directory}/downloads"

cat > "${apt_sources}/dockermon.sources" <<EOF
Types: deb
URIs: file:${generated_repository}
Suites: stable
Components: main
Architectures: amd64
Signed-By: ${generated_repository}/dockermon-archive-keyring.gpg
EOF

readonly apt_options=(
  -o "APT::Sandbox::User=$(id -un)"
  -o 'Debug::NoLocking=true'
  -o 'Dir::Etc::sourcelist=-'
  -o "Dir::Etc::sourceparts=${apt_sources}"
  -o "Dir::State::lists=${apt_lists}"
  -o 'Dir::State::status=/var/lib/dpkg/status'
  -o "Dir::Cache=${apt_root}/cache"
  -o "Dir::Cache::archives=${apt_archives}"
)

apt-get "${apt_options[@]}" update

readonly expected_version="$(dpkg-deb --field "${package_path}" Version)"
readonly policy_output="$(apt-cache "${apt_options[@]}" policy dockermon)"

if ! grep -Fq "Candidate: ${expected_version}" <<< "${policy_output}"; then
  printf '%s\n' "${policy_output}" >&2
  printf 'APT did not select Dockermon %s from the generated repository.\n' \
    "${expected_version}" >&2
  exit 1
fi

if ! find "${apt_lists}" -maxdepth 1 \( -type f -o -type l \) \
  -name '*dep11_Components-amd64.yml*' -print -quit | grep -q .; then
  find "${apt_lists}" -maxdepth 2 -printf '%y %P\n' >&2
  printf 'APT did not download the generated DEP-11 metadata.\n' >&2
  exit 1
fi

(
  cd "${test_directory}/downloads"
  apt-get "${apt_options[@]}" download dockermon
)

readonly downloaded_package="${test_directory}/downloads/dockermon_${expected_version}_amd64.deb"

if [[ ! -f "${downloaded_package}" ]]; then
  printf 'APT did not download the expected package: %s\n' "${downloaded_package}" >&2
  exit 1
fi

if ! cmp --silent "${package_path}" "${downloaded_package}"; then
  printf 'Package downloaded through APT differs from the source artifact.\n' >&2
  exit 1
fi

printf 'APT repository test passed for Dockermon %s.\n' "${expected_version}"
