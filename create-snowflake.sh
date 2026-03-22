#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/config.toml"

# Read a value from the config file. Usage: config_get <key> [default]
config_get() {
    local key="$1"
    local default="$2"
    if [ -f "$CONFIG_FILE" ]; then
        local value
        value=$(sed -n "s/^${key}[[:space:]]*=[[:space:]]*\"\(.*\)\"/\1/p" "$CONFIG_FILE")
        if [ -n "$value" ] || grep -q "^${key}[[:space:]]*=" "$CONFIG_FILE"; then
            echo "$value"
            return
        fi
    fi
    echo "$default"
}

# Read configuration
HOMEPAGE=$(config_get "homepage" "https://duckduckgo.com")
SEARCH_ENGINE=$(config_get "search_engine" "DuckDuckGo")
THEME=$(config_get "theme" "dark")

# Determine homepage startup mode: 1 = homepage, 0 = blank
if [ -n "$HOMEPAGE" ]; then
    STARTUP_PAGE=1
else
    STARTUP_PAGE=0
fi

# Create a temporary profile directory
PROFILE_DIR=$(mktemp -d)

# Ensure cleanup happens on exit or interrupt
trap "rm -rf '$PROFILE_DIR'; echo 'Temporary profile deleted'" EXIT INT TERM

echo "Setting up new temporary profile at:"
echo "$PROFILE_DIR"

# Create the profile structure
mkdir -p "$PROFILE_DIR/extensions"

# Download uBlock Origin XPI (only if missing or older than 1 week)
UBLOCK_XPI_FILE="/tmp/ublock-origin.xpi"
UBLOCK_URL="https://addons.mozilla.org/firefox/downloads/latest/ublock-origin/latest.xpi"

# Check if file exists and is less than 7 days old
if [ ! -f "$UBLOCK_XPI_FILE" ] || [ $(find "$UBLOCK_XPI_FILE" -mtime +7 2>/dev/null | wc -l) -gt 0 ]; then
    echo "Downloading uBlock Origin..."
    curl -sL "$UBLOCK_URL" -o "$UBLOCK_XPI_FILE"
fi

# Copy to profile extensions directory
cp "$UBLOCK_XPI_FILE" "$PROFILE_DIR/extensions/uBlock0@raymondhill.net.xpi"

# Determine theme ID
case "$THEME" in
    dark)  THEME_ID="firefox-compact-dark@mozilla.org" ;;
    light) THEME_ID="firefox-compact-light@mozilla.org" ;;
    *)     THEME_ID="default-theme@mozilla.org" ;;
esac

# Create user.js for preferences
cat > "$PROFILE_DIR/user.js" << USERJS
// Set default search engine
user_pref("browser.search.defaultenginename", "${SEARCH_ENGINE}");
user_pref("browser.search.selectedEngine", "${SEARCH_ENGINE}");
user_pref("browser.newtabpage.activity-stream.improvesearch.handoffToAwesomebar", false);
user_pref("browser.urlbar.placeholderName", "${SEARCH_ENGINE}");
user_pref("browser.urlbar.update2.engineAliasRefresh", true);

// Use theme
user_pref("extensions.activeThemeID", "${THEME_ID}");

// Auto-enable extensions without prompts
user_pref("extensions.autoDisableScopes", 0);
user_pref("extensions.enabledScopes", 15);

// Allow uBlock Origin in private windows
user_pref("extensions.webextensions.uBlock0@raymondhill.net.privateBrowsingAllowed", true);

// Disable privacy notice tab on first run
user_pref("datareporting.policy.firstRunURL", "");
user_pref("browser.startup.homepage_override.mstone", "ignore");

// Disable surveys
user_pref("app.shield.optoutstudies.enabled", false);
user_pref("browser.newtabpage.activity-stream.asrouter.userprefs.cfr.addons", false);
user_pref("browser.newtabpage.activity-stream.asrouter.userprefs.cfr.features", false);

// Disable telemetry and data collection
user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("datareporting.policy.dataSubmissionEnabled", false);
user_pref("toolkit.telemetry.enabled", false);
user_pref("toolkit.telemetry.unified", false);
user_pref("toolkit.telemetry.archive.enabled", false);

// Hide bookmarks toolbar
user_pref("browser.toolbars.bookmarks.visibility", "never");

// Set homepage
user_pref("browser.startup.homepage", "${HOMEPAGE}");
user_pref("browser.startup.page", ${STARTUP_PAGE});
USERJS

echo "Starting firefox"

# Launch Firefox, optionally opening a URL passed as argument
firefox -no-remote -profile "$PROFILE_DIR" "$@"
