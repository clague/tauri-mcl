<script>
  export let page = 0;
  export let width = "75px";
  export let wide_width = "160px";

  export let sidebar_width = width;
  function sidebar_expand() {
    sidebar_width = wide_width;
  }
  function sidebar_shrink() {
    sidebar_width = width;
  } 
</script>

<div class="sidebar">
  <ul class="list-group flex-column d-inline-block first-menu" on:mouseenter={sidebar_expand} on:mouseleave={sidebar_shrink} style="width: {sidebar_width}">
    <li class="list-group-item pl-1 py-2" on:click={() => page=1}>
      <span class="ml-2 align-middle">Account</span>
    </li>

    <li class="list-group-item pl-1 py-2" on:click={() => page=2}>
      <span class="ml-2 align-middle">Download</span>
    </li>

    <li class="list-group-item pl-1 py-2" on:click={() => page=3}>
      <span class="ml-2 align-middle">Instance</span>
    </li>

    <li class="list-group-item pl-1 py-2" on:click={() => page=4}>
      <span class="ml-2 align-middle">Settings</span>
    </li>
  </ul> <!-- /.first-menu -->
</div> <!-- /.sidebar -->


<style lang="scss">
// Custom utility classes
$padding-1: 5px;
$padding-2: 15px;
//left
.pl-1 {
	padding-left: $padding-1 !important;
}

//top & bottom
.py-1 {
	padding-top: $padding-1 !important;
	padding-bottom: $padding-1 !important;
}

.py-2 {
	padding-top: $padding-2 !important;
	padding-bottom: $padding-2 !important;
}

.sidebar {
  position: fixed;
  height: 100%;
  top: 0;
  left: 0;
  z-index: 1;
}
//black menu
.first-menu {
  margin-left: 0px;
  position: fixed;
  background-color: #292a2c;
  height: 100%;
  overflow: hidden;
  transition: width .5s;
  span {
    font-size: 24px;
    color: #fff;
    font-size: 24px;
    white-space: nowrap;
    font-family: 'Sarasa Gothic SC Semibold', sans-serif;
    opacity: 0;
    visibility: hidden;
  }
  &:hover span {
    opacity: 1;
    visibility: visible;
  }
  .list-group-item {
    border-radius: 0;
    &:hover {
      background-color: #009fff;
    }
  }
}
.list-group-item {
  font-size: 14px;
}
.list-group-item {
  background-color: transparent;
  border: none;
  border-radius: 0;
}
</style>