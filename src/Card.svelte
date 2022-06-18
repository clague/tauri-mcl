<script>
	import { Card, Stack, Text } from "@kahi-ui/framework";
</script>

<Stack
	class="card-sizing"
	orientation="horizontal"
	spacing="medium"
	alignment_y="top"
	variation="wrap"
>
	<Card.Container sizing="nano">
		<Card.Header />
		<Card.Section>
			<Text />
		</Card.Section>
	</Card.Container>
</Stack>

<style type="text/scss">
	:global(.card-sizing > .card) {
		width: 170px;
		height: 90px;
		margin: 10px;
	}
</style>
