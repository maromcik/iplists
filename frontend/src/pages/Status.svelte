<script lang="ts">
    import { onMount } from "svelte";
    import { apiFetchJson, ApiRequestError } from "../js/api";
    import { parseAppStatus } from "../js/status";
    import ErrorAlert from "../component/ErrorAlert.svelte";
    import StatusBadge from "../component/StatusBadge.svelte";
    import {
        AppErrorKind,
        StatusCode,
        type ApiError,
        type AppStatus,
        type ComponentStatus,
    } from "../js/types";

    let status: AppStatus | null = null;
    let error: ApiError | null = null;
    let loading = true;

    const BANNER_COLORS: Record<StatusCode, string> = {
        [StatusCode.Ok]: "border-green-500 bg-green-50 dark:bg-green-900/20",
        [StatusCode.Warning]: "border-yellow-400 bg-yellow-50 dark:bg-yellow-900/20",
        [StatusCode.Error]: "border-red-500 bg-red-50 dark:bg-red-900/20",
        [StatusCode.Disaster]: "border-red-600 bg-red-50 dark:bg-red-900/20",
    };

    let components: { title: string; value: ComponentStatus }[] = [];
    $: if (status) {
        components = [
            { title: "Locations", value: status.locations },
            { title: "ASNs", value: status.asns },
            { title: "Geo", value: status.geo },
            { title: "Blocklist", value: status.blocklist },
        ];
    }

    async function load() {
        loading = true;
        error = null;
        try {
            status = parseAppStatus(await apiFetchJson<unknown>("/api/status"));
        } catch (err) {
            status = null;
            if (err instanceof ApiRequestError) {
                error = err.response;
            } else {
                error = {
                    code: 0,
                    kind: AppErrorKind.Unknown,
                    description: err instanceof Error ? err.message : String(err),
                };
            }
        } finally {
            loading = false;
        }
    }

    onMount(load);

    function formatDate(value: string | null): string {
        if (value === null) return "—";
        const date = new Date(value);
        return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
    }
</script>

<div class="w-full max-w-6xl mx-auto p-4">
    <div class="flex items-center justify-between mb-6">
        <h3 class="text-3xl font-bold text-gray-900 dark:text-white">Status</h3>
        <button
            on:click={load}
            class="inline-flex items-center gap-2 px-4 py-2 rounded-md bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm transition-colors"
        >
            <i class="fas fa-rotate-right"></i>
            Refresh
        </button>
    </div>

    {#if loading}
        <p class="text-gray-600 dark:text-gray-400">Loading status…</p>
    {:else if error}
        <ErrorAlert {error} title="Could not load status" />
    {:else if status}
        <div
            class="border-l-4 {BANNER_COLORS[status.overall.status_code]} rounded-xl shadow-sm p-5 mb-4 flex items-center gap-4"
        >
            <StatusBadge status={status.overall} />
            <div class="min-w-0">
                <h4 class="text-lg font-semibold text-gray-900 dark:text-white">
                    Overall
                </h4>
                <p class="text-sm text-gray-700 dark:text-gray-300 break-words">
                    {status.overall.message}
                </p>
            </div>
        </div>


        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            {#each components as component (component.title)}
                <div class="bg-cards rounded-xl shadow-sm p-5 text-gray-900 dark:text-gray-100">
                    <h4 class="text-xl font-bold mb-3">{component.title}</h4>

                    <div class="mb-4">
                        <p
                            class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400 mb-1"
                        >
                            Component
                        </p>
                        <StatusBadge status={component.value.status} />
                        <p
                            class="text-sm mt-2 text-gray-700 dark:text-gray-300 break-words"
                        >
                            {component.value.status.message}
                        </p>
                    </div>

                    <div>
                        <p
                            class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400 mb-1"
                        >
                            Update
                        </p>

                        <dl
                            class="mt-2 text-sm text-gray-600 dark:text-gray-400 space-y-1"
                        >
                            <div class="flex gap-2">
                                <dt class="font-medium w-24">Last update:</dt>
                                <dd>
                                    {formatDate(component.value.update.last_update)}
                                </dd>
                            </div>
                            <div class="flex gap-2">
                                <dt class="font-medium w-24">Next update:</dt>
                                <dd>{formatDate(component.value.update.next_update)}</dd>
                            </div>
                        </dl>
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</div>
