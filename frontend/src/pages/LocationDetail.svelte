<script lang="ts">
    import { onMount } from "svelte";
    import { activeLocation, locationType } from "../js/store";

    let ips = "";
    let format = "Json";
    let copyButtonText = "Copy";
    let copiedUrl = "";
    let formatId = "format-select";

    async function fetchIps() {
        if (!$activeLocation || !$locationType) return;
        const loc = $activeLocation;
        let locationValue: string;
        if ('alpha2' in loc) {
            locationValue = loc.alpha2;
        } else {
            locationValue = loc.region;
        }
        const response = await fetch(`/iplist/location?${$locationType}=${encodeURIComponent(locationValue)}&format=${format.toLowerCase()}`);
        
        let text = await response.text();
        
        if (format === 'Json') {
            try {
                const parsed = JSON.parse(text);
                const finalParsed = (typeof parsed === 'string') ? JSON.parse(parsed) : parsed;
                ips = JSON.stringify(finalParsed, null, 2);
            } catch (e) {
                ips = text;
            }
        } else {
            ips = text;
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

    function copyUrl(url: string) {
        navigator.clipboard.writeText(url).then(() => {
            copiedUrl = url;
            setTimeout(() => {
                copiedUrl = "";
            }, 2000);
        });
    }

    // Reactively fetch when format or location changes
    $: format, $activeLocation, fetchIps();

    $: locationValue = ($activeLocation && $locationType)
        ? ('alpha2' in $activeLocation ? $activeLocation.alpha2 : $activeLocation.region)
        : "";
</script>

<div class="w-full max-w-4xl mx-auto p-4">
    <h3 class="text-3xl font-bold mb-6 text-gray-900 dark:text-white">IPs for <span class="text-amber-600">{locationValue}</span> ({$locationType})</h3>

    <div class="mb-6 p-4 bg-gray-100 dark:bg-gray-800 rounded-xl">
        <h4 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Static Lists:</h4>
        <div class="space-y-2">
            {#each [`txt`, `nft`] as ext}
                {@const url = `http://localhost:8000/static/lists/gen/${locationValue}.${ext}`}
                <div class="bg-gray-900 text-amber-500 p-3 rounded-lg font-mono text-sm break-all flex justify-between items-center gap-2">
                    <a href={url} target="_blank" class="hover:underline flex-grow">
                        {url}
                    </a>
                    <button 
                        on:click={() => copyUrl(url)} 
                        class="text-white bg-gray-700 hover:bg-gray-600 px-2 py-1 rounded text-xs whitespace-nowrap transition-colors"
                    >
                        {copiedUrl === url ? "Copied!" : "Copy"}
                    </button>
                </div>
            {/each}
        </div>
    </div>

    <div class="mb-6">
        <label for={formatId} class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Output Format:</label>
        <select id={formatId} bind:value={format} class="bg-gray-100 dark:bg-gray-700 p-3 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 transition-all">
            <option value="Json">JSON</option>
            <option value="Text">Text</option>
            <option value="Nftables">Nftables</option>
        </select>
    </div>

    <div class="relative">
        <button on:click={copyToClipboard} class="absolute top-2 right-2 bg-gray-700 hover:bg-gray-600 text-white text-xs px-2 py-1 rounded transition-colors">
            {copyButtonText}
        </button>
        <pre class="bg-gray-900 dark:bg-black text-amber-500 p-6 rounded-xl shadow-inner overflow-auto text-sm font-mono leading-relaxed">{ips.length > 50000 ? ips.substring(0, 50000) + '\n\n... (Content truncated, please download the full file above)' : ips}</pre>
    </div>
</div>
