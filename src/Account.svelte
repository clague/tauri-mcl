<script>
  import { Popover, ContextButton, Menu, Box, Spinner} from "@kahi-ui/framework";
  import { invoke } from '@tauri-apps/api/tauri';

  let logged = {};
  let logging = {};
  let logging_index = 0;
  let active_uuid = "";

  invoke('get_logged')
    .then((res) => logged = res)
    .catch((e) => console.error(e))
  
  invoke('get_logging')
    .then((res) => logging = res)
    .catch((e) => console.error(e))

  invoke('get_active')
    .then((res) => active_uuid = res)
    .catch((e) => console.error(e))

  function login() {
    logging_index += 1;
    let item = { 
      index: logging_index,
      err_message: ""
    };
    logging[logging_index.toString()] = item;
    logging = logging; // refresh page, maybe

    invoke("login", { index: item.index })
      .then(res => {
        logged[res.uuid] = res;
        if (active_uuid == "") {
          active_uuid = res.uuid;
        }
        delete logging[item.index];

        logged = logged;
        logging = logging;
      })
      .catch((err) => {
        item.err_message = err;
        logging = logging;
      })
  }

  function login_abort(index) {
    invoke('login_abort', { index: index })
      .then((res) => {
        console.log(`Message: ${res}`)
      })
      .catch((e) => {
        console.error(e)
      });
  };
</script>

<Popover
  logic_id="popover-account"
  placement="top"
  alignment_x="left"
  spacing="medium"
  dismissible
  hidden
>
  <ContextButton id="popover-trigger" palette="accent" variation="outline">
    {#if active_uuid != ""}
      <img src="https://crafatar.com/avatars/{active_uuid}" alt="Account avatar">
      {logged[active_uuid].name}
    {:else}
      You haven't logged in yet
    {/if}
  </ContextButton>
  <Box palette="auto" elevation="medium" padding="small" shape="rounded">
    <Menu.Container id="popover" palette="auto">
      <Menu.Divider>
        Account Manager
      </Menu.Divider>
      <!-- {#if JSON.stringify(logged) == "{}" && JSON.stringify(logging) == "{}"}
      <Menu.Label on:click={login}>
        Press to log in!
      </Menu.Label>
      {/if} -->

      {#each Object.keys(logged) as uuid}
      <Menu.Label>
        {logged[uuid].name}
      </Menu.Label>
      {/each}

      <Menu.Label on:click={login}>
        Log in
      </Menu.Label>
    </Menu.Container>
  </Box>
      
</Popover>

<style type="text/scss">
  @use "sass:math";

  $width: 250px;
  $height: 40px;

  :global(#popover-trigger) {
    max-width: $width;
    --button-padding-x: 12px;
    max-height: $height;
  }
  :global(#popover-trigger img) {
    max-height: math.div($height, 2.0);
  }
  :global(#popover) {
    width: $width;
  }
  :global(.account-tile) {
    width: $width - 20px;
  }
</style>
