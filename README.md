# 🦀 ratcrate-tui: Ratatui Ecosystem Crate Discoverer

[![CI](https://github.com/ratcrate/ratcrate-tui/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ratcrate/ratcrate-tui/actions/workflows/ci.yml)
[![Release](https://github.com/ratcrate/ratcrate-tui/actions/workflows/all-releases.yml/badge.svg)](https://github.com/ratcrate/ratcrate-tui/actions/workflows/all-releases.yml)
[![Documentation](https://docs.rs/ratcrate-tui/badge.svg)](https://docs.rs/ratcrate-tui)
[![Security Audit](https://github.com/ratcrate/ratcrate-tui/actions/workflows/security-audit.yml/badge.svg)](https://github.com/ratcrate/ratcrate-tui/actions/workflows/security-audit.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE) 
[![Crates.io](https://img.shields.io/crates/v/ratcrate-tui.svg)](https://crates.io/crates/ratcrate-tui)
[![Downloads](https://img.shields.io/crates/d/ratcrate-tui.svg)](https://crates.io/crates/ratcrate-tui)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Contributors](https://img.shields.io/github/contributors/ratcrate/ratcrate-tui)](https://github.com/ratcrate/ratcrate-tui/graphs/contributors)



**ratcrate-tui** is a fast, terminal-based user interface (TUI) for exploring crates within the Ratatui ecosystem. Find core libraries, popular community packages, view stats, and get install commands—all without leaving your terminal.

It is built with Rust, leveraging the power of `ratatui` for the interface and `crossterm` for terminal interaction.

This is the 3rd tool in the Ratatui ecosystem. 

Try out

 - Web version [Ratcrate](https://ratcrate.in) 
 - CLI version  [ratcrate-cli](https://github.com/ratcrate/ratcrate-cli)

# ✨ Demo

![ratcrate-tui](https://github.com/user-attachments/assets/42708536-847d-483c-9305-f6f9a19facf6)



# ✨ Features

* **Fast Crate Listing:** Browse all known Ratatui ecosystem crates.
* **Detailed Views:** See descriptions, version, downloads, links, and categories for any selected crate.
* **Intelligent Caching:** Caches data locally to ensure near-instantaneous load times after the first run.
* **Powerful Filtering & Search:**
    * Filter by **core libraries** vs. community packages.
    * Search by **name** or **description**.
    * Pre-defined lists: **Top** (by downloads), **Recent** (by weekly downloads), and **Newest** crates.
* **TUI-First Design:** Intuitive, Vim-like navigation (`j`/`k`/`g`/`G`/`Ctrl+d`/`Ctrl+u`).
* **Statistics View:** See aggregate stats on total downloads, core/community distribution, and the top 5 crates.

# ⬇️ Installation

## Prerequisites
You need to have the latest stable Rust toolchain installed. You can install it using `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh
```


## From Crates.io (Recommended)

Once the project is published to Crates.io, you can install it directly with cargo:

```bash
$> cargo install ratcrates
```

## Homebrew

```
$> brew install rvbug/tap/ratcrate-tui
```

## From Source
Clone the repository and build it yourself:

```bash
$> git clone [https://github.com/rvbug/ratcrates.git](https://github.com/rvbug/ratcrates.git)
$> cd ratcrates
$> cargo install --path .
```

## 🚀 Usage
Simply run the command in your terminal:

```bash
$> ratcrate-tui
```

## ⌨️ Controls & Commands
The TUI operates in two main modes: Normal (Navigation) and Command (Input).

## ⌨️ Controls

Normal Mode (Default)

| Keybind | Action|
| --- | --- | 
| `j` / `↓` | Move selection down | 
| `k` / `↑`| Move selection up | 
|`Ctrl+d` | Page down (jump 10 lines)| 
|`Ctrl+u` | Page up (jump 10 lines)| 
|`g` |Go to the top of the list | 
|`G` |Go to the bottom of the list | 
|`TAB` |Toggle **Statistics** view (`View::Stats`) | 
| `?`| Toggle **Help** view (`View::Help`)| 
| `:`| Enter **Command** mode | 
|`/` | Enter **Command** mode with a pre-typed `:search` prefix| 
|`q` | Quit the application| 


## ⌨️ Commands

| command | description | example |
| --- | --- | --- | 
| `:q` `:quit`| Quit the application | `:quit` |
| `:all`| Show all available crates (resets filters). | `:all`|
| `:core`| Show all available crates (resets filters).|`:core` |
| `:top [N]`| Show the top N crates by total downloads. (Default: 10)|`:top 5` |
| `:recent [N]`| Show the top N crates by weekly (recent) downloads.|`:recent 20` |
| `:new [N]`| Show the N newest crates (by creation date). | `:new 20`|
| `:search <query>`|Search crate names and descriptions for a query. |`:search terminal` |
| `/<query>`|Shortcut for search (automatically prepends `:search`). | `/player` |



# Future Plans
- [ ] Icon & beautification
- [ ] Add Banner via `qbanner` library


<br>

<h1 align="center">Support</h1>

<h3 align="center">
  If you find my projects helpful or want to support my research and open-source work:
</h3>
<p align="center">
  <a href="https://ko-fi.com/rvbugged">
    <img src="https://ko-fi.com/img/githubbutton_sm.svg" alt="Ko-fi" height="40">
  </a>
</p>



 


