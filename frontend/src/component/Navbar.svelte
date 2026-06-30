<script lang="ts">
  import { link } from 'svelte-routing';

  type NavItem = { label: string, path: string };
  export let navItems: NavItem[] = [];

  function toggleMenu() {
    const menu = document.getElementById('phone-menu');
    if (menu) {
      menu.classList.toggle('hidden');
    }
  }

  // Close menu when clicking outside
  function closeMenu(event: MouseEvent) {
    const menu = document.getElementById('phone-menu');
    const hamburger = document.getElementById('hamburger');
    if (menu && hamburger && !menu.classList.contains('hidden') && 
        !menu.contains(event.target as Node) && !hamburger.contains(event.target as Node)) {
      menu.classList.add('hidden');
    }
  }
</script>

<svelte:body on:click={closeMenu} />

<div id="navbar"
     class="bg-muni-blue p-2 sm:p-4 flex shadow-md fixed w-full z-10 top-0 h-16 sm:h-20 justify-between items-center">

    <a id="navbar-image" href="/" class="flex items-center">
        <h2 class="font-extrabold text-white text-lg sm:text-xl xl:text-2xl mt-5 mr-5">
            IP Lists
        </h2>
        <div class="border-l-2 border border-gray-300 h-6"></div>
        <img src="/static/img/muni.png"
             alt="Logo"
             class="h-10 sm:h-12 lg:h-14 ml-4 object-contain"
        >
    </a>

    <!-- Navigation Links Laptop View -->
    <div id="laptop-menu" class="hidden lg:flex sm:ml-2 sm:mr-2 md:mr-4 md:ml-4 lg:items-center sm:space-x-3 md:space-x-6 lg:space-x-10 xl:space-x-16 xl:text-xl">
        {#each navItems as item}
        <a href={item.path}
           use:link
           class="flex items-center text-white hover:text-amber-500 hover:scale-110 cursor-pointer transition-transform">
            {item.label}
        </a>
        {/each}
    </div>

    <!-- Hamburger Menu Button -->
    <div class="flex lg:hidden items-center">
        <button
                id="hamburger"
                on:click={toggleMenu}
                class="focus:outline-none"
                aria-label="Toggle menu"
                aria-expanded="false"
        >
            <div class="w-8 h-0.5 md:w-10 lg:w-12 md:h-1 m-2 bg-white"></div>
            <div class="w-8 h-0.5 md:w-10 lg:w-12 md:h-1 m-2 bg-white"></div>
            <div class="w-8 h-0.5 md:w-10 lg:w-12 md:h-1 m-2 bg-white"></div>
        </button>
    </div>
</div>

<!-- Navigation Links Mobile View -->
<div id="phone-menu" class="fixed top-16 sm:top-20 left-0 w-full bg-muni-blue text-white text-2xl pt-4 sm:pt-8 lg:pt-10 pb-6 lg:hidden hidden z-20">
    <div class="flex flex-col space-y-6 items-center">
        {#each navItems as item}
        <a href={item.path}
           use:link
           on:click={toggleMenu}
           class="flex items-center hover:text-amber-500 hover:scale-110 transition-transform">
            {item.label}
        </a>
        {/each}
    </div>
</div>
