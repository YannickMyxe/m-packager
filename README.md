# m-packager

`m-packager` is intended to install a Minecraft server from a Modrinth `.mrpack` modpack. The project is currently a minimal Rust application; the installer described here is the proposed design, not implemented functionality.

## Proposed installation flow

1. Read the `.mrpack` archive and validate `modrinth.index.json`.
2. Determine the Minecraft version and mod loader/version from the manifest. The manifest is the default source of truth; for a NeoForge pack, the installer should use the declared NeoForge version rather than ask the user to choose an unrelated loader.
3. Create a staging directory and install the server files:
   - Extract pack `overrides/`, then apply `server-overrides/` so server-specific files take precedence.
   - Copy configuration and other files from both override directories into the server directory, with `server-overrides/` taking precedence.
   - Download required files listed in the index from their declared URLs, skipping files marked unsupported on servers. Do not include optional files by default; provide an explicit way to include optional content.
   - Verify downloaded files against the manifest's hashes and sizes before accepting them.
   - Obtain the Minecraft server and loader files for the declared versions, using the loader's supported server installation process.
4. Produce and document a launch command or script appropriate for that loader and Java version. It starts the installed server; it does not install the pack, accept the EULA, or execute pack-supplied scripts.
5. Install into a new or empty server directory initially. Do not replace an existing server or execute arbitrary scripts from a pack automatically.

An install is successful when the installer has staged and verified the selected files, prepared the loader's server files and launch instructions, and safely committed the result to the destination. Starting Minecraft is not part of installation success. The operator must explicitly accept the EULA before starting the server; the installer must not accept it on the operator's behalf.

The installer should treat archives and paths as untrusted input: reject paths that escape the destination, limit unreasonable extraction sizes, and avoid silently overwriting unrelated files. A failed install should not leave a partially installed server in the target directory.

## Docker deployment

The chosen approach is to run `m-packager` in a one-shot installer container. Mount the `.mrpack` file and persistent server directory into it. The installer reads the local pack once for that installation, then downloads the indexed mod and other files into the server directory. It does not need to download the `.mrpack` itself. Once installation succeeds, start the Minecraft server container with that same server directory mounted.

```text
local modpack.mrpack ──mounted──> one-shot installer container
                                           │
                     downloads and verifies indexed files
                                           ▼
                                  persistent volume
                                           │
                                           ▼
                                    server container
```

This keeps the host setup lightweight:

- The host does not need Rust or Java installed.
- The installer and server can use a controlled, documented Java version.
- The server files live outside the container image, so replacing/restarting a container does not discard the world or configuration.
- Installation is a separate, explicit operation rather than something that unexpectedly runs on every server restart.

The project maintains the installer's `Dockerfile`; users should not need to build or run the Rust application on the host. The installer image needs network access and write access to the mounted directory. The installer and server containers must agree on the directory layout and compatible Java version.

A Docker Compose example is a template for users to adapt to their server image, volume paths, and settings. It should leave EULA acceptance false by default, requiring the operator to change it to true before starting the server. The installer should not generate or modify Dockerfiles at runtime.

Run the installer explicitly as a one-shot container or Compose service, wait for it to finish successfully, and then start the server container. Do not reinstall the pack automatically on every server-container startup. Restarting the server only uses the already-installed files. To change packs or apply updates, run the installer again as a separate operation; update support should stage changes and back up the existing server before applying them.

## Backups and updates

Backups are intentionally deferred for the first install-only version. Initially, the installer should refuse to overwrite a non-empty destination unless an explicit safe policy is added.

When updates are implemented, the recommended behavior is to:

1. Back up the existing server files before changing them by default.
2. Stage and verify the new files before applying the update.
3. Preserve user data such as worlds and user-edited configuration unless the user explicitly chooses otherwise.
4. Provide an explicit opt-out flag (for example, `--no-backup`) for users who understand the risk.

## Pack scripts and files

Copy scripts and configuration files included in the pack as files, subject to the same safe-path checks. Do not automatically execute arbitrary scripts supplied by a pack. The installer may generate its own loader-appropriate start script or command. If pack-script execution is supported later, it should require an explicit opt-in and clearly show what will run.

## Step-by-step project plan

Build and verify a usable installer in small end-to-end increments. The first milestone is an install-only CLI for the current NeoForge pack, run explicitly in a one-shot container. Do not start with a TUI, in-place updates, or automatic pack-script execution.

### 1. Define and inspect the real pack

