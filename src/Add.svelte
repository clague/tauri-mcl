<script>
	import { Stack, Button } from "@kahi-ui/framework";
</script>

<Stack orientation="horizontal" spacing="medium" variation="wrap">
	<Button variation="outline" palette="accent" id="stack">
		➕
	</Button>
</Stack>

<style type="text/scss">
	:global(#stack) {
		max-width: 250px;
		--button-padding-x: 12px;
		max-height: 40px;
	}
</style>
