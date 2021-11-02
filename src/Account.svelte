<script>
  import { Button, Tile , Text, Spinner, Stack, Wave} from "@kahi-ui/framework";
  import { invoke } from '@tauri-apps/api/tauri';

  export let logged;
  let logged_index = 0;
  export let logging;
  let logging_index = 0;

  function login() {
    logging_index += 1;
    let item = { index: logging_index, err_result: "" };
    logging.push(item);
    logging = logging;

    invoke("login", { index: item.index })
      .then(res => {
        logged_index += 1;
        logged.push({index: logged_index, name: res[0], uuid: res[1]});
        logging.splice(logging.indexOf(item), 1);

        logged = logged;
        logging = logging;
      })
      .catch((err) => {
        item.err_result = err;
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
<Stack 
  class="accouts-stack"
  orientation="vertical"
  spacing="medium"
>
  {#each logged as item}
  <Tile.Container class="account-view" palette="auto" width="">
    <Tile.Figure shape="rounded">
      <img src="https://crafatar.com/avatars/{item.uuid}" alt="account avatar"/>
    </Tile.Figure>

    <Tile.Section>
      <Tile.Header>{item.name}</Tile.Header>

      <Text is="small">

      </Text>
    </Tile.Section>

    <Tile.Footer>
      <Button palette="accent" on:click={() => {
        logged.splice(logged.indexOf(item), 1);
        logged = logged;
      }}>Delete</Button>
      <Button palette="accent">Refresh</Button>
    </Tile.Footer>
  </Tile.Container>
  {/each}

  {#each logging as item}
  <Tile.Container class="account-view" palette="auto" width="">
    <Tile.Figure shape="pill">
      <Spinner />
    </Tile.Figure>

    <Tile.Section>
      {#if item.err_result == ""}
      <Tile.Header>Logging...</Tile.Header>
      {:else}
      <Tile.Header>Error!</Tile.Header>
      <Text is="small">
        {item.err_result}
      </Text>
      {/if}
    </Tile.Section>

    <Tile.Footer>
      <Button palette="accent" on:click={() => {
        login_abort(item.index);
        logging.splice(logging.indexOf(item), 1);
        logging = logging;
      }}>Delete</Button>
    </Tile.Footer>
  </Tile.Container>
  {/each}
  <Button palette="affirmative" on:click={login}>
    Login
  </Button>
</Stack>

<style>
  :global(.accouts-stack) {
    margin: auto;
    width: 70%;
  }
</style>