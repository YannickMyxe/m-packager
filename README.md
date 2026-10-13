# m-packager

`m-packager` is intended to install a Minecraft server from a Modrinth `.mrpack` modpack. The project is currently a minimal Rust application; the installer described here is the proposed design, not implemented functionality.

## Proposed installation flow

1. Read the `.mrpack` archive and validate `modrinth.index.json`.
2. Determine the Minecraft version and mod loader/version from the manifest. The manifest is the default source of truth; for a NeoForge pack, the installer should use the declared NeoForge version rather than ask the user to choose an unrelated loader.
3. Create a staging directory and install the server files:
   - Extract pack `overrides/`, then apply `server-overrides/` so server-specific files take precedence.
   - Download files listed in the index from their declared URLs, skipping files marked unsupported on servers. Optional files should have a documented default and may be made selectable.
   - Verify downloaded files against the manifest's hashes and sizes before accepting them.
   - Obtain the Minecraft server and loader files for the declared versions, using the loader's supported server installation process.
4. Produce a launch command or script appropriate for that loader and Java version.
5. Install into a new or empty server directory initially. Do not replace an existing server or execute arbitrary scripts from a pack automatically.

The installer should treat archives and paths as untrusted input: reject paths that escape the destination, limit unreasonable extraction sizes, and avoid silently overwriting unrelated files. A failed install should not leave a partially installed server in the target directory.

## Docker deployment options

Both approaches can work. They differ in where the installer runs and what has to be installed on the host.

### Option A: Run the installer in a one-shot Docker container (recommended)

Build or pull an installer image containing `m-packager` and a compatible Java runtime. Run it with the `.mrpack` file and persistent server directory mounted into the container. Once installation succeeds, start the existing Minecraft server container with that same server directory mounted.

```text
modpack.mrpack ──mounted──> installer container
                                  │
                           prepares server files
                                  ▼
                           persistent volume
                                  │
                                  ▼
                            server container
```

**Advantages**

- The host does not need Rust or Java installed.
- The installer and server can use a controlled, documented Java version.
- The server files live outside the container image, so replacing/restarting a container does not discard the world or configuration.
- Installation is a separate, explicit operation rather than something that unexpectedly runs on every server restart.

**Trade-offs**

- The installer image and its Java version need to be maintained.
- The installer needs network access and write access to the mounted directory.
- The installer and server containers must agree on the directory layout and compatible Java version.

This should be a one-shot install/update command or Compose service that runs to completion before the server starts. It should not reinstall the pack automatically on every server-container startup. A later update flow should stage changes and back up the existing server before applying them.

### Option B: Run the installer on the host before starting Docker

Install or build `m-packager` on the host and run it against the server directory that Docker will mount as a volume. Then start the server container as usual.

```text
modpack.mrpack ──> m-packager on host ──> server directory/volume ──> server container
```

**Advantages**

- No separate installer container or image is needed.
- The install step is straightforward to run locally or from a deployment script.
- The generated server files can be inspected directly on the host before starting the server.

**Trade-offs**

- The host needs the installer and any runtime dependencies required for server setup, especially a compatible Java runtime.
- Different host operating systems and Java installations can make setup less consistent.
- File permissions and ownership must be compatible with the user running the server container.

### Recommendation

Use **Option A** if Docker is the normal way this server is deployed. It keeps installation dependencies out of the host and makes the runtime environment easier to reproduce. Choose **Option B** if you already manage Java and server files on the host, or want the simplest local development workflow.

Either way, use a persistent directory/volume shared with the server container. Install explicitly, and initially require a new or empty destination rather than modifying an existing server in place.

## Backups and updates

Backups are intentionally deferred for the first install-only version. Initially, the installer should refuse to overwrite a non-empty destination unless an explicit safe policy is added.

When updates are implemented, the recommended behavior is to:

1. Back up the existing server files before changing them by default.
2. Stage and verify the new files before applying the update.
3. Preserve user data such as worlds and user-edited configuration unless the user explicitly chooses otherwise.
4. Provide an explicit opt-out flag (for example, `--no-backup`) for users who understand the risk.

## Pack scripts and files

Copy scripts and configuration files included in the pack as files, subject to the same safe-path checks. Do not automatically execute arbitrary scripts supplied by a pack. The installer may generate its own loader-appropriate start script or command. If pack-script execution is supported later, it should require an explicit opt-in and clearly show what will run.

## Initial implementation scope

- Command-line installer; no TUI required for the first version.
- Read loader and Minecraft versions from the pack manifest.
- Start with the current NeoForge pack/use case.
- Install into a new or empty directory.
- Apply overrides and server overrides.
- Download and verify server-compatible indexed files.
- Install the declared Minecraft/NeoForge server files and provide a launch command.
- Add tests for malformed archives, unsafe paths, hash/size mismatches, and failed/interrupted downloads.

Optional-file selection, in-place updates, backups, and additional loaders can follow once the first end-to-end NeoForge install works.
