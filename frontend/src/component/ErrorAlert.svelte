<script lang="ts">
    import { AppErrorKind, type ApiError } from "../js/types";

    export let error: ApiError | null = null;
    export let title = "Something went wrong";

    $: code = error?.code ?? 500;
    $: kind = error?.kind ?? AppErrorKind.Unknown;
    $: description = error?.description ?? "Unknown error";

    function reload() {
        window.location.reload();
    }
</script>

{#if error}
    <div class="w-full max-w-4xl mx-auto p-4">
        <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl p-6 shadow-sm">
            <div class="flex items-start gap-4">
                <div class="text-red-600 dark:text-red-400 text-3xl mt-0.5">
                    <i class="fas fa-circle-exclamation"></i>
                </div>
                <div class="flex-1 min-w-0">
                    <h3 class="text-lg font-semibold text-red-800 dark:text-red-300 mb-1">
                        {title}
                    </h3>
                    <p class="text-sm font-mono text-red-700 dark:text-red-400 mb-2">
                        {code} {kind}
                    </p>
                    <p class="text-red-700 dark:text-red-300 break-words">
                        {description}
                    </p>
                    <div class="mt-4 flex gap-3">
                        <button
                            on:click={reload}
                            class="inline-flex items-center gap-2 px-4 py-2 rounded-md bg-red-600 hover:bg-red-700 text-white text-sm transition-colors"
                        >
                            <i class="fas fa-rotate-right"></i>
                            Retry
                        </button>
                        <a
                            href="/"
                            class="inline-flex items-center gap-2 px-4 py-2 rounded-md bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm transition-colors"
                        >
                            <i class="fas fa-home"></i>
                            Home
                        </a>
                    </div>
                </div>
            </div>
        </div>
    </div>
{/if}
