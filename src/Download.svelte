<script>
  import {Button, Progress} from "@kahi-ui/framework";
  let progress = 0;
</script>

<Button
palette="affirmative"
>
下载
</Button>
<Progress />

<style>
  :global(.progress-stack) {
    margin: auto;
    width: 70%;
    height: 80%;
  }
</style>