<script lang="ts">
  import { onMount } from "svelte";
  import { link } from 'svelte-routing';

  type NavItem = { label: string, path: string };
  export let navItems: NavItem[] = [];

  // Show mobile icon and display menu
  let showMobileMenu = false;

  // Mobile menu click event handler
  const handleMobileIconClick = () => (showMobileMenu = !showMobileMenu);

  // Media match query handler
  const mediaQueryHandler = (e: MediaQueryListEvent) => {
    // Reset mobile state
    if (!e.matches) {
      showMobileMenu = false;
    }
  };

  // Menu selection
  const handleMenuSelection = () => {
    showMobileMenu = false;
  };

  function handleKeyDown(e: KeyboardEvent) {
    if (e.keyCode === 13) {
      handleMobileIconClick();
    }
  }

  // Attach media query listener on mount hook
  onMount(() => {
    const mediaListener = window.matchMedia("(max-width: 767px)");
    mediaListener.addEventListener("change", mediaQueryHandler);
  });
</script>

<nav class="bg-muni-blue h-[45px]">
  <div class="max-w-[980px] mx-auto px-5 flex items-center h-full">
    <!-- MUNI Logo -->
    <a href="/" class="mr-4 flex-shrink-0">
      <img src="/static/img/muni.png" alt="MUNI" class="h-16 object-contain" />
    </a>

    <!-- Mobile icon -->
    <div
      role="button"
      tabindex=0
      on:keydown={handleKeyDown}
      on:click={handleMobileIconClick}
      class={`relative w-[25px] h-[14px] cursor-pointer md:hidden ${showMobileMenu ? "active" : ""}`}
    >
      <div class={`absolute w-full h-[2px] bg-white transition-all origin-center ${showMobileMenu ? "top-[6px] rotate-[-45deg]" : "top-0"}`} />
      <div class={`absolute w-full h-[2px] bg-white transition-all origin-center ${showMobileMenu ? "opacity-0" : "top-[6px]"}`} />
      <div class={`absolute w-full h-[2px] bg-white transition-all origin-center ${showMobileMenu ? "bottom-[6px] rotate-[45deg]" : "bottom-0"}`} />
    </div>

    <!-- Navbar List -->
    <ul class={`absolute md:static md:flex md:w-auto md:bg-transparent ${showMobileMenu ? "block bg-muni-blue top-[45px] left-0 w-full p-4" : "hidden"}`}>
      {#each navItems as item}
        <li class="relative border-b border-gray-600 md:border-none">
          <a
            use:link
            href={item.path}
            on:click={handleMenuSelection}
            class="text-sat-xl flex items-center px-4 hover:text-amber-500 transition-colors"
          >{item.label}</a>
        </li>
      {/each}
    </ul>
  </div>
</nav>
