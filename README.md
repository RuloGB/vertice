# Vertice

Vertice is a desktop application that inventories the AI components — skills and
agents — installed by AI clients (Claude Code, OpenCode, Copilot, Codex…) on your
machine. It is **read-only**: it never writes outside its own application data
directory.

## Install (Windows)

Windows is the only platform published today. macOS and Linux build in CI but no
installer is distributed for them yet.

1. Open the [Releases](https://github.com/RuloGB/vertice/releases) page and pick
   the latest release.
2. Download one of:
   - `Vertice_<version>_x64-setup.exe` — the normal installer (NSIS).
   - `Vertice_<version>_x64_en-US.msi` — the same application as an MSI, for
     managed or scripted deployments.
3. Run it. Windows will show a warning; read the next section before dismissing it.

## Why Windows warns about this installer

On first run, Windows SmartScreen shows **"Windows protected your PC"** and hides
the *Run anyway* button behind *More info*.

This is not a virus report. SmartScreen has no opinion about what the file does.
The warning means the installer is **not signed with a code-signing certificate**,
so Windows cannot tie the file to a verified publisher, and the file has no
established download reputation yet.

Vertice does not ship a code-signing certificate. Certificates are issued to a
legal identity, cost money to maintain annually, and the project has not taken
that step yet. Until it does, every release will produce this warning — including
releases that are exactly what they claim to be.

If you would rather not dismiss a warning you cannot check, don't. Verify the
download first.

### Verify the download before running it

Every installer is built by a public GitHub Actions workflow, and that workflow
signs a **build provenance attestation** for the exact bytes it produced. The
attestation is free, needs no certificate, and records which commit and which
workflow run built the file. It does not remove the SmartScreen warning — it
gives you a way to check the file yourself instead of trusting it blindly.

With the [GitHub CLI](https://cli.github.com/) installed:

```bash
gh attestation verify Vertice_0.1.0_x64-setup.exe --repo RuloGB/vertice
```

A successful run reports the commit and workflow that produced the file. If
verification fails, the file did not come from this project's release pipeline —
delete it and do not run it.

### Dismissing the warning

Once you have verified the download and decided to proceed:

1. Click **More info** in the SmartScreen dialog.
2. Click **Run anyway**.

You can also unblock the file before running it, in PowerShell:

```powershell
Unblock-File -Path .\Vertice_0.1.0_x64-setup.exe
```

## Build from source

Requires the Rust toolchain pinned in `rust-toolchain.toml` and Node.js 22.

```bash
npm --prefix frontend ci
```

```bash
npx --prefix frontend tauri build
```

For the day-to-day development loop, see [AGENTS.md](AGENTS.md).

## License

MIT OR Apache-2.0.
