<script lang="ts">
    import { apiFetchText, ApiRequestError } from "../js/api";
    import ErrorAlert from "../component/ErrorAlert.svelte";
    import { AppErrorKind, type ApiError } from "../js/types";

    let ips = "";
    let error: ApiError | null = null;
    let loading = false;

    const params = new URLSearchParams(window.location.search);
    let version = params.get("version") || "";
    let format = params.get("format") || "Json";

    let copyButtonText = "Copy";
    let copiedUrl = "";

    // Guards against an older request overwriting a newer one.
    let requestId = 0;

    async function fetchIps() {
        const id = ++requestId;
        loading = true;
        error = null;

        const search = new URLSearchParams();
        search.set("format", format);
        if (version) search.set("version", version);

        try {
            const text = await apiFetchText(`/api/blocklist?${search.toString()}`);
            if (id !== requestId) return;

            if (format === "Json") {
                try {
                    ips = JSON.stringify(JSON.parse(text), null, 2);
                } catch {
                    ips = text;
                }
            } else {
                ips = text;
            }
        } catch (err) {
            if (id !== requestId) return;
            ips = "";
            error = err instanceof ApiRequestError
                ? err.response
                : { code: 0, kind: AppErrorKind.Unknown, description: String(err) };
        } finally {
            if (id === requestId) loading = false;
        }

        const newUrl = `/blocklist?${search.toString()}`;
        if (window.location.pathname + window.location.search !== newUrl) {
            window.history.replaceState(null, "", newUrl);
        }
    }

    function copyToClipboard(text: string) {
        navigator.clipboard.writeText(text).then(() => {
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

    $: version, format, fetchIps();
    $: apiUrl = `${window.location.origin}/api/blocklist?format=${format}${version ? `&version=${version}` : ""}`;
</script>

<div class="w-full max-w-4xl mx-auto p-4">
    <h3 class="text-3xl font-bold mb-6 text-gray-900 dark:text-white">Blocklist</h3>

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

            <div class="mb-6 flex flex-wrap gap-6">
                <div>
                    <label for="version-select" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">IP Version:</label>
                    <select id="version-select" bind:value={version} class="bg-gray-100 dark:bg-gray-700 p-3 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 transition-all">
                        <option value="">All</option>
                        <option value="ipv4">IPv4</option>
                        <option value="ipv6">IPv6</option>
                    </select>
                </div>

                <div>
                    <label for="format-select" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Output Format:</label>
                    <select id="format-select" bind:value={format} class="bg-gray-100 dark:bg-gray-700 p-3 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-amber-500 focus:border-amber-500 transition-all">
                        <option value="Json">JSON</option>
                        <option value="Text">Text</option>
                        <option value="Nftables">Nftables</option>
                        <option value="NftablesNamedSets">Nftables Named Sets</option>
                    </select>
                </div>
            </div>

            {#if error}
                <ErrorAlert error={error} title="Could not load blocklist" />
            {:else}
                <div class="relative">
                    <button on:click={() => copyToClipboard(ips)} class="absolute top-2 right-2 bg-gray-700 hover:bg-gray-600 text-white text-xs px-2 py-1 rounded transition-colors">
                        {copyButtonText}
                    </button>
                    {#if loading}
                        <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">Loading blocklist…</p>
                    {/if}
                    <pre class="bg-gray-900 dark:bg-black text-amber-500 p-6 rounded-xl shadow-inner overflow-auto text-sm font-mono leading-relaxed">{ips}</pre>
                </div>
            {/if}
        </div>
    </div>
</div>
