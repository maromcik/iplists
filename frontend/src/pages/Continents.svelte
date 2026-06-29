<script>
    import { onMount } from "svelte";
    let continents = [];
    onMount(async () => {
        const response = await fetch("/iplist/continent");
        continents = await response.json();
    });
</script>

<h3>Continents</h3>
<ul>
    {#each continents as continent}
        <li>{continent}</li>
    {/each}
</ul>
