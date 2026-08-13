<!-- SPDX-SnippetBegin -->
<!-- SPDX-SnippetCopyrightText: 2026 Voxtera Contributors -->
<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

<div align="center">

<img src="site/public/images/voxtera-logo.png" alt="Voxtera Logo" width="240" />

# Voxtera

*An open-source, procedural voxel action-adventure RPG built in Rust.*

[![License: GPL v3](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)
[![Rust Edition](https://img.shields.io/badge/Rust-2024_Edition-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Release](https://img.shields.io/github/v/release/stoltembergg-png/voxtera?label=release)](https://github.com/stoltembergg-png/voxtera/releases)
[![Website](https://img.shields.io/badge/Website-voxtera--nu.vercel.app-black?style=flat&logo=vercel)](https://voxtera-nu.vercel.app)
[![Platforms](https://img.shields.io/badge/Platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)](#build-from-source)

<br />

<img src="site/public/images/voxtera-clean-hero.png" alt="Voxtera Hero Banner" width="100%" />

</div>

---

## Overview

Voxtera is an open-world voxel action-adventure RPG set in a procedurally generated fantasy universe. The project combines a Rust game engine, real-time multiplayer, procedural world generation, dedicated servers, and a web download portal.

- Procedural voxel worlds with biomes, mountains, ruins, and dungeons.
- Real-time action combat, RPG progression, stamina, weapons, and abilities.
- Cross-platform rendering based on Rust and `wgpu`.
- Multiplayer networking based on Quinn/QUIC and a dedicated server architecture.
- Real-time NPC and world simulation through `rtsim`.
- GNU GPL v3.0 open-source licensing.

## Current Release

The current stable distribution is [Voxtera `v0.4.12`](https://github.com/stoltembergg-png/voxtera/releases/tag/v0.4.12).

| Item | Current value |
| :--- | :--- |
| Stable release | `v0.4.12` |
| Default game server | `15.228.166.136:14004` |
| Windows launcher | [VoxteraLauncher.exe](https://github.com/stoltembergg-png/voxtera/releases/download/v0.4.12/VoxteraLauncher.exe) |
| Windows client archive | [Voxtera-windows-x64-v0.4.12.zip](https://github.com/stoltembergg-png/voxtera/releases/download/v0.4.12/Voxtera-windows-x64-v0.4.12.zip) |
| Linux server archive | [voxtera-server-linux-x86_64-v0.4.12.tar.gz](https://github.com/stoltembergg-png/voxtera/releases/download/v0.4.12/voxtera-server-linux-x86_64-v0.4.12.tar.gz) |
| Release manifest | [manifest-v0.4.12.json](https://github.com/stoltembergg-png/voxtera/releases/download/v0.4.12/manifest-v0.4.12.json) |
| Web portal | [voxtera-nu.vercel.app](https://voxtera-nu.vercel.app) |

The Linux server release is built in Rocky Linux 9 and is checked against a maximum required symbol version of `GLIBC_2.34`. The release pipeline also verifies hydrated Git LFS assets, archive contents, SHA-256 hashes, and the executable bit of `veloren-server-cli`.

### `v0.4.12` fixes

- Replaces the old default server address with `15.228.166.136:14004`.
- Migrates legacy saved server addresses when existing settings are loaded.
- Fixes the Conrod `WouldCycle(Depth)` panic triggered when opening the social panel with `O`.
- Uses dedicated `tab_underlines[]` widget IDs instead of reusing tab button IDs.
- Updates the Windows launcher and web download redirects to `v0.4.12`.

## Features and Gameplay

<div align="center">

| Explore | Build |
| :---: | :---: |
| <img src="site/public/images/mountain-valley.jpg" alt="Explore Mountain Valley" width="450" /> | <img src="site/public/images/voxtera-build-village.png" alt="Build Voxel Village" width="450" /> |
| Traverse procedural valleys, frozen peaks, and ancient ruins. | Construct settlements, fortresses, and homes with other players. |

<br />

| Adventure | In-game experience |
| :---: | :---: |
| <img src="site/public/images/ruins-adventure.jpg" alt="Combat in Volcanic Ruins" width="450" /> | <img src="site/public/images/gameplay-capture.png" alt="Voxtera Gameplay Capture" width="450" /> |
| Battle monsters, discover lore, and conquer dungeon challenges. | Experience real-time lighting, shaders, and action RPG systems. |

</div>

## Play Voxtera

### Web portal and downloads

The official download portal is [voxtera-nu.vercel.app](https://voxtera-nu.vercel.app).

| Platform | Download | Notes |
| :--- | :--- | :--- |
| Windows | [Download the launcher](https://voxtera-nu.vercel.app/downloads/VoxteraLauncher.exe) | Windows 10/11, 64-bit |
| Linux | [Download the server archive](https://github.com/stoltembergg-png/voxtera/releases/download/v0.4.12/voxtera-server-linux-x86_64-v0.4.12.tar.gz) | Dedicated server; client can be built from source |
| macOS | Build from source | No macOS binary is included in the current `v0.4.12` release |

The launcher downloads the client, applies updates, and connects to the configured server. Existing installations with the previous hostname are migrated automatically when settings are loaded.

## Workspace Architecture

Voxtera is organized as a modular Cargo workspace:

```text
voxtera/
├── voxygen/       # Game client GUI, rendering, input, animation, and shaders
├── server/        # Dedicated game server engine and tick loop
├── server-cli/    # Server management and administration interface
├── client/        # Client networking, authentication, and entity state
├── common/        # Shared ECS components, assets, physics, and game state
├── world/         # Procedural world generation, climate, biomes, and dungeons
├── rtsim/         # Real-time NPC and world simulation
├── network/       # Low-latency Quinn/QUIC networking layer
└── site/          # Vite/React download portal deployed on Vercel
```

The Cargo workspace currently uses Rust 2024 edition. The distribution release version and the internal workspace version are tracked separately: `v0.4.12` is the current packaged release, while the workspace manifest contains the engine development version.

## Build from Source

### Prerequisites

- Rust toolchain with Rust 2024 edition support.
- Git LFS for repository assets.
- A C/C++ compiler and CMake for native dependencies.
- Graphics drivers supporting Vulkan, DirectX 12, or Metal, depending on the platform.
- Node.js and pnpm for the web portal.

### Clone and hydrate assets

```bash
git clone https://github.com/stoltembergg-png/voxtera.git
cd voxtera
git lfs install
git lfs pull
```

### Run the client

```bash
cargo run --release --bin Voxtera
```

Omit `--release` for faster development builds.

### Run a local dedicated server

```bash
cargo run --release --package veloren-server-cli
```

### Run the web portal

```bash
cd site
pnpm install --frozen-lockfile
pnpm dev
```

The production build and tests can be run with:

```bash
pnpm test -- --run
pnpm build
```

## Verification and Release Gates

The repository uses automated checks for the release and regression-sensitive areas:

```bash
python scripts/friends_panel_widget_id_gate.py
python scripts/test_release_workflow.py
```

The GitHub Actions release workflow additionally validates:

- Rust formatting and server compilation/tests;
- Windows client and launcher packaging;
- Linux server compatibility with glibc `2.34`;
- hydrated Git LFS assets;
- archive contents and Linux executable permissions;
- release manifest and SHA-256 hashes.

## Contributing

Contributions are welcome from engine developers, server operators, artists, translators, and players.

1. Fork the repository or create a feature branch from `main`.
2. Keep changes focused and include regression tests or validation gates where applicable.
3. Run the relevant local checks before opening a pull request.
4. Open a pull request against `main` and wait for the required `CI / Quality Gate` check.

The `main` branch is the canonical integration branch. Feature and fix branches should be removed after their pull requests are merged or explicitly closed.

## License

Voxtera is distributed under the [GNU General Public License v3.0 or later](LICENSE).

<div align="center">

<img src="site/public/images/voxtera-closing-valley.png" alt="Voxtera Closing Valley Banner" width="100%" />

*Your journey begins block by block.*

[Website](https://voxtera-nu.vercel.app) · [Documentation](docs/) · [Contributing](CONTRIBUTING.md) · [License](LICENSE)

</div>
