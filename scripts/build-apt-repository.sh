#!/usr/bin/env bash

set -euo pipefail

if (( $# != 2 )); then
  printf 'Usage: %s PACKAGE.deb OUTPUT_DIRECTORY\n' "$0" >&2
  exit 2
fi

readonly package_path="$(realpath -- "$1")"
readonly output_directory="$(realpath -m -- "$2")"
readonly package_name="dockermon"
readonly repository_architecture="amd64"
readonly repository_component="main"
readonly repository_origin="Dockermon"
readonly repository_suite="stable"
readonly repository_url="${APT_REPOSITORY_URL:-https://apt.forseti.pl}"
readonly compose_binary="${APPSTREAM_COMPOSE_BINARY:-/usr/libexec/appstreamcli-compose}"

if [[ ! -f "${package_path}" ]]; then
  printf 'Debian package was not found: %s\n' "${package_path}" >&2
  exit 1
fi

if [[ -e "${output_directory}" ]] &&
  [[ -n "$(find "${output_directory}" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
  printf 'Output directory must be empty: %s\n' "${output_directory}" >&2
  exit 1
fi

if [[ ! -x "${compose_binary}" ]]; then
  printf 'AppStream Compose binary was not found: %s\n' "${compose_binary}" >&2
  exit 1
fi

for required_command in appstreamcli apt-ftparchive dpkg-deb gpg gpgv gzip tar xz; do
  if ! command -v "${required_command}" >/dev/null; then
    printf 'Required command was not found: %s\n' "${required_command}" >&2
    exit 1
  fi
done

if [[ -z "${APT_SIGNING_KEY_FINGERPRINT:-}" ]]; then
  printf 'APT_SIGNING_KEY_FINGERPRINT must identify the repository signing key.\n' >&2
  exit 1
fi

if [[ -z "${APT_SIGNING_KEY_PASSPHRASE:-}" ]]; then
  printf 'APT_SIGNING_KEY_PASSPHRASE must unlock the repository signing key.\n' >&2
  exit 1
fi

readonly detected_package_name="$(dpkg-deb --field "${package_path}" Package)"
readonly package_version="$(dpkg-deb --field "${package_path}" Version)"
readonly package_architecture="$(dpkg-deb --field "${package_path}" Architecture)"

if [[ "${detected_package_name}" != "${package_name}" ]]; then
  printf 'Expected package %s, found %s.\n' "${package_name}" "${detected_package_name}" >&2
  exit 1
fi

if [[ "${package_architecture}" != "${repository_architecture}" ]]; then
  printf 'Expected architecture %s, found %s.\n' \
    "${repository_architecture}" "${package_architecture}" >&2
  exit 1
fi

readonly temporary_directory="$(mktemp -d "${TMPDIR:-/tmp}/dockermon-apt.XXXXXX")"
trap 'rm -rf -- "${temporary_directory}"' EXIT

readonly package_root="${temporary_directory}/package-root"
readonly compose_result="${temporary_directory}/compose-result"
readonly package_pool="${output_directory}/pool/main/d/dockermon"
readonly distribution_directory="${output_directory}/dists/${repository_suite}"
readonly binary_directory="${distribution_directory}/${repository_component}/binary-${repository_architecture}"
readonly dep11_directory="${distribution_directory}/${repository_component}/dep11"

mkdir -p \
  "${package_root}" \
  "${compose_result}" \
  "${package_pool}" \
  "${binary_directory}" \
  "${dep11_directory}"

readonly published_package_name="${package_name}_${package_version}_${package_architecture}.deb"
cp -- "${package_path}" "${package_pool}/${published_package_name}"

(
  cd "${output_directory}"
  apt-ftparchive packages pool/main > "dists/${repository_suite}/${repository_component}/binary-${repository_architecture}/Packages"
)

gzip -9n -c "${binary_directory}/Packages" > "${binary_directory}/Packages.gz"
xz -9e -c "${binary_directory}/Packages" > "${binary_directory}/Packages.xz"

dpkg-deb --extract "${package_path}" "${package_root}"
"${compose_binary}" \
  --no-net \
  --origin="${package_name}" \
  --result-root="${compose_result}" \
  "${package_root}"

readonly composed_catalog="${compose_result}/usr/share/swcatalog/xml/${package_name}.xml.gz"
readonly dep11_metadata="${dep11_directory}/Components-${repository_architecture}.yml"

if [[ ! -f "${composed_catalog}" ]]; then
  printf 'AppStream Compose did not create the expected catalog: %s\n' \
    "${composed_catalog}" >&2
  exit 1
fi

appstreamcli convert --format=yaml "${composed_catalog}" "${dep11_metadata}"

readonly required_dep11_entries=(
  "Package: ${package_name}"
  'ProjectLicense: LicenseRef-Apache-2.0-with-Commons-Clause-1.0'
  'bugtracker: https://github.com/forsetius/dockermon/issues'
  'help: https://github.com/forsetius/dockermon/blob/master/USER_GUIDE.md'
  "version: ${package_version}"
  'First public release of Dockermon.'
)

for required_entry in "${required_dep11_entries[@]}"; do
  if ! grep -Fq "${required_entry}" "${dep11_metadata}"; then
    printf 'Generated DEP-11 metadata is missing: %s\n' "${required_entry}" >&2
    exit 1
  fi
done

gzip -9n -c "${dep11_metadata}" > "${dep11_metadata}.gz"

create_icon_archive() {
  local size="$1"
  local icon_directory="${compose_result}/usr/share/swcatalog/icons/${package_name}/${size}"
  local icon_file="pl.forseti.dockermon.png"
  local archive_path="${dep11_directory}/icons-${size}.tar.gz"

  if [[ ! -f "${icon_directory}/${icon_file}" ]]; then
    printf 'Composed AppStream icon was not found: %s\n' \
      "${icon_directory}/${icon_file}" >&2
    exit 1
  fi

  tar \
    --create \
    --file=- \
    --directory="${icon_directory}" \
    --sort=name \
    --mtime='UTC 1970-01-01' \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    "${icon_file}" \
    | gzip -9n > "${archive_path}"
}

create_icon_archive 48x48
create_icon_archive 64x64
create_icon_archive 64x64@2
create_icon_archive 128x128
create_icon_archive 128x128@2

(
  cd "${output_directory}"
  apt-ftparchive \
    -o "APT::FTPArchive::Release::Origin=${repository_origin}" \
    -o "APT::FTPArchive::Release::Label=${repository_origin}" \
    -o "APT::FTPArchive::Release::Suite=${repository_suite}" \
    -o "APT::FTPArchive::Release::Codename=${repository_suite}" \
    -o "APT::FTPArchive::Release::Architectures=${repository_architecture}" \
    -o "APT::FTPArchive::Release::Components=${repository_component}" \
    -o 'APT::FTPArchive::Release::Description=Dockermon package repository' \
    release "dists/${repository_suite}" > "dists/${repository_suite}/Release"
)

for indexed_path in \
  "${repository_component}/binary-${repository_architecture}/Packages.xz" \
  "${repository_component}/dep11/Components-${repository_architecture}.yml.gz" \
  "${repository_component}/dep11/icons-64x64.tar.gz"; do
  if ! grep -Fq "${indexed_path}" "${distribution_directory}/Release"; then
    printf 'Release file does not index: %s\n' "${indexed_path}" >&2
    exit 1
  fi
done

printf '%s' "${APT_SIGNING_KEY_PASSPHRASE}" \
  | gpg \
    --batch \
    --yes \
    --pinentry-mode loopback \
    --passphrase-fd 0 \
    --local-user "${APT_SIGNING_KEY_FINGERPRINT}" \
    --digest-algo SHA256 \
    --clearsign \
    --output "${distribution_directory}/InRelease" \
    "${distribution_directory}/Release"

printf '%s' "${APT_SIGNING_KEY_PASSPHRASE}" \
  | gpg \
    --batch \
    --yes \
    --pinentry-mode loopback \
    --passphrase-fd 0 \
    --local-user "${APT_SIGNING_KEY_FINGERPRINT}" \
    --digest-algo SHA256 \
    --armor \
    --detach-sign \
    --output "${distribution_directory}/Release.gpg" \
    "${distribution_directory}/Release"

gpg --batch --export "${APT_SIGNING_KEY_FINGERPRINT}" \
  > "${output_directory}/dockermon-archive-keyring.gpg"
gpg --batch --armor --export "${APT_SIGNING_KEY_FINGERPRINT}" \
  > "${output_directory}/dockermon-archive-keyring.asc"

readonly normalized_fingerprint="${APT_SIGNING_KEY_FINGERPRINT//[[:space:]]/}"

cat > "${output_directory}/dockermon.sources" <<EOF
Types: deb
URIs: ${repository_url}/
Suites: ${repository_suite}
Components: ${repository_component}
Architectures: ${repository_architecture}
Signed-By: /etc/apt/keyrings/dockermon-archive-keyring.gpg
EOF

cat > "${output_directory}/index.html" <<EOF
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Dockermon APT repository</title>
  </head>
  <body>
    <main>
      <h1>Dockermon APT repository</h1>
      <p>Current package: Dockermon ${package_version} for amd64.</p>
      <p>Signing key fingerprint: <code>${normalized_fingerprint}</code></p>
      <pre>sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSLo /tmp/dockermon-archive-keyring.gpg ${repository_url}/dockermon-archive-keyring.gpg
sudo install -m 0644 /tmp/dockermon-archive-keyring.gpg /etc/apt/keyrings/dockermon-archive-keyring.gpg
curl -fsSLo /tmp/dockermon.sources ${repository_url}/dockermon.sources
sudo install -m 0644 /tmp/dockermon.sources /etc/apt/sources.list.d/dockermon.sources
sudo apt update
sudo apt install dockermon</pre>
    </main>
  </body>
</html>
EOF

gpgv \
  --keyring "${output_directory}/dockermon-archive-keyring.gpg" \
  "${distribution_directory}/InRelease"
gpgv \
  --keyring "${output_directory}/dockermon-archive-keyring.gpg" \
  "${distribution_directory}/Release.gpg" \
  "${distribution_directory}/Release"

printf '%s\n' "${output_directory}"
