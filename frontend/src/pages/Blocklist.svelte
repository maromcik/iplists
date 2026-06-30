<script lang="ts">
    let ips = "";
    
    const params = new URLSearchParams(window.location.search);
    let version = params.get('version') || ""; // "" means all

    let copyButtonText = "Copy";

    async function fetchIps() {
        // Construct API URL
        let apiUrl = `/iplist/blocklist`;
        if (version) {
            apiUrl += `?version=${encodeURIComponent(version)}`;
        }
        
        const response = await fetch(apiUrl);
        ips = await response.text();

        // Update URL
        const newParams = new URLSearchParams();
        if (version) newParams.set('version', version);
        
        let newUrl = `/blocklist`;
        if (newParams.toString()) {
            newUrl += `?${newParams.toString()}`;
        }

        if (window.location.pathname + window.location.search !== newUrl) {
            window.history.replaceState(null, '', newUrl);
        }
    }

    function copyToClipboard() {
        navigator.clipboard.writeText(ips).then(() => {
            copyButtonText = "Copied!";
            setTimeout(() => {
                copyButtonText = "Copy";
            }, 2000);
        });
    }

    // Reactively fetch when version changes
    $: version, fetchIps();
</script>

<div class="w-full max-w-4xl mx-auto p-4">
    <h3 class="text-3xl font-bold mb-6 text-gray-900 dark:text-white">Blocklist</h3>

    <div class="mb-6">
        <label for="version-select" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">IP Version:</label>
        <select id="version-select" bind:value={version} class="bg-gray-100 dark:bg-gray-700 p-3 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 transition-all">
            <option value="">All</option>
            <option value="ipv4">IPv4</option>
            <option value="ipv6">IPv6</option>
        </select>
    </div>

    <div class="relative">
        <button on:click={copyToClipboard} class="absolute top-2 right-2 bg-gray-700 hover:bg-gray-600 text-white text-xs px-2 py-1 rounded transition-colors">
            {copyButtonText}
        </button>
        <pre class="bg-gray-900 dark:bg-black text-amber-500 p-6 rounded-xl shadow-inner overflow-auto text-sm font-mono leading-relaxed">{ips}</pre>
    </div>
</div>
