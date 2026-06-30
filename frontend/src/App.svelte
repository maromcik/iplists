<script lang="ts">
	import { Router, Route } from "svelte-routing";
	import NavBar from "./component/Navbar.svelte";
	import Countries from "./pages/Countries.svelte";
	import Continents from "./pages/Continents.svelte";
	import AsnDetail from "./pages/AsnDetail.svelte";
	import LocationDetail from "./pages/LocationDetail.svelte";

	const navItems = [
		{ label: "Countries", 	path: "/countries" },
		{ label: "Continents", 	path: "/continents" },
		{ label: "ASNs", 		path: "/asn" },
	];
</script>

<NavBar {navItems} />

<main class="bg-gray-50 dark:bg-gray-900 min-h-screen p-6 mt-12 flex flex-col items-center">
	<Router>
		<Route path="/countries" component={Countries} />
		<Route path="/continents" component={Continents} />
		<Route path="/asn" component={AsnDetail} />
		<Route path="/location" component={LocationDetail} />
		<Route path="/" component={Countries} />
	</Router>
</main>

