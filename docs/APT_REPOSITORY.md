# Dockermon APT repository

Dockermon publishes its latest Debian package as a signed APT repository through GitHub Pages at
`https://apt.forseti.pl`. GitHub Releases retains the downloadable package for each release, while
Pages contains the latest package, APT indexes, DEP-11 AppStream metadata, cached icons, and the
public repository key.

## Repository contents

The release workflow generates this structure without committing generated binaries or indexes to
Git:

```text
dists/stable/InRelease
dists/stable/Release
dists/stable/Release.gpg
dists/stable/main/binary-amd64/Packages.xz
dists/stable/main/dep11/Components-amd64.yml.gz
dists/stable/main/dep11/icons-*.tar.gz
pool/main/d/dockermon/dockermon_VERSION_amd64.deb
dockermon-archive-keyring.gpg
dockermon.sources
```

The DEP-11 files make Dockermon searchable in GNOME Software and provide its icon, license, release
description, and project links before installation.

## One-time maintainer setup

### 1. Create a dedicated signing key

Create this key outside the repository and protect it with a strong passphrase. Do not reuse a
personal signing key.

```bash
signing_home="$(mktemp -d)"
chmod 0700 "${signing_home}"
gpg --homedir "${signing_home}" --quick-generate-key \
  "Dockermon APT Repository <marcin.pazdziora@forseti.pl>" \
  rsa3072 sign 2y
gpg --homedir "${signing_home}" --list-secret-keys --keyid-format long
```

Copy the full fingerprint from the output, then export an encrypted backup to a secure location:

```bash
signing_fingerprint="FULL_FINGERPRINT"
gpg --homedir "${signing_home}" --armor \
  --export-secret-keys "${signing_fingerprint}" \
  > /secure/offline/location/dockermon-apt-private.asc
gpg --homedir "${signing_home}" --armor \
  --export "${signing_fingerprint}" \
  > /secure/offline/location/dockermon-apt-public.asc
```

Keep the private-key backup and its passphrase outside the repository. The key expires after two
years so its expiration can be extended or a replacement can be published deliberately.
After confirming that both backup files are readable and the GitHub secrets from the next step are
configured, remove the temporary signing directory stored in `${signing_home}`.

### 2. Configure the release-signing environment

Create an environment named `release-signing` under **Settings → Environments**. Add these
environment secrets:

- `APT_SIGNING_PRIVATE_KEY`: the complete armored contents of `dockermon-apt-private.asc`.
- `APT_SIGNING_KEY_PASSPHRASE`: the signing-key passphrase.

With GitHub CLI, the same operation is:

```bash
gh secret set --env release-signing APT_SIGNING_PRIVATE_KEY \
  < /secure/offline/location/dockermon-apt-private.asc
gh secret set --env release-signing APT_SIGNING_KEY_PASSPHRASE
```

The second command prompts for the passphrase without adding it to shell history.

If the account plan exposes deployment protection rules, require manual approval for the
`release-signing` environment. Only the APT repository job receives these secrets, after the Debian
package has already been built and validated.

### 3. Configure GitHub Pages and DNS

1. Make the repository public.
2. In **Settings → Pages**, set **Source** to **GitHub Actions**.
3. In the same view, configure the custom domain `apt.forseti.pl`.
4. Configure a DNS `CNAME` record from `apt.forseti.pl` to `forsetius.github.io`.
5. Enable **Enforce HTTPS** after GitHub finishes issuing the certificate.

The custom domain keeps the user-facing repository URL independent of GitHub Pages. The repository
can move to another host later without changing users' APT configuration.

## Publishing a release

The release workflow accepts tags that exactly match the version in `package.json`, for example
`v0.1.3` for package version `0.1.3`.

After the intended commit is merged into `master` and CI succeeds:

```bash
git switch master
git pull --ff-only
git tag -a v0.1.3 -m "Dockermon 0.1.3"
git push origin v0.1.3
```

The workflow then:

1. builds and validates the Debian package;
2. generates and signs the APT and DEP-11 indexes;
3. verifies the repository through an isolated real APT client;
4. creates the GitHub Release and uploads the package;
5. deploys the repository through GitHub Pages.

The workflow is safe to rerun. An existing GitHub Release is reused and its Debian asset is
replaced with the newly validated artifact.

## Local repository validation

`scripts/test-apt-repository.sh` generates an ephemeral signing key, builds a temporary repository,
runs `apt update` against it, checks that APT selects the expected Dockermon version, confirms that
APT downloads DEP-11 metadata, and compares the package downloaded through APT with the source
artifact.

The test needs `appstream-compose`, `apt-utils`, `gnupg`, and `xz-utils`:

```bash
sudo apt install appstream-compose apt-utils gnupg xz-utils
./scripts/test-apt-repository.sh \
  src-tauri/target/release/bundle/deb/Dockermon_0.1.3_amd64.deb
```

## User installation

Users can install the repository key and source definition published by Pages:

```bash
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSLo /tmp/dockermon-archive-keyring.gpg \
  https://apt.forseti.pl/dockermon-archive-keyring.gpg
sudo install -m 0644 /tmp/dockermon-archive-keyring.gpg \
  /etc/apt/keyrings/dockermon-archive-keyring.gpg
curl -fsSLo /tmp/dockermon.sources https://apt.forseti.pl/dockermon.sources
sudo install -m 0644 /tmp/dockermon.sources \
  /etc/apt/sources.list.d/dockermon.sources
sudo apt update
sudo apt install dockermon
```

The generated source definition uses `Signed-By`, so the Dockermon key is trusted only for this
repository rather than globally.
