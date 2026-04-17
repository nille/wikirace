# 🌐 wikirace

> ⚠️ Work in progress

A Wikipedia degrees-of-separation game for the terminal. Navigate from one random article to another by following links — in as few steps as possible.

Built with [Ratatui](https://ratatui.rs/) and the Wikipedia API.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)

## How to play

```
cargo run --release
```

You're given a random start article and a random target. Browse the links on each page to navigate toward the target. The game tracks your steps and time.

## Controls

| Key | Action |
|-----|--------|
| `↑` `↓` | Navigate links |
| `Enter` | Follow selected link |
| Type | Fuzzy filter links |
| `Esc` | Clear filter |
| `Tab` | Toggle hints |
| `Ctrl+G` | Give up |
| `Ctrl+C` | Quit |
| `r` | Play again (end screen) |

## Features

- **Fuzzy search** — type to filter links with fuzzy matching, best matches first
- **Hints** — toggle with `Tab` to highlight links that connect to the target (via Wikipedia backlinks). Hint links float to the top of the list
- **Optimal path** — after winning or giving up, a BFS search shows the shortest path (up to 4 hops)
- **Article previews** — see extracts for both the current article and the target so you know what you're looking at and aiming for

## Requirements

- Rust 1.75+ (or use [mise](https://mise.jdx.dev/): `mise install`)
- Internet connection (fetches from Wikipedia API)
