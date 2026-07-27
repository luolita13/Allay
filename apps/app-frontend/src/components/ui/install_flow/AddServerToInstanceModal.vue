<script setup>
import { CheckIcon, PlusIcon, SearchIcon } from '@modrinth/assets'
import {
	Admonition,
	Avatar,
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	NewModal as Modal,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { useQueryClient } from '@tanstack/vue-query'
import { computed, ref } from 'vue'

import { trackEvent } from '@/helpers/analytics'
import { list } from '@/helpers/instance'
import { getInstanceIconSrc } from '@/helpers/instance-icon'
import { add_server_to_instance, get_instance_worlds } from '@/helpers/worlds.ts'

const { handleError } = injectNotificationManager()
const queryClient = useQueryClient()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: { id: 'app.add-server-modal.header', defaultMessage: 'Add server to instance' },
	warning: {
		id: 'app.add-server-modal.warning',
		defaultMessage: 'This server may not be compatible with all instances.',
	},
	searchPlaceholder: { id: 'app.add-server-modal.search-placeholder', defaultMessage: 'Search for an instance' },
	adding: { id: 'app.add-server-modal.adding', defaultMessage: 'Adding...' },
	added: { id: 'app.add-server-modal.added', defaultMessage: 'Added' },
	add: { id: 'app.add-server-modal.add', defaultMessage: 'Add' },
	cancel: { id: 'app.add-server-modal.cancel', defaultMessage: 'Cancel' },
})

const modal = ref()
const searchFilter = ref('')
const instances = ref([])

const serverName = ref('')
const serverAddress = ref('')

const shownInstances = computed(() =>
	instances.value.filter((instance) => {
		return instance.name.toLowerCase().includes(searchFilter.value.toLowerCase())
	}),
)

defineExpose({
	show: async (name, address) => {
		serverName.value = name
		serverAddress.value = address
		searchFilter.value = ''

		const instanceValues = await list().catch(handleError)
		await Promise.allSettled(
			instanceValues.map(async (instance) => {
				instance.adding = false
				instance.added = false

				try {
					const worlds = await get_instance_worlds(instance.id)
					instance.added = worlds.some(
						(w) => w.type === 'server' && w.address === serverAddress.value,
					)
				} catch {
					// Ignore - will show as not added
				}
			}),
		)

		instances.value = instanceValues
		modal.value.show()

		trackEvent('AddServerToInstanceStart', { source: 'AddServerToInstanceModal' })
	},
})

async function addServer(instance) {
	instance.adding = true
	try {
		await add_server_to_instance(instance.id, serverName.value, serverAddress.value, 'prompt')
		instance.added = true
		await queryClient.invalidateQueries({ queryKey: ['worlds', instance.id] })

		trackEvent('AddServerToInstance', {
			server_name: serverName.value,
			instance_name: instance.name,
			source: 'AddServerToInstanceModal',
		})
	} catch (err) {
		handleError(err)
	}
	instance.adding = false
}
</script>

<template>
	<Modal ref="modal" :header="formatMessage(messages.header)">
		<div class="flex flex-col gap-4 min-w-[350px]">
			<Admonition type="warning" :body="formatMessage(messages.warning)" />
			<StyledInput
				v-model="searchFilter"
				:icon="SearchIcon"
				type="search"
				:placeholder="formatMessage(messages.searchPlaceholder)"
				autocomplete="off"
			/>
			<div class="max-h-[21rem] overflow-y-auto">
				<div
					v-for="instance in shownInstances"
					:key="instance.id"
					class="flex w-full items-center justify-between gap-2 bg-bg-raised text-icon shadow-none"
				>
					<ButtonStyled type="transparent">
						<router-link
							:to="`/instance/${encodeURIComponent(instance.id)}`"
							@click="modal.hide()"
						>
							<Avatar
								:src="getInstanceIconSrc(instance)"
								class="mr-2 [--size:2rem]"
							/>
							{{ instance.name }}
						</router-link>
					</ButtonStyled>
					<ButtonStyled>
						<button :disabled="instance.added || instance.adding" @click="addServer(instance)">
							<PlusIcon v-if="!instance.added && !instance.adding" />
							<CheckIcon v-else-if="instance.added" />
							{{
								instance.adding
									? formatMessage(messages.adding)
									: instance.added
										? formatMessage(messages.added)
										: formatMessage(messages.add)
							}}
						</button>
					</ButtonStyled>
				</div>
			</div>
			<div class="input-group push-right">
				<ButtonStyled>
					<button @click="modal.hide()">{{ formatMessage(messages.cancel) }}</button>
				</ButtonStyled>
			</div>
		</div>
	</Modal>
</template>
