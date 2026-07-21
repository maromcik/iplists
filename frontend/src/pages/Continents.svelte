<script lang="ts">
    import { onMount } from "svelte";
    import { navigate } from 'svelte-routing';
    import { apiFetchJson, ApiRequestError } from "../js/api";
    import ErrorAlert from "../component/ErrorAlert.svelte";
    import { AppErrorKind, type ApiError } from "../js/types";

    let continents: string[] = [];
    let error: ApiError | null = null;
    let loading = true;

    onMount(async () => {
        try {
            continents = await apiFetchJson<string[]>("/api/iplist/continent");
        } catch (err) {
            error = err instanceof ApiRequestError
                ? err.response
                : { code: 0, kind: AppErrorKind.Unknown, description: String(err) };
        } finally {
            loading = false;
        }
    });

    function selectContinent(continent: string) {
        navigate(`/location?continent=${encodeURIComponent(continent)}&format=json`);
    }
</script>

<div class="w-full max-w-4xl mx-auto p-4">
    <h3 class="text-3xl font-bold mb-6 text-gray-900 dark:text-white">Continents</h3>

    {#if loading}
        <p class="text-gray-600 dark:text-gray-400">Loading continents…</p>
    {:else if error}
        <ErrorAlert error={error} title="Could not load continents" />
    {:else if continents.length === 0}
        <p class="text-gray-600 dark:text-gray-400">No continents available.</p>
    {:else}
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
            {#each continents as continent}
                <button 
                    class="bg-cards p-6 rounded-xl shadow-sm text-gray-900 dark:text-gray-100 font-medium hover:shadow-md hover:bg-gray-300 dark:hover:bg-gray-600 transition-all duration-200"
                    on:click={() => selectContinent(continent)}>
                    <i class="fas fa-globe mr-2"></i>{continent}
                </button>
            {/each}
        </div>
    {/if}
</div>
