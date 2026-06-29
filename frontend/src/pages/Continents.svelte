<script>
    import { onMount } from "svelte";
    import { activeLocation, locationType } from "../js/store.js";
    
    export let changeMenu;

    let continents = [];
    onMount(async () => {
        const response = await fetch("/iplist/continent");
        continents = await response.json();
    });

    function selectContinent(continent) {
        activeLocation.set(continent);
        locationType.set("continent");
        changeMenu(6); // 6 is the ID for LocationDetail
    }
</script>

<div class="w-full max-w-4xl mx-auto p-4">
    <h3 class="text-3xl font-bold mb-6 text-gray-900 dark:text-white">Continents</h3>
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
        {#each continents as continent}
            <button 
                class="bg-cards p-6 rounded-xl shadow-sm text-gray-900 dark:text-gray-100 font-medium hover:shadow-md hover:bg-gray-300 dark:hover:bg-gray-600 transition-all duration-200"
                on:click={() => selectContinent(continent)}>
                {continent.region}
            </button>
        {/each}
    </div>
</div>
