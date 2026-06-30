<script lang="ts">
    import { onMount } from "svelte";
    import { navigate } from 'svelte-routing';
    import type { Location } from "../js/types";
    
    let countries: Location[] = [];
    onMount(async () => {
        const response = await fetch("/iplist/country");
        countries = await response.json();
    });

    function selectCountry(country: Location) {
        navigate(`/location?country=${country.alpha2}&format=json`);
    }
</script>

<div class="w-full max-w-6xl mx-auto p-4">
    <h3 class="text-3xl font-bold mb-6 text-gray-900 dark:text-white">Countries</h3>
    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
        {#each countries as country}
            <button 
                class="bg-cards text-gray-900 dark:text-gray-100 p-4 rounded-xl shadow-sm hover:shadow-md hover:bg-gray-300 dark:hover:bg-gray-600 transition-all duration-200"
                on:click={() => selectCountry(country)}>
                <i class="fas fa-flag mr-2"></i>{country.name} ({country.alpha2})
            </button>
        {/each}
    </div>
</div>