- Keep a representative `.mrpack` available for local testing; do not commit a large pack or copyrighted mod files to the repository.
- Inspect its `modrinth.index.json`, archive entries, overrides, server overrides, and optional/server support flags.
- Confirm the expected Minecraft and NeoForge versions, Java runtime requirement, and how the server should be launched.
- Define required-only as the default; optional files are included only through an explicit user choice.
- Treat pack configuration and other override files as server files to install, applying `server-overrides/` after `overrides/`.
- Define success as verified files plus prepared server components and launch/EULA instructions; starting the server is not required.

**Done when:** the input format and expected output for this pack are understood, including which files should be installed by default, which are optional or server-unsupported, and how overrides affect the result.

### 2. Establish the Rust CLI and test structure

- Choose a small CLI interface, for example `m-packager install <pack.mrpack> --destination <directory>`.
- Add clear help and error output, and return a non-zero exit code on failure.
- Organize the code into focused areas for CLI handling, pack/index parsing, safe file operations, downloads, and loader setup.
- Add unit and integration test scaffolding, using small synthetic archives instead of distributing real mod files.

**Done when:** the command parses arguments and reports invalid input clearly, with tests runnable through Cargo.

### 3. Read and validate the pack

- Open the `.mrpack` as a ZIP and parse `modrinth.index.json`.
- Validate the supported index format and required fields, including Minecraft and loader dependencies.
- Use the manifest's loader and version as the default; initially support the NeoForge path needed by the current pack.
- Interpret file server-support and environment metadata according to the pack format. Install required server-supported files by default; optional files require an explicit user choice.
- Reject unsafe paths, duplicate/conflicting entries, and unreasonable archive sizes before writing files.

**Done when:** valid fixture packs produce a validated install plan, while malformed or unsafe packs fail before changing the destination.

### 4. Install pack-provided files and indexed downloads

- Build the install in a staging directory rather than writing a partial result directly into the final destination.
- Copy configuration and other files from `overrides/`, followed by `server-overrides/`, into the server directory.
- Support an explicit optional-file inclusion choice; never install server-unsupported files.
- Download eligible indexed files to temporary files; enforce size limits and verify declared hashes and sizes before accepting them.
- Report which files are downloaded, skipped, or rejected. Do not silently treat failed downloads as success.

**Done when:** tests cover successful downloads, hash/size mismatches, unsupported files, interrupted requests, override precedence, and unsafe paths.

### 5. Set up the Minecraft and NeoForge server

- Install the Minecraft and NeoForge server components matching the manifest using their supported installation process.
- Make Java requirements explicit and verify that the runtime available to the installer/server is compatible.
- Generate or document a loader-appropriate start command and the operator's first-start steps. Do not execute scripts supplied by the pack or accept the EULA on the user's behalf.

**Done when:** the installer prepares the server components, provides a usable launch command and clear EULA instructions, and does not need to start Minecraft to report installation success.

### 6. Commit a successful install safely

- Initially require a new or empty destination; never silently overwrite an existing server.
- After all downloads, checks, and server setup succeed, move the staged install into place.
- On failure, report the cause and clean up only the installer's own staging data, leaving existing user data untouched.

**Done when:** integration tests prove a failed install does not leave a partial server or alter an existing destination.

### 7. Package and test the one-shot Docker workflow

- Create an installer image with `m-packager` and the required Java runtime.
- Maintain the installer `Dockerfile` in the repository and provide a Compose example users can adapt.
- Document the pack mount, destination mount, permissions, and network access needed by the installer.
- Run the installer as an explicit one-shot command or Compose service; start the server container only after installation succeeds.
- Mount the same persistent server directory into the server container. Keep installation out of the server container's normal restart path.
- Set EULA acceptance to false in the Compose example by default; the operator must change it to true before starting the server.
- Test the full workflow: install once, start the server, restart it without reinstalling, and confirm the installed files and world persist.

**Done when:** the documented Docker workflow can install the representative pack and start its server from a persistent volume.

### 8. Add updates and backups only after initial installs work

- Design updates separately from first-time installation.
- Back up the existing server by default, stage and verify the update, and preserve worlds and user-edited configuration.
- Add an explicit opt-out such as `--no-backup` only when the backup workflow is implemented and tested.
- Consider caching, more loaders, and a TUI based on real usage needs.

**Done when:** an update can be safely applied and recovered from, with tests proving backup, preservation, and failure behavior.
