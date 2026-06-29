<script>
    import { onMount } from "svelte";
    import { activeLocation, locationType } from "../js/store.js";
    
    // We need to trigger a menu change or page change in App.svelte
    // Let's assume the caller passes a function to change the menu
    export let changeMenu;

    let countries = [];
    onMount(async () => {
        const response = await fetch("/iplist/country");
        countries = await response.json();
    });

    function selectCountry(country) {
        activeLocation.set(country);
        locationType.set("country");
        changeMenu(6); // 6 is the ID for LocationDetail
    }
</script>

<h3>Countries</h3>
<ul>
    {#each countries as country}
        <li>
            <button on:click={() => selectCountry(country)}>{country}</button>
        </li>
    {/each}
</ul>
