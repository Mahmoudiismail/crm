// API requests
let registeredAppsCache = null;

async function fetchApps() {
    try {
        const response = await fetch("/api/apps/list");
        if (response.ok) {
            registeredAppsCache = await response.json();
            return registeredAppsCache;
        }
    } catch (e) {
        console.error("Failed to fetch apps", e);
    }
    return [];
}

async function fetchAppManifest(appId) {
    try {
        const response = await fetch("/api/apps/manifest?app_id=" + encodeURIComponent(appId));
        if (response.ok) {
            return await response.json();
        }
    } catch (e) {
        console.error("Failed to fetch app manifest", e);
    }
    return null;
}

window.api = {
    fetchApps,
    fetchAppManifest,
    getRegisteredAppsCache: () => registeredAppsCache,
    setRegisteredAppsCache: (cache) => { registeredAppsCache = cache; }
};
