<script lang="ts">
    import { onMount } from "svelte";
    import { apiFetchJson, ApiRequestError } from "../js/api";
    import ErrorAlert from "../component/ErrorAlert.svelte";
    import { AppErrorKind, type ApiError, type CombinedIpRange } from "../js/types";

    let result: CombinedIpRange | null = null;
    let error: ApiError | null = null;
    let loading = false;

    const params = new URLSearchParams(window.location.search);
    let ip = params.get('ip') || "";
    let ipInput = ip;

    let copiedUrl = "";

    async function lookup() {
        if (!ip) return;

        loading = true;
        error = null;
        result = null;

        const apiUrl = `/api/iplist/geo?ip=${encodeURIComponent(ip)}`;

        try {
            result = await apiFetchJson<CombinedIpRange>(apiUrl);
        } catch (err) {
            error = err instanceof ApiRequestError
                ? err.response
                : { code: 0, kind: AppErrorKind.Unknown, description: String(err) };
        } finally {
            loading = false;
        }

        const newUrl = `/lookup?ip=${encodeURIComponent(ip)}`;
        if (window.location.pathname + window.location.search !== newUrl) {
            window.history.replaceState(null, '', newUrl);
        }
    }

    function searchIp() {
        ip = ipInput.trim();
        lookup();
    }

    function copyUrl(url: string) {
        navigator.clipboard.writeText(url).then(() => {
            copiedUrl = url;
            setTimeout(() => { copiedUrl = ""; }, 2000);
        });
    }

    onMount(() => {
        if (ip) {
            lookup();
        }
    });
</script>

<div class="w-full max-w-4xl mx-auto p-4">
    <h3 class="text-3xl font-bold mb-6 text-gray-900 dark:text-white">
        <i class="fas fa-magnifying-glass mr-2"></i>API Lookup
    </h3>

    <form class="mb-6 flex flex-col sm:flex-row gap-4" on:submit|preventDefault={searchIp}>
        <input
            type="text"
            bind:value={ipInput}
            placeholder="Enter IP address (e.g. 2001:718:801:406::99)"
            class="flex-grow min-w-0 p-3 rounded-lg border border-gray-300 dark:border-gray-600 bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-amber-500"
        />
        <button
            type="submit"
            disabled={loading || !ipInput.trim()}
            class="bg-amber-600 hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed text-white px-6 py-3 rounded-lg font-bold transition-colors flex items-center gap-2"
        >
            <i class="fas fa-magnifying-glass"></i>
            Search
        </button>
    </form>

    {#if loading}
        <p class="text-gray-600 dark:text-gray-400">Looking up IP…</p>
    {:else if error}
        <ErrorAlert error={error} title="Lookup failed" />
    {:else if result}
        {@const url = `${window.location.origin}/api/iplist/geo?ip=${encodeURIComponent(result.ip)}`}
        <div class="mb-6 p-4 bg-gray-100 dark:bg-gray-800 rounded-xl">
            <h4 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">API Query:</h4>
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
        </div>

        <div class="p-4 bg-gray-100 dark:bg-gray-800 rounded-xl shadow-inner">
            <dl class="divide-y divide-gray-300 dark:divide-gray-700">
                <div class="py-3 grid grid-cols-1 sm:grid-cols-3 gap-1 sm:gap-4">
                    <dt class="text-sm font-medium text-gray-500 dark:text-gray-400">IP Address</dt>
                    <dd class="text-sm font-mono text-gray-900 dark:text-gray-100 sm:col-span-2">{result.ip}</dd>
                </div>
                <div class="py-3 grid grid-cols-1 sm:grid-cols-3 gap-1 sm:gap-4">
                    <dt class="text-sm font-medium text-gray-500 dark:text-gray-400">Network</dt>
                    <dd class="text-sm font-mono text-gray-900 dark:text-gray-100 sm:col-span-2">{result.network}</dd>
                </div>
                <div class="py-3 grid grid-cols-1 sm:grid-cols-3 gap-1 sm:gap-4">
                    <dt class="text-sm font-medium text-gray-500 dark:text-gray-400">ASN</dt>
                    <dd class="text-sm font-mono text-gray-900 dark:text-gray-100 sm:col-span-2">{result.asn}</dd>
                </div>
                <div class="py-3 grid grid-cols-1 sm:grid-cols-3 gap-1 sm:gap-4">
                    <dt class="text-sm font-medium text-gray-500 dark:text-gray-400">ISP</dt>
                    <dd class="text-sm text-gray-900 dark:text-gray-100 sm:col-span-2">{result.isp || "—"}</dd>
                </div>
                <div class="py-3 grid grid-cols-1 sm:grid-cols-3 gap-1 sm:gap-4">
                    <dt class="text-sm font-medium text-gray-500 dark:text-gray-400">Country</dt>
                    <dd class="text-sm text-gray-900 dark:text-gray-100 sm:col-span-2">{result.location.name} ({result.location.code})</dd>
                </div>
                <div class="py-3 grid grid-cols-1 sm:grid-cols-3 gap-1 sm:gap-4">
                    <dt class="text-sm font-medium text-gray-500 dark:text-gray-400">Continent</dt>
                    <dd class="text-sm text-gray-900 dark:text-gray-100 sm:col-span-2">{result.location.continent}</dd>
                </div>
            </dl>
        </div>
    {/if}
</div>
