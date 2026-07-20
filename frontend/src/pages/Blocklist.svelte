<script lang="ts">
    import { onMount } from "svelte";
    import { apiFetch } from "../js/api";

    let ips = "";

    const params = new URLSearchParams(window.location.search);
    let version = params.get('version') || "";

    let copyButtonText = "Copy";
    let copiedUrl = "";

    async function fetchIps() {
        let apiUrl = '/api/blocklist';
        if (version) {
            apiUrl += `?version=${version}`;
        }
        
        const response = await apiFetch(apiUrl);
        ips = await response.text();

        const newParams = new URLSearchParams();
        if (version) newParams.set('version', version);
        const newUrl = newParams.toString() ? `/blocklist?${newParams.toString()}` : `/blocklist`;
        if (window.location.pathname + window.location.search !== newUrl) {
            window.history.replaceState(null, '', newUrl);
        }
    }

    function copyToClipboard(ips: string) {
        navigator.clipboard.writeText(ips).then(() => {
            copyButtonText = "Copied!";
            setTimeout(() => { copyButtonText = "Copy"; }, 2000);
        });
    }

    function copyUrl(url: string) {
        navigator.clipboard.writeText(url).then(() => {
            copiedUrl = url;
            setTimeout(() => { copiedUrl = ""; }, 2000);
        });
    }

    $: version, fetchIps();
    $: apiUrl = version ? `${window.location.origin}/api/blocklist?version=${version}` : `${window.location.origin}/api/blocklist`;
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

    <div class="space-y-6">
        <div>
            <h4 class="text-xl font-semibold mb-2 text-gray-700 dark:text-gray-300">
                {version === 'ipv4' ? 'IPv4 Blocklist' : version === 'ipv6' ? 'IPv6 Blocklist' : 'Blocklist'}
            </h4>
            <div class="mb-2 p-4 bg-gray-100 dark:bg-gray-800 rounded-xl">
                <h4 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">API Query:</h4>
                <div class="bg-gray-900 text-amber-500 p-3 rounded-lg font-mono text-sm break-all flex justify-between items-center gap-2">
                    <a href={apiUrl} target="_blank" class="hover:underline flex-grow">
                        {apiUrl}
                    </a>
                    <button 
                        on:click={() => copyUrl(apiUrl)} 
                        class="text-white bg-gray-700 hover:bg-gray-600 px-2 py-1 rounded text-xs whitespace-nowrap transition-colors"
                    >
                        {copiedUrl === apiUrl ? "Copied!" : "Copy"}
                    </button>
                </div>
            </div>
            <div class="relative">
                <button on:click={() => copyToClipboard(ips)} class="absolute top-2 right-2 bg-gray-700 hover:bg-gray-600 text-white text-xs px-2 py-1 rounded transition-colors">
                    {copyButtonText}
                </button>
                <pre class="bg-gray-900 dark:bg-black text-amber-500 p-6 rounded-xl shadow-inner overflow-auto text-sm font-mono leading-relaxed">{ips}</pre>
            </div>
        </div>
    </div>
</div>
