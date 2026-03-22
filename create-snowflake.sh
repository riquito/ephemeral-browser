#!/bin/bash

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

# Create user.js for preferences
cat > "$PROFILE_DIR/user.js" << 'EOF'
// Set default search engine to DuckDuckGo (or others)
user_pref("browser.search.defaultenginename", "DuckDuckGo");
user_pref("browser.search.selectedEngine", "DuckDuckGo");
user_pref("browser.newtabpage.activity-stream.improvesearch.handoffToAwesomebar", false);
user_pref("browser.urlbar.placeholderName", "DuckDuckGo");
user_pref("browser.urlbar.update2.engineAliasRefresh", true);

// Use dark theme
user_pref("extensions.activeThemeID", "firefox-compact-dark@mozilla.org");

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
EOF

echo "Starting firefox"

# Launch Firefox
firefox -no-remote -profile "$PROFILE_DIR" # -private-window
