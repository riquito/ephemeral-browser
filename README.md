# Ephemeral Browser

<p align="center">
  <img src="logo.png" alt="Ephemeral browser logo" width="200" />
</p>

**Ephemeral Browser** is a lightweight script that launches your browser in a disposable, privacy-oriented profile. Every session starts clean and leaves no trace behind.

Think of it as [Firefox Focus](https://www.mozilla.org/en-US/firefox/browsers/mobile/focus/) for the desktop - every session starts clean and leaves no trace behind.

Currently supports Firefox, with plans to add Chrome support.

> This project is not affiliated with or endorsed by Mozilla or Google.

## How it works

Running `create-ephemeral-browser.sh` will:

1. Create a temporary browser profile directory
2. Download and install [uBlock Origin](https://ublockorigin.com/) (cached locally for 7 days)
3. Apply a set of privacy-focused preferences (see below)
4. Launch the browser using the temporary profile
5. Delete the profile automatically when the window is closed

## Privacy defaults

The generated profile ships with these preferences out of the box:

- **DuckDuckGo** as the default search engine
- **uBlock Origin** pre-installed and enabled (including in private windows)
- **Dark compact theme** enabled
- **Telemetry, health reports, and data collection** disabled
- **Shield studies and feature recommendations** disabled
- **Bookmarks toolbar** hidden
- **First-run tabs and privacy notices** suppressed

## Requirements

- Firefox (or Chrome, when supported)
- `curl` (for downloading uBlock Origin)
- A POSIX-compatible shell (`bash`)

## Usage

```sh
./create-ephemeral-browser.sh
```

## Desktop integration (GNOME)

To add Ephemeral Browser as a desktop application, create a `.desktop` file:

```sh
vim ~/.local/share/applications/ephemeral-browser.desktop
update-desktop-database ~/.local/share/applications/
```

## License

This project is provided as-is for personal use.
