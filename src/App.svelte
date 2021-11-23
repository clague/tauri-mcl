<script>
    import "@kahi-ui/framework/dist/kahi-ui.framework.min.css";
    import Account from "./Account.svelte";
    import Tilebar from "./Titlebar.svelte";

</script>
<Tilebar/>
<div class="content">
  <div class="account">
    <Account/>
  </div>
</div>

<style type="text/scss">
  .content {
    margin-top: 30px;
  }
  .account {
    position: absolute;
    bottom: 15px;
    right: 10px;
  }
</style>
