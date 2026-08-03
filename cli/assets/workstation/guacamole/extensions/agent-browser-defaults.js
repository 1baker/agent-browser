/*
 * Applies agent-browser's Guacamole input default once per browser origin.
 * The migration marker allows later user-selected input methods to persist.
 */
(function applyAgentBrowserGuacamoleDefaults() {
    'use strict';

    var preferencesKey = 'GUAC_PREFERENCES';
    var migrationKey = 'AGENT_BROWSER_GUAC_DEFAULTS_VERSION';
    var migrationVersion = '1';

    try {
        var storage = window.localStorage;
        if (!storage || storage.getItem(migrationKey) === migrationVersion)
            return;

        var storedPreferences = storage.getItem(preferencesKey);
        var preferences = storedPreferences ? JSON.parse(storedPreferences) : {};
        if (!preferences || typeof preferences !== 'object' || Array.isArray(preferences))
            preferences = {};

        preferences.inputMethod = 'text';
        storage.setItem(preferencesKey, JSON.stringify(preferences));
        storage.setItem(migrationKey, migrationVersion);
    }
    catch (ignore) {
        // Guacamole already tolerates unavailable browser-local storage.
    }
}());
