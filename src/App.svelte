<script>
    import "@kahi-ui/framework/dist/kahi-ui.framework.min.css";
    import Sidebar from "./Sidebar.svelte";
    import Account from "./Account.svelte";
    import Download from "./Download.svelte";

    let logging = [];
    let logged = [];
    let sidebar_width;
    let current_page = 0;
</script>

<Sidebar width=75px wide_width=160px bind:sidebar_width bind:page={current_page}/>

<div class="content" style="margin-left: {sidebar_width}">
  {#if current_page == 1}
  <Account logging={logging} logged={logged}/>
  {:else if current_page == 2}
  <Download/>
  {/if}
</div>

<style type="text/scss">
  .content {
    transition: margin-left .5s;
  }
</style>
