<!-- SPDX-SnippetBegin -->
<!-- SPDX-SnippetCopyrightText: 2026 Voxtera Contributors -->
<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

<div align="center">

<img src="site/public/images/voxtera-logo.png" alt="Voxtera Logo" width="240" />

# **Voxtera**

*An open-source, procedural voxel action-adventure RPG built in Rust.*

[![License: GPL v3](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)
[![Rust Edition](https://img.shields.io/badge/Rust-2024_Edition-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Website](https://img.shields.io/badge/Website-voxtera.vercel.app-black?style=flat&logo=vercel)](https://voxtera.vercel.app)
[![Version](https://img.shields.io/badge/Version-0.18.0--dev-brightgreen)](Cargo.toml)
[![Platforms](https://img.shields.io/badge/Platforms-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](#-play-voxtera)

<br/>

<img src="site/public/images/voxtera-clean-hero.png" alt="Voxtera Hero Banner" width="100%" />

</div>

---

## 🌌 Overview

**Voxtera** is a vast, open-world voxel action-adventure RPG set in a procedurally generated fantasy universe. Inspired by iconic titles like *Cube World*, *The Legend of Zelda: Breath of the Wild*, *Dwarf Fortress*, and *Minecraft*, Voxtera blends high-performance engine engineering with rich, atmospheric exploration and player-driven creation.

- 🌍 **Procedural Voxel World**: Explore infinite biomes, sweeping mountain ranges, ancient ruins, and perilous dungeons.
- ⚔️ **Action Combat & RPG Systems**: Real-time action combat, stamina management, weapon skill trees, and magic abilities.
- ⚙️ **Custom Engine Core**: Built ground-up in **Rust** leveraging `wgpu` for high-performance cross-platform graphics, custom shader pipelines, and temporal optimizations.
- 🌐 **Multiplayer Engine**: Integrated dedicated server architecture powered by Quinn (QUIC), real-time NPC simulation (`rtsim`), and Supabase authentication.
- 🆓 **100% Free & Open Source**: Licensed under GNU GPL v3.0 — free to play, host, customize, and build upon forever.

---

## ✨ Features & Gameplay

<div align="center">

| 🌲 **Explore** | 🔨 **Build** |
| :---: | :---: |
| <img src="site/public/images/mountain-valley.jpg" alt="Explore Mountain Valley" width="450" /> | <img src="site/public/images/voxtera-build-village.png" alt="Build Voxel Village" width="450" /> |
| Traverse lush valleys, frozen peaks, and ancient ruins across endless procedural terrains. | Construct settlements, fortresses, and homes block by block with your friends. |

<br/>

| ⚔️ **Adventure** | 🎮 **In-Game Experience** |
| :---: | :---: |
| <img src="site/public/images/ruins-adventure.jpg" alt="Combat in Volcanic Ruins" width="450" /> | <img src="site/public/images/gameplay-capture.png" alt="Voxtera Gameplay Capture" width="450" /> |
| Battle monsters, uncover hidden lore, and conquer dungeon challenges. | Smooth action RPG mechanics with real-time dynamic lighting and shaders. |

</div>

---

## 🚀 Play Voxtera

### Official Web Portal & Downloads

Visit our official web portal to download the game, launcher, and view live status:

👉 **[https://voxtera.vercel.app](https://voxtera.vercel.app)**

### 📦 Game Launcher (`VoxteraLauncher`)

We recommend using the official **Voxtera Launcher** for Windows and macOS. The launcher handles automatic client updates, patch delivery, and seamless server connecting.

<div align="center">

| Step | Visual | Description |
| :---: | :---: | :--- |
| **01. Download** | <img src="site/public/images/voxtera-step-chest.png" width="64" alt="Chest" /> | Grab the latest launcher build for your platform. |
| **02. Install** | <img src="site/public/images/voxtera-step-portal.png" width="64" alt="Portal" /> | Launcher automatically configures the game files and updates. |
| **03. Play** | <img src="site/public/images/voxtera-step-sword-shield.png" width="64" alt="Sword and Shield" /> | Launch Voxtera and step into your procedural adventure. |

</div>

<br/>

| Platform | Download Link | Requirements / Notes |
| :--- | :--- | :--- |
| 🪟 **Windows** | [Download VoxteraLauncher.exe](https://voxtera.vercel.app/downloads/VoxteraLauncher.exe) | Windows 10/11 (64-bit standalone GUI) |
| 🍎 **macOS** | [Download macOS Bundle](https://voxtera.vercel.app) | Universal binary (Intel & Apple Silicon) |
| 🐧 **Linux** | Source Build | Compile via Cargo (see instructions below) |

---

## 🛠️ Workspace Architecture

Voxtera is developed as a modular Cargo workspace in Rust:

```
voxtera/
├── voxygen/       # Game client GUI & rendering engine (wgpu, shaders, input, animation)
├── server/        # Dedicated game server engine & tick loop
├── server-cli/    # CLI management & administrative interface for server hosters
├── client/        # Client-side network protocol, auth integration & entity state
├── common/        # Shared ECS components, assets pipeline, physics, and game state
├── world/         # Procedural voxel world generator, climate, biomes & dungeons
├── rtsim/         # Real-time world simulation (NPC behaviors, economic paths, travel)
├── network/       # Low-latency networking layer built on Quinn / QUIC
└── site/          # Modern showcase web portal (Vite + React deployed on Vercel)
```

---

## 💻 Building from Source

### Prerequisites

To build Voxtera locally, ensure your environment includes:

- **Rust Toolchain**: 2024 edition (`rustup update`)
- **C/C++ Compiler & CMake**: Required for native dependencies (`gcc`, `clang`, or MSVC)
- **Graphics API Support**: Vulkan, DirectX 12, or Metal compliant graphics drivers

### 1. Clone the Repository

```bash
git clone https://github.com/stoltembergg-png/voxtera.git
cd voxtera
```

### 2. Run the Game Client

```bash
cargo run --release --bin Voxtera
```

*(Note: Omit `--release` for faster compilation during rapid development, though release mode delivers optimal worldgen and rendering frame rates).*

### 3. Run a Local Dedicated Server

```bash
cargo run --release --package veloren-server-cli
```

### 4. Run the Web Portal (`site/`)

```bash
cd site
npm install
npm run dev
```

---

## ❓ FAQ & Licensing

### Is Voxtera free to play?
**Yes!** Voxtera is 100% free and open-source under the [GNU General Public License v3.0](LICENSE). You can host private servers, create custom mods, inspect the source code, and distribute modifications freely.

### What platforms are supported?
Voxtera natively supports **Windows**, **Linux**, and **macOS** (`x86_64` and `ARM64` / Apple Silicon).

---

## 🤝 Contributing & Community

We welcome contributions from developers, voxel artists, audio creators, translators, and players!

- 🦀 **Rust Core & Engine**: Explore open issues and pull requests on GitHub.
- 🎨 **Voxel Assets & Models**: Create and submit voxel artwork made with MagicaVoxel or Veloren Voxel Editor.
- 🌐 **Localization / Translations**: Translate in-game UI and dialogs in `assets/voxygen/i18n/`.

---

<div align="center">

<img src="site/public/images/voxtera-closing-valley.png" alt="Voxtera Closing Valley Banner" width="100%" />

### *Your journey begins block by block.*

**[Website](https://voxtera.vercel.app)** • **[Documentation](docs/)** • **[Contributing](CONTRIBUTING.md)** • **[License](LICENSE)**

</div>
