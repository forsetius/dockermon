#!/usr/bin/env bash

set -euo pipefail

readonly script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly repository_root="$(cd -- "${script_directory}/.." && pwd)"
readonly bundle_directory="${repository_root}/src-tauri/target/release/bundle/deb"
readonly desktop_identifier="pl.forseti.dockermon"

cd "${repository_root}"

pnpm tauri build --bundles deb

package_version="$(node --print "require('./package.json').version")"
shopt -s nullglob
package_candidates=("${bundle_directory}/Dockermon_${package_version}_"*.deb)
shopt -u nullglob

if (( ${#package_candidates[@]} != 1 )); then
  printf 'Expected exactly one Dockermon %s Debian package, found %s.\n' \
    "${package_version}" "${#package_candidates[@]}" >&2
  exit 1
fi

readonly package_path="${package_candidates[0]}"
temporary_directory="$(mktemp -d "${TMPDIR:-/tmp}/dockermon-deb.XXXXXX")"
readonly temporary_directory
trap 'rm -rf -- "${temporary_directory}"' EXIT

readonly package_root="${temporary_directory}/package-root"
dpkg-deb --raw-extract "${package_path}" "${package_root}"

readonly generated_desktop_file="${package_root}/usr/share/applications/Dockermon.desktop"
readonly normalized_desktop_file="${package_root}/usr/share/applications/${desktop_identifier}.desktop"
readonly control_file="${package_root}/DEBIAN/control"
readonly homepage_url="https://github.com/forsetius/dockermon"

if [[ ! -f "${generated_desktop_file}" ]]; then
  printf 'Generated desktop entry was not found: %s\n' "${generated_desktop_file}" >&2
  exit 1
fi

mv -- "${generated_desktop_file}" "${normalized_desktop_file}"

if grep -q '^Homepage:' "${control_file}"; then
  if ! grep -Fxq "Homepage: ${homepage_url}" "${control_file}"; then
    printf 'Generated package contains an unexpected Homepage field.\n' >&2
    exit 1
  fi
else
  sed -i "/^Description:/i Homepage: ${homepage_url}" "${control_file}"
fi

(
  cd "${package_root}"
  find . -path './DEBIAN' -prune -o -type f -print0 \
    | LC_ALL=C sort -z \
    | xargs -0 md5sum \
    | sed 's#  \./#  #' > DEBIAN/md5sums
)

readonly rebuilt_package="${temporary_directory}/$(basename -- "${package_path}")"
dpkg-deb --root-owner-group --build "${package_root}" "${rebuilt_package}"
mv -- "${rebuilt_package}" "${package_path}"

printf '%s\n' "${package_path}"
