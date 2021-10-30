<script>
    import { once, listen } from '@tauri-apps/api/event';
    import { Button, Progress } from "@kahi-ui/framework";
    import { invoke } from '@tauri-apps/api/tauri';

    let login_promise = invoke('login', { index: 1 });
    let button_enabled = true;

    let progress = 0.5;

    function stop()  {
        button_enabled = false;
        invoke('login_abort', { index: 1 })
            .then((res) => console.log(`Message: ${res}`))
            .catch((e) => console.error(e));
    };

    const unlisten = listen('download_progress', event => {
        download_progress = event.payload.message;
    })
</script>

<div data-component="navbar">
<div data-component="sidebar">
  <div class="sidebar">
  <ul class="list-group flex-column d-inline-block first-menu">
    <li class="list-group-item pl-3 py-2">
      <a href="#"><i class="fa fa-user-o" aria-hidden="true"><span class="ml-2 align-middle">Reporting</span></i></a>
    <li class="list-group-item pl-3 py-2">
      <a href="#"><i class="fa fa-user-o" aria-hidden="true"><span class="ml-2 align-middle">Content</span></i></a>
    </li> <!-- /.list-group-item -->
    <li class="list-group-item pl-3 py-2">
      <a href="#">
        <i class="fa fa-user-o" aria-hidden="true"><span class="ml-2 align-middle">Engagement</span></i>
      </a>
    </li> <!-- /.list-group-item -->
    <li class="list-group-item pl-3 py-2">
      <a href="#"><i class="fa fa-user-o" aria-hidden="true"><span class="ml-2 align-middle">Image Center</span></i></a>
    </li>
    <li class="list-group-item pl-3 py-2">
      <a href="#"><i class="fa fa-user-o" aria-hidden="true"><span class="ml-2 align-middle">Settings</span></i></a>
    </li>
    <li class="list-group-item pl-3 py-2">
      <a href="#"><i class="fa fa-user-o" aria-hidden="true"><span class="ml-2 align-middle">Support</span></i></a>
    </li>
  </ul> <!-- /.first-menu -->
  </div> <!-- /.sidebar -->
</div>
</div>

{#await login_promise}
    <div class='loader'></div>
{:then result} 
    <h1 class='complete'> 完成:{result} </h1>
{:catch error}
    <h1 class='fail'> 发生错误：{error} </h1>
{/await}

<Button
    palette="affirmative"
    on:click={stop}
    disabled={!button_enabled}
>
    停止
</Button>

<Progress {progress} />

<Button
    palette="negative"
    on:click={() =>
        (progress = Math.max(0, progress - 0.05))}
>
    -0.05
</Button>

<Button
    palette="affirmative"
    on:click={() =>
        (progress = Math.min(1, progress + 0.05))}
>
    +0.05
</Button>

<style type="text/scss">
   @import "./loading.scss";
</style>
