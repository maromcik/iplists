<script>
    import { onMount } from "svelte";
    import { activeLocation, locationType } from "../js/store.js";

    let ips = "";
    let format = "Json";
    let copyButtonText = "Copy";

    async function fetchIps() {
        if (!$activeLocation || !$locationType) return;
        const locationValue = $locationType === 'country' ? $activeLocation.alpha2 : $activeLocation.region;
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

    // Reactively fetch when format or location changes
    $: format, $activeLocation, fetchIps();
</script>

<div class="w-full max-w-4xl mx-auto p-4">
    <h3 class="text-3xl font-bold mb-6 text-gray-900 dark:text-white">IPs for <span class="text-amber-600">{$locationType === 'country' ? $activeLocation.alpha2 : $activeLocation.region}</span> ({$locationType})</h3>

    <div class="mb-6">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Output Format:</label>
        <select bind:value={format} class="bg-gray-100 dark:bg-gray-700 p-3 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 transition-all">
            <option value="Json">JSON</option>
            <option value="Text">Text</option>
            <option value="Nftables">Nftables</option>
        </select>
    </div>

    <div class="relative">
        <button on:click={copyToClipboard} class="absolute top-2 right-2 bg-gray-700 hover:bg-gray-600 text-white text-xs px-2 py-1 rounded transition-colors">
            {copyButtonText}
        </button>
        <pre class="bg-gray-900 dark:bg-black text-amber-500 p-6 rounded-xl shadow-inner overflow-auto text-sm font-mono leading-relaxed">{ips}</pre>
    </div>
</div>
