<script>
	import { Heading } from "@kahi-ui/framework";
	import "@kahi-ui/framework/dist/kahi-ui.framework.min.css";
	import Account from "./Account.svelte";
	import Tilebar from "./Titlebar.svelte";
	import Add from "./Add.svelte";
	import Card from "./Card.svelte";

	let instances = {};
</script>

<Tilebar />
<div class="content">
	<div class="add">
		<Add class="add" />
	</div>
	<div class="account">
		<Account class="account" />
	</div>
	<div class="instances">
		{#if JSON.stringify(instances) == "{}"}
			<Heading is="h2" align="center" id="nothing-tip">
				Nothing here, try adding an instance?
				<img
					id="add-instance"
					src="assets/img/plus-circle.svg"
					alt="add instance"
				/>
			</Heading>
		{/if}
	</div>
	<div class="border">
		<Card />
		<Card />
		<Card />
		<Card />
		<Card />
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
	:global(#nothing-tip) {
		position: absolute;
		width: 100%;
		top: 30%;
	}
	#add-instance {
		margin: 10px auto 0 auto;
		width: 30px;
	}
	.add {
		position: absolute;
		bottom: 15px;
		left: 10px;
	}
	.border {
		position: absolute;
		width: 950px;
		height: 50%;
		top: 50%;
		left: 25.260417%;
		display: flex;
		flex-direction: row;
		flex-wrap: wrap;
		align-content: flex-start;
		justify-content: flex-start;
		align-items: center;
	}
</style>
