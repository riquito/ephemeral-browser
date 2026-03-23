# Ephemeral Browser

<p align="center">
  <img src="logo.png" alt="Ephemeral browser logo" width="200" />
</p>

**Ephemeral Browser** is a lightweight tool that launches your browser in a disposable, privacy-oriented profile. Every session starts clean and leaves no trace behind.

Think of it as [Firefox Focus](https://www.mozilla.org/en-US/firefox/browsers/mobile/focus/) for the desktop - every session starts clean and leaves no trace behind.

Supports Firefox, Chromium, and Google Chrome (you can only run browsers you already installed).

## Why?

I love Firefox Focus. Did you try it? You install it on your phone, set it as default and then every link you open, it open there. When you close it, that browser's profile is gone. It gives me a lot less to worry about when I open from, say, Signal or whatsapp, a link that a friend shared with me (e.g. a link to youtube). Its cookies won't stay in my session forever, among other things. I keep a regular browser around, but the pages there are opened by me.

But there's no Firefox Focus for desktop, so I created this tool. Whether to set it as default or open it on-demand (by clicking an icon or other means) is up to you.

## How it works

Running `ephemeral-browser` will:

1. Create a temporary browser profile directory
2. Download and install [uBlock Origin](https://ublockorigin.com/) (cached locally for 7 days)
3. Apply a set of privacy-focused preferences (see below)
4. Launch the browser using the temporary profile
5. Delete the profile automatically when the window is closed

You can also pass URLs as arguments to open specific pages:

```sh
ephemeral-browser https://example.com
```

## Privacy defaults

The generated profile ships with these preferences out of the box:

- **DuckDuckGo** as the default search engine
- **uBlock Origin** pre-installed and enabled (full version on Firefox, Lite/MV3 on Chromium/Chrome)
- **Telemetry, health reports, and data collection** disabled
- **Shield studies and feature recommendations** disabled
- **Bookmarks toolbar** hidden (unless configured)
- **First-run tabs and privacy notices** suppressed
- **Dark theme** (with a purplish colour, so you know it's not your normal browser)

## Configuration

Settings are read from a `config.toml` file, searched in this order:

1. Next to the executable
2. Current working directory
3. OS config directory: `~/.config/ephemeral-browser/` (Linux), `~/Library/Application Support/ephemeral-browser/` (macOS), `%AppData%\ephemeral-browser\` (Windows)

All fields are optional — sensible defaults are used when the file is missing or incomplete.

```toml
homepage = "https://duckduckgo.com"
search_engine = "DuckDuckGo"
theme = "dark"    # "dark", "light", or "default"
browser = "firefox"  # "firefox", "chromium", or "chrome"

[toolbar]
enabled = true

[[toolbar.tabs]]
label = "YouTube"
url = "https://www.youtube.com"

[[toolbar.tabs]]
label = "AliExpress"
url = "https://www.aliexpress.com"
```

| Key | Default | Description |
|-----|---------|-------------|
| `homepage` | `https://duckduckgo.com` | Start page. Set to `""` for a blank page. |
| `search_engine` | `DuckDuckGo` | Default search engine (see [Known limitations](#known-limitations)). |
| `theme` | `dark` | Browser theme (`dark`, `light`, or `default`). |
| `browser` | `firefox` | Browser to use (`firefox`, `chromium`, or `chrome`). |
| `browser_path` | — | Custom path to the browser binary (e.g. a self-built Chromium). |
| `toolbar.enabled` | `false` | Show the bookmarks toolbar. |
| `toolbar.tabs` | — | List of toolbar bookmarks, each with `label` and `url`. |

A `config.toml.example` is included as a starting point.

## Requirements

- Firefox, Chromium, or Google Chrome (at least one)

## Building

```sh
cargo build --release
```

The binary will be at `target/release/ephemeral-browser`.

## Development

Set up the git hooks after cloning:

```sh
git config core.hooksPath hooks/
```

This enables a pre-commit hook that checks formatting (`cargo fmt`), build, and linting (`cargo clippy`).

## Desktop integration (GNOME)

To add Ephemeral Browser as a desktop application, create a `.desktop` file:

```sh
vim ~/.local/share/applications/ephemeral-browser.desktop
update-desktop-database ~/.local/share/applications/
```

## Known limitations

- **Default search engine cannot be changed via profile configuration.** Firefox does not allow setting the default search engine through `user.js` preferences. The only supported mechanism is [enterprise policies](https://mozilla.github.io/policy-templates/#searchengines), which must be placed in the Firefox installation directory and require write access to it. The `search_engine` config option does not allow to add new search engines and it doesn't control the search engine you get in the search bar.
- **uBlock Origin Lite on Chromium/Chrome.** Modern Chromium/Chrome no longer supports Manifest V2 extensions, so the full uBlock Origin cannot be used. uBlock Origin Lite (MV3) is installed instead, which has reduced filtering capabilities compared to the full version.
