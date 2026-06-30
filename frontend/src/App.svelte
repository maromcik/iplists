<script lang="ts">
	import { user } from "./js/store";
	import NavBar from "./component/Navbar.svelte";
	import Countries from "./pages/Countries.svelte";
	import Continents from "./pages/Continents.svelte";
	import LocationDetail from "./pages/LocationDetail.svelte";

	let menu: number;

	type MenuItem = { label: string, id: number };

	const get_menu_items = (): MenuItem[] => {
			return [
				{ label: "Countries", id: 7 },
				{ label: "Continents", id: 8 },
				{ label: "Location", id: 6 },
			];
	};

	$: menu_items = get_menu_items();
</script>

<!-- MENNU BAR ON TOP -->
<NavBar navItems={menu_items} bind:menu />

<main class="bg-gray-50 dark:bg-gray-900 min-h-screen p-6 flex flex-col items-center">
{#if menu === 6}
	<LocationDetail />
{:else if menu === 7}
	<Countries changeMenu={(id) => menu = id} />
{:else if menu === 8}
	<Continents changeMenu={(id) => menu = id} />
{:else}
	<h2 class="text-2xl font-bold">Page Not Found or Completed Yet</h2>
{/if}
</main>

