<script>
    import { onMount } from "svelte";
    import { activeLocation, locationType } from "../js/store.js";

    let ips = "";
    let format = "Json";

    async function fetchIps() {
        if (!$activeLocation || !$locationType) return;
        const response = await fetch(`/iplist/location?${$locationType}=${$activeLocation}&format=${format.toLowerCase()}`);
        ips = await response.text();
    }

    // Reactively fetch when format or location changes
    $: format, $activeLocation, fetchIps();
</script>

<h3>IPs for {$activeLocation} ({$locationType})</h3>
<select bind:value={format}>
    <option value="Json">JSON</option>
    <option value="Text">Text</option>
    <option value="Nftables">Nftables</option>
</select>
<pre>{ips}</pre>
